//! Local stdio adapter. The SDK owns framing, lifecycle and cancellation delivery.
mod schema;
use crate::*;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext};
use serde_json::{Value, json};
use std::{borrow::Cow, collections::BTreeMap, sync::Arc};
use tokio::sync::{Mutex, Semaphore};

pub(crate) const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone)]
struct Server {
    api: SpecApi,
    defaults: ContextSelector,
    admitted: Arc<Semaphore>,
    management: Arc<Mutex<()>>,
    projects: Arc<Mutex<BTreeMap<String, Arc<Mutex<()>>>>>,
}
fn error(code: &str, message: impl Into<String>) -> SpecError {
    SpecError::new(code, message)
}
fn failed(e: SpecError) -> CallToolResult {
    CallToolResult::structured_error(
        json!({"error":{"code":e.code,"message":e.message,"details":e.details}}),
    )
}
fn response(value: SpecResponse) -> Value {
    json!({"context":value.context.as_ref().map(crate::storage::context_value),"data":value.data})
}
fn bounded(result: &CallToolResult, id: &RequestId) -> Result<(), SpecError> {
    let envelope = json!({"jsonrpc":"2.0","id":id,"result":result});
    if serde_json::to_vec(&envelope)
        .map_err(crate::documents::io)?
        .len()
        + 1
        > MAX_BYTES
    {
        Err(error("size_limit", "serialized MCP response exceeds 8 MiB"))
    } else {
        Ok(())
    }
}
impl Server {
    fn execution_key(&self, selector: &ContextSelector) -> Result<String, SpecError> {
        let registry = crate::registry::load(&self.api.runtime)?;
        match crate::storage::selected(&self.api, &registry, selector) {
            Ok(p) => Ok(format!("project:{}", p.id)),
            Err(e) if e.code == "project_not_found" => {
                let path = selector.checkout.as_ref().unwrap_or(&self.api.runtime.cwd);
                let path = std::fs::canonicalize(self.api.runtime.cwd.join(path))
                    .map_err(crate::documents::io)?;
                let clean = crate::git::context_runtime(&self.api.runtime);
                let cx = crate::api::Execution::new(&clean);
                let path = crate::context::checkout_top(&cx, &path)?.unwrap_or(path);
                Ok(format!("checkout:{}", crate::registry::path_value(&path)))
            }
            Err(e) => Err(e),
        }
    }
    async fn invoke(
        &self,
        name: &str,
        arguments: Value,
        context: RequestContext<RoleServer>,
    ) -> CallToolResult {
        if context.ct.is_cancelled() {
            return failed(error("cancelled", "operation cancelled"));
        }
        let permit = match self.admitted.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                return failed(error(
                    "busy",
                    "32 application operations are already admitted",
                ));
            }
        };
        let (request, selector, management) =
            match schema::request(name, &arguments, &self.defaults) {
                Ok(r) => r,
                Err(e) => return failed(e),
            };
        let management_guard = if management {
            tokio::select! {
                _ = context.ct.cancelled() => return failed(error("cancelled", "operation cancelled")),
                g = self.management.clone().lock_owned() => Some(g),
            }
        } else {
            None
        };
        if context.ct.is_cancelled() {
            return failed(error("cancelled", "operation cancelled"));
        }
        let key = match request.as_ref() {
            Some(
                SpecRequest::ReconciliationStatus { project, .. }
                | SpecRequest::ReconciliationRead { project, .. },
            ) => Some(format!("project:{project}")),
            Some(SpecRequest::ReconciliationResolve(r)) => Some(format!("project:{}", r.project)),
            Some(SpecRequest::ReconcileApply { plan, .. }) => {
                Some(format!("project:{}", plan.project))
            }
            Some(SpecRequest::ProjectAttach(r)) => Some(format!("project:{}", r.project)),
            Some(SpecRequest::Projects) => Some("registry".into()),
            _ => None,
        };
        let discovery_server = self.clone();
        let discovery_selector = selector.clone();
        let discovery_token = context.ct.clone();
        // Discovery can run Git or touch a slow filesystem. Its admitted permit
        // stays in the worker even if the awaiting SDK handler is dropped.
        let discovery = tokio::task::spawn_blocking(move || {
            let key = if discovery_token.is_cancelled() {
                Err(error("cancelled", "operation cancelled"))
            } else {
                key.map_or_else(|| discovery_server.execution_key(&discovery_selector), Ok)
            };
            (key, permit, management_guard)
        })
        .await;
        let (key, permit, management_guard) = match discovery {
            Ok(value) => value,
            Err(e) => return failed(error("execution_failed", e.to_string())),
        };
        if context.ct.is_cancelled() {
            return failed(error("cancelled", "operation cancelled"));
        }
        let key = match key {
            Ok(key) => key,
            Err(e) => return failed(e),
        };
        let serial = {
            let mut projects = self.projects.lock().await;
            projects.retain(|_, lock| Arc::strong_count(lock) > 1);
            projects
                .entry(key)
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let project_guard = tokio::select! {
            _ = context.ct.cancelled() => return failed(error("cancelled", "operation cancelled")),
            g = serial.lock_owned() => g,
        };
        if bounded(
            &failed(error(
                "size_limit",
                "MCP response cannot fit the request ID",
            )),
            &context.id,
        )
        .is_err()
        {
            return failed(error(
                "size_limit",
                "MCP response cannot fit the request ID",
            ));
        }
        let token = context.ct.clone();
        let mut control = crate::InvocationControl::new(move || token.is_cancelled());
        control.limited = true;
        let admission_id = context.id.clone();
        control.result_admission = Some(Arc::new(move |value| {
            bounded(&CallToolResult::structured(value.clone()), &admission_id)
        }));
        let api = self.api.with_control(control);
        let id = context.id.clone();
        let operation_name = name.to_owned();
        // Move every permit into the synchronous worker. A cancelled waiter cannot
        // release capacity while a started operation still owns filesystem work.
        let work = tokio::task::spawn_blocking(move || {
            let _permits = (permit, management_guard, project_guard);
            let output = api.check_control().and_then(|()| match request {
                Some(r) => api.execute(r).map(response),
                None => api.refresh(&selector).map(|r|json!({"context":r.context.as_ref().map(crate::storage::context_value),"data":r.value()})),
            });
            let result = match output.and_then(|v|{schema::validate_output(&operation_name,&v)?;Ok(v)}) { Ok(v) => CallToolResult::structured(v), Err(e) => failed(e) };
            bounded(&result,&id).map_or_else(failed,|()|result)
        }).await;
        match work {
            Ok(r) => r,
            Err(e) => failed(error("execution_failed", e.to_string())),
        }
    }
}
impl ServerHandler for Server {
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Owned(vec![
            ProtocolVersion::V_2025_11_25,
            ProtocolVersion::V_2026_07_28,
        ])
    }
    fn get_info(&self) -> ServerConfig {
        {
            let mut config = ServerConfig::default();
            config.capabilities = ServerCapabilities::builder().enable_tools().build();
            config
        }
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: schema::tools(),
            ..Default::default()
        })
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let name = request.name.as_ref();
        if !schema::NAMES.contains(&name) {
            return Err(ErrorData::invalid_params("unknown spec tool", None));
        }
        Ok(self
            .invoke(
                name,
                Value::Object(request.arguments.unwrap_or_default()),
                context,
            )
            .await
            .into())
    }
}

