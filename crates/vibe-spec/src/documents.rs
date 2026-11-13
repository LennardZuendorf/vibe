//! Rooted Markdown documents. Managed authoring admission is separate from full validation.
use crate::{
    ContextSelector, RemoveDocumentRequest, SpecApi, SpecContext, SpecError, SpecResponse,
    WriteDocumentRequest,
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

pub(crate) type Files = BTreeMap<String, Vec<u8>>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentMetadata {
    pub path: String,
    pub bytes: usize,
    pub revision: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentContent {
    pub metadata: DocumentMetadata,
    pub content: String,
}
#[derive(Clone, Debug)]
pub struct DocumentList {
    pub context: SpecContext,
    pub documents: Vec<DocumentMetadata>,
}
#[derive(Clone, Debug)]
pub struct DocumentRead {
    pub context: SpecContext,
    pub document: DocumentContent,
}
impl SpecApi {
    pub fn list_documents(&self, selector: &ContextSelector) -> Result<DocumentList, SpecError> {
        let (context, documents) = crate::transaction::read_typed(self, selector, |context| {
            Ok(collect(context)?
                .into_iter()
                .map(|(path, content)| DocumentMetadata {
                    path,
                    bytes: content.len(),
                    revision: vibe_core::markers::hash_bytes(&content),
                })
                .collect())
        })?;
        Ok(DocumentList { context, documents })
    }
    pub fn read_document(
        &self,
        selector: &ContextSelector,
        logical: &str,
    ) -> Result<DocumentRead, SpecError> {
        let (context, document) = crate::transaction::read_typed(self, selector, |context| {
            let content = bytes(context, logical)?
                .ok_or_else(|| SpecError::new("document_not_found", "document does not exist"))?;
            let metadata = DocumentMetadata {
                path: logical.into(),
                bytes: content.len(),
                revision: vibe_core::markers::hash_bytes(&content),
            };
            let content = String::from_utf8(content).map_err(|_| {
                SpecError::new("invalid_document", "document is not UTF-8 Markdown")
            })?;
            Ok(DocumentContent { metadata, content })
        })?;
        Ok(DocumentRead { context, document })
    }
}
pub(crate) fn io(e: impl std::fmt::Display) -> SpecError {
    SpecError::new("document_io", e.to_string())
}
fn invalid() -> SpecError {
    SpecError::new(
        "invalid_document",
        "expected a rooted Markdown path under spec/ or superpowers/ without links or traversal",
    )
}
pub(crate) fn path(context: &SpecContext, logical: &str) -> Result<PathBuf, SpecError> {
    if logical.contains('\\')
        || logical.contains('\0')
        || logical.contains(':')
        || logical
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.eq_ignore_ascii_case(".git"))
        || !logical.ends_with(".md")
    {
        return Err(invalid());
    }
    let (prefix, relative) = logical.split_once('/').ok_or_else(invalid)?;
    if Path::new(relative)
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(invalid());
    }
    let anchor = match prefix {
        "spec" => &context.spec_root,
        "superpowers" => &context.artifact_root,
        _ => return Err(invalid()),
    };
    let target = anchor.join(relative);
    crate::registry::confined(anchor, &target).map_err(|_| invalid())?;
    match fs::symlink_metadata(&target) {
        Ok(m) if !m.is_file() => Err(invalid()),
        Ok(_) => Ok(target),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(target),
        Err(e) => Err(io(e)),
    }
}
pub(crate) fn bytes(context: &SpecContext, logical: &str) -> Result<Option<Vec<u8>>, SpecError> {
    match fs::read(path(context, logical)?) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io(e)),
    }
}
pub(crate) fn collect(context: &SpecContext) -> Result<Files, SpecError> {
    fn walk(
        context: &SpecContext,
        dir: &Path,
        prefix: &str,
        out: &mut Files,
    ) -> Result<(), SpecError> {
        crate::registry::confined(dir, dir).map_err(|_| invalid())?;
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(io(e)),
        };
        for entry in entries {
            let entry = entry.map_err(io)?;
            let name = entry.file_name().into_string().map_err(|_| invalid())?;
            let logical = format!("{prefix}/{name}");
            let kind = entry.file_type().map_err(io)?;
            if kind.is_symlink() || !(kind.is_file() || kind.is_dir()) {
                return Err(invalid());
            }
            if name.eq_ignore_ascii_case(".git") {
                return Err(invalid());
            }
            if kind.is_dir() {
                walk(context, &entry.path(), &logical, out)?;
            } else if name.ends_with(".md") {
                out.insert(
                    logical.clone(),
                    fs::read(path(context, &logical)?).map_err(io)?,
                );
            }
        }
        Ok(())
    }
    let mut out = Files::new();
    walk(context, &context.spec_root, "spec", &mut out)?;
    walk(context, &context.artifact_root, "superpowers", &mut out)?;
    Ok(out)
}
/// Private history admits only Markdown beneath its two document anchors.
pub(crate) fn admit_global_tree(context: &SpecContext) -> Result<(), SpecError> {
    fn walk(dir: &Path) -> Result<(), SpecError> {
        crate::registry::confined(dir, dir).map_err(|_| invalid())?;
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(io(e)),
        };
        for entry in entries {
            let entry = entry.map_err(io)?;
            let kind = entry.file_type().map_err(io)?;
            if kind.is_dir() {
                walk(&entry.path())?;
            } else if !kind.is_file()
                || !entry
                    .file_name()
                    .to_str()
                    .is_some_and(|n| n.ends_with(".md"))
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    walk(&context.spec_root)?;
    walk(&context.artifact_root)
}
pub(crate) fn list(api: &SpecApi, selector: &ContextSelector) -> Result<SpecResponse, SpecError> {
    let result = api.list_documents(selector)?;
    let documents: Vec<_> = result
        .documents
        .into_iter()
        .map(|d| json!({"path":d.path,"bytes":d.bytes,"revision":d.revision}))
        .collect();
    Ok(SpecResponse {
        data: json!({"documents":documents,"context":crate::storage::context_value(&result.context)}),
        context: Some(result.context),
    })
}
pub(crate) fn read(
    api: &SpecApi,
    selector: &ContextSelector,
    logical: &str,
) -> Result<SpecResponse, SpecError> {
    let result = api.read_document(selector, logical)?;
    Ok(SpecResponse {
        data: json!({"path":result.document.metadata.path,"content":result.document.content,"revision":result.document.metadata.revision,"context":crate::storage::context_value(&result.context)}),
        context: Some(result.context),
    })
}
pub(crate) fn write(
    api: &SpecApi,
    request: WriteDocumentRequest,
) -> Result<SpecResponse, SpecError> {
    crate::transaction::apply(
        api,
        &request.selector,
        &request.expected_revision,
        vec![Change {
            path: request.path,
            content: Some(request.content.into_bytes()),
        }],
    )
}
pub(crate) fn remove(
    api: &SpecApi,
    request: RemoveDocumentRequest,
) -> Result<SpecResponse, SpecError> {
    crate::transaction::apply(
        api,
        &request.selector,
        &request.expected_revision,
        vec![Change {
            path: request.path,
            content: None,
        }],
    )
}
#[derive(Clone, Debug)]
pub(crate) struct Change {
    pub path: String,
    pub content: Option<Vec<u8>>,
}

/// Typed diagnostic seam; Task 5 supplies semantic findings without running external linters.
pub(crate) fn diagnostics(runtime: &crate::Runtime, context: &SpecContext) -> serde_json::Value {
    match crate::validate::context_result(runtime, context, crate::ValidationScope::SpecOnly, false)
    {
        Ok(result) => {
            json!([{"code":"semantic_validation","message":"Recorded draft; semantic findings do not reject ordinary authoring","result":result.value()}])
        }
        Err(error) => json!([{"code":error.code,"message":error.message}]),
    }
}

pub(crate) fn logical(context: &SpecContext, physical: &Path) -> Result<String, SpecError> {
    for (prefix, root) in [
        ("spec", &context.spec_root),
        ("superpowers", &context.artifact_root),
    ] {
        if let Ok(relative) = physical.strip_prefix(root) {
            let logical = format!("{prefix}/{}", relative.to_string_lossy().replace('\\', "/"));
            path(context, &logical)?;
            return Ok(logical);
        }
    }
    Err(invalid())
}

pub(crate) fn logical_legacy(context: &SpecContext, path: &Path) -> Result<String, SpecError> {
    logical(context, &crate::storage::future_physical(path)?)
}