/// Run the SDK stdio service using one immutable process snapshot.
pub fn serve(runtime: Runtime) -> Result<(), SpecError> {
    serve_selected(runtime, ContextSelector::default())
}
pub(crate) fn serve_selected(runtime: Runtime, defaults: ContextSelector) -> Result<(), SpecError> {
    let server = Server {
        api: SpecApi::new(runtime)?,
        defaults,
        admitted: Arc::new(Semaphore::new(32)),
        management: Arc::new(Mutex::new(())),
        projects: Arc::new(Mutex::new(BTreeMap::new())),
    };
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(32)
        .enable_all()
        .build()
        .map_err(crate::documents::io)?
        .block_on(async move {
            let transport = rmcp::transport::async_rw::AsyncRwTransport::new_server(
                tokio::io::stdin(),
                tokio::io::stdout(),
            )
            .with_max_line_length(MAX_BYTES);
            server
                .serve(BoundedTransport(transport))
                .await
                .map_err(|e| error("mcp_transport", e.to_string()))?
                .waiting()
                .await
                .map_err(|e| error("mcp_transport", e.to_string()))?;
            Ok(())
        })
}

/// Bound every SDK envelope, including lifecycle/protocol errors and escaped IDs.
/// SDK framing remains untouched. A message that cannot fit closes the transport.
struct BoundedTransport<T>(T);
impl<T: rmcp::transport::Transport<RoleServer, Error = std::io::Error>>
    rmcp::transport::Transport<RoleServer> for BoundedTransport<T>
{
    type Error = std::io::Error;
    fn send(
        &mut self,
        item: rmcp::service::TxJsonRpcMessage<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send + 'static {
        let fits = serde_json::to_vec(&item).is_ok_and(|b| b.len() < MAX_BYTES);
        let future = if fits { Some(self.0.send(item)) } else { None };
        async move {
            match future {
                Some(f) => f.await,
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "serialized MCP envelope exceeds 8 MiB",
                )),
            }
        }
    }
    async fn receive(&mut self) -> Option<rmcp::service::RxJsonRpcMessage<RoleServer>> {
        self.0.receive().await
    }
    async fn close(&mut self) -> Result<(), Self::Error> {
        self.0.close().await
    }
}

#[cfg(test)]
mod envelope_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use rmcp::transport::Transport;
    use tokio::io::AsyncReadExt;
    #[tokio::test]
    async fn spec_projects_mcp_8_unfittable_error_keeps_id_and_writes_no_replacement() {
        let id = "id-\"\\".to_owned() + &"x".repeat(MAX_BYTES - 90);
        let result = failed(error(
            "size_limit",
            "response cannot fit the original request ID",
        ));
        assert!(bounded(&result, &RequestId::String(id.clone().into())).is_err());
        let message: rmcp::service::TxJsonRpcMessage<RoleServer> =
            serde_json::from_value(json!({"jsonrpc":"2.0","id":id,"result":result})).unwrap();
        let (writer, mut observer) = tokio::io::duplex(64);
        let inner =
            rmcp::transport::async_rw::AsyncRwTransport::new_server(tokio::io::empty(), writer);
        let mut bounded = BoundedTransport(inner);
        let refusal = bounded.send(message).await.unwrap_err();
        assert_eq!(refusal.kind(), std::io::ErrorKind::InvalidData);
        drop(bounded);
        let mut emitted = Vec::new();
        observer.read_to_end(&mut emitted).await.unwrap();
        assert!(emitted.is_empty());
    }
}
