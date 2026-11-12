//! Version-1 spec registration reads. Storage mutations share these records.
use crate::api::{Execution, SpecError};
use crate::{BranchId, ProjectId, Runtime, StorageMode};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct Checkout {
    pub path: PathBuf,
    pub common_dir: Option<PathBuf>,
}
#[derive(Clone, Debug)]
pub(crate) struct Project {
    pub id: ProjectId,
    #[allow(dead_code)] // Retain registration metadata for management operations.
    pub name: String,
    pub mode: StorageMode,
    pub main_checkout: PathBuf,
    pub checkouts: Vec<Checkout>,
    #[allow(dead_code)]
    pub remote_aliases: Vec<String>,
}
#[derive(Clone, Debug)]
pub(crate) struct RegistrySnapshot {
    pub root: PathBuf,
    pub projects: Vec<Project>,
    pub revision: String,
}
#[derive(Clone, Debug)]
pub(crate) struct Branch {
    pub id: BranchId,
    pub code_ref: String,
    pub spec_ref: String,
    #[allow(dead_code)] // Historical metadata never supplies a current code context.
    pub parent_spec_revision: Option<String>,
    #[allow(dead_code)]
    pub last_code_head: Option<String>,
    #[allow(dead_code)]
    pub last_code_ref: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) struct ProjectState {
    pub branches: Vec<Branch>,
    #[allow(dead_code)] // Snapshot records do not participate in checkout resolution.
    pub snapshots: Vec<crate::refresh::SnapshotBinding>,
    pub observations: Vec<crate::refresh::Observation>,
    pub revision: String,
}

fn invalid(message: impl Into<String>) -> SpecError {
    SpecError::new("invalid_registry", message)
}
fn object<'a>(v: &'a Value, fields: &[&str]) -> Result<&'a Map<String, Value>, SpecError> {
    let obj = v.as_object().ok_or_else(|| invalid("expected an object"))?;
    if obj.keys().any(|key| !fields.contains(&key.as_str())) {
        return Err(invalid("unknown registry field"));
    }
    Ok(obj)
}
fn field<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a Value, SpecError> {
    obj.get(key)
        .ok_or_else(|| invalid(format!("missing {key}")))
}
fn string(v: &Value) -> Result<String, SpecError> {
    v.as_str()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| invalid("expected a non-empty string"))
}
fn array(v: &Value) -> Result<&Vec<Value>, SpecError> {
    v.as_array().ok_or_else(|| invalid("expected an array"))
}
fn optional_string(obj: &Map<String, Value>, key: &str) -> Result<Option<String>, SpecError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => string(v).map(Some),
    }
}
fn optional_oid(obj: &Map<String, Value>, key: &str) -> Result<Option<String>, SpecError> {
    let value = optional_string(obj, key)?;
    if value
        .as_ref()
        .is_some_and(|s| !matches!(s.len(), 40 | 64) || !s.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err(invalid(format!("{key} must be a Git object ID or null")));
    }
    Ok(value)
}
fn version(obj: &Map<String, Value>) -> Result<(), SpecError> {
    if field(obj, "v")?.as_u64() != Some(1) {
        return Err(invalid("unsupported registry version"));
    }
    Ok(())
}

/// Strings are portable. Raw OS paths use a tagged native encoding, never lossy UTF-8.
pub(crate) fn path(v: &Value) -> Result<PathBuf, SpecError> {
    let p = if let Some(s) = v.as_str() {
        PathBuf::from(s)
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let obj = object(v, &["unix_bytes"])?;
            let bytes = array(field(obj, "unix_bytes")?)?
                .iter()
                .map(|b| {
                    b.as_u64()
                        .and_then(|n| u8::try_from(n).ok())
                        .ok_or_else(|| invalid("invalid Unix path byte"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            PathBuf::from(std::ffi::OsString::from_vec(bytes))
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            let obj = object(v, &["windows_wide"])?;
            let wide = array(field(obj, "windows_wide")?)?
                .iter()
                .map(|b| {
                    b.as_u64()
                        .and_then(|n| u16::try_from(n).ok())
                        .ok_or_else(|| invalid("invalid Windows path unit"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            PathBuf::from(std::ffi::OsString::from_wide(&wide))
        }
        #[cfg(not(any(unix, windows)))]
        {
            return Err(invalid("unsupported path encoding"));
        }
    };
    if !p.is_absolute()
        || p.components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || p.as_os_str().as_encoded_bytes().contains(&0)
    {
        return Err(invalid(
            "registered paths must be absolute without traversal or NUL",
        ));
    }
    Ok(p)
}

pub(crate) fn store_root(runtime: &Runtime) -> Result<PathBuf, SpecError> {
    let cx = Execution::new(runtime);
    let var = |key| cx.var_os(key).filter(|v| !v.is_empty()).map(PathBuf::from);
    let root = if let Some(root) = var("VIBE_STORE") {
        root
    } else if cfg!(windows) {
        var("APPDATA")
            .ok_or_else(|| SpecError::new("invalid_context", "APPDATA is unset"))?
            .join("vibe")
    } else if let Some(root) = var("XDG_CONFIG_HOME").filter(|p| p.is_absolute()) {
        root.join("vibe")
    } else {
        var("HOME")
            .ok_or_else(|| SpecError::new("invalid_context", "HOME is unset"))?
            .join(".config/vibe")
    };
    Ok(crate::init::normalise(&runtime.cwd.join(root)))
}

/// Refuse symlinks anywhere beneath the explicit storage anchor, including ancestors.
pub(crate) fn confined(anchor: &Path, target: &Path) -> Result<(), SpecError> {
    let rel = target
        .strip_prefix(anchor)
        .map_err(|_| invalid("path is outside the storage anchor"))?;
    if !anchor.is_absolute()
        || rel
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(invalid("invalid storage path component"));
    }
    for here in target.ancestors().collect::<Vec<_>>().into_iter().rev() {
        match std::fs::symlink_metadata(here) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(invalid("symlink in managed storage path"));
            }
            Ok(m) if !(m.is_dir() || m.is_file()) => {
                return Err(invalid("non-regular managed storage entry"));
            }
            Ok(m) if here != target && !m.is_dir() => {
                return Err(invalid("non-directory managed storage ancestor"));
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(invalid(format!("cannot inspect storage: {e}"))),
        }
    }
    Ok(())
}
fn read(anchor: &Path, target: &Path) -> Result<Option<Vec<u8>>, SpecError> {
    confined(anchor, target)?;
    if std::fs::symlink_metadata(target).is_ok_and(|m| !m.is_file()) {
        return Err(invalid("registration must be a regular file"));
    }
    match std::fs::read(target) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(invalid(format!("cannot read registration: {e}"))),
    }
}

pub(crate) fn load(runtime: &Runtime) -> Result<RegistrySnapshot, SpecError> {
    let root = store_root(runtime)?;
    let Some(bytes) = read(&root, &root.join("spec/registry.json"))? else {
        return Ok(RegistrySnapshot {
            root,
            projects: Vec::new(),
            revision: "absent".into(),
        });
    };
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|e| invalid(format!("invalid registry JSON: {e}")))?;
    let obj = object(&value, &["v", "projects"])?;
    version(obj)?;
    let mut ids = BTreeSet::new();
    let projects = array(field(obj, "projects")?)?
        .iter()
        .map(|v| {
            let p = object(
                v,
                &[
                    "id",
                    "name",
                    "mode",
                    "main_checkout",
                    "checkouts",
                    "remote_aliases",
                ],
            )?;
            let id = ProjectId::new(string(field(p, "id")?)?).map_err(|e| invalid(e.message))?;
            if !ids.insert(id.clone()) {
                return Err(invalid("duplicate project ID"));
            }
            let mode = match field(p, "mode")?.as_str() {
                Some("repo-first") => StorageMode::RepoFirst,
                Some("global-first") => StorageMode::GlobalFirst,
                _ => return Err(invalid("invalid project storage mode")),
            };
            let main_checkout = path(field(p, "main_checkout")?)?;
            let checkouts = array(field(p, "checkouts")?)?
                .iter()
                .map(|v| {
                    let c = object(v, &["path", "common_dir"])?;
                    Ok(Checkout {
                        path: path(field(c, "path")?)?,
                        common_dir: match field(c, "common_dir")? {
                            Value::Null => None,
                            v => Some(path(v)?),
                        },
                    })
                })
                .collect::<Result<Vec<_>, SpecError>>()?;
            if !checkouts.iter().any(|c| c.path == main_checkout) {
                return Err(invalid("main checkout is not registered"));
            }
            let remote_aliases = array(field(p, "remote_aliases")?)?
                .iter()
                .map(string)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Project {
                id,
                name: string(field(p, "name")?)?,
                mode,
                main_checkout,
                checkouts,
                remote_aliases,
            })
        })
        .collect::<Result<Vec<_>, SpecError>>()?;
    Ok(RegistrySnapshot {
        root,
        projects,
        revision: vibe_core::markers::hash_bytes(&bytes),
    })
}

pub(crate) fn load_project(
    registry: &RegistrySnapshot,
    id: &ProjectId,
) -> Result<ProjectState, SpecError> {
    let target = registry
        .root
        .join("spec/projects")
        .join(id.as_str())
        .join("project.json");
    let bytes = read(&registry.root, &target)?
        .ok_or_else(|| invalid("registered project.json is missing"))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|e| invalid(format!("invalid project JSON: {e}")))?;
    let obj = object(&value, &["v", "branches", "snapshots", "observations"])?;
    version(obj)?;
    let mut ids = BTreeSet::new();
    let mut refs = BTreeSet::new();
    let branches = array(field(obj, "branches")?)?
        .iter()
        .map(|v| {
            let b = object(
                v,
                &[
                    "id",
                    "code_ref",
                    "spec_ref",
                    "parent_spec_revision",
                    "last_code_head",
                    "last_code_ref",
                ],
            )?;
            let id = BranchId::new(string(field(b, "id")?)?).map_err(|e| invalid(e.message))?;
            let code_ref = string(field(b, "code_ref")?)?;
            let spec_ref = string(field(b, "spec_ref")?)?;
            if !crate::context::valid_ref(&code_ref) || !crate::context::valid_ref(&spec_ref) {
                return Err(invalid("invalid branch ref"));
            }
            if !ids.insert(id.clone()) || !refs.insert(code_ref.clone()) {
                return Err(invalid("duplicate branch identity"));
            }
            let last_code_ref = optional_string(b, "last_code_ref")?;
            if last_code_ref
                .as_ref()
                .is_some_and(|r| !crate::context::valid_ref(r))
            {
                return Err(invalid("invalid last_code_ref"));
            }
            Ok(Branch {
                id,
                code_ref,
                spec_ref,
                parent_spec_revision: optional_oid(b, "parent_spec_revision")?,
                last_code_head: optional_oid(b, "last_code_head")?,
                last_code_ref,
            })
        })
        .collect::<Result<Vec<_>, SpecError>>()?;
    let snapshots = array(field(obj, "snapshots")?)?
        .iter()
        .map(crate::refresh::SnapshotBinding::parse)
        .collect::<Result<Vec<_>, _>>()?;
    let observations = obj
        .get("observations")
        .map(|v| {
            array(v)?
                .iter()
                .map(crate::refresh::Observation::parse)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    crate::refresh::validate_identities(id, &branches, &snapshots, &observations)?;
    Ok(ProjectState {
        branches,
        snapshots,
        observations,
        revision: vibe_core::markers::hash_bytes(&bytes),
    })
}

/// Conservative equality for ordinary network origins; never consult SSH aliases or Git rewrites.
pub(crate) fn canonical_remote(value: &str) -> Option<String> {
    if value.is_empty()
        || value.chars().any(|c| c.is_whitespace() || c.is_control())
        || value.contains(['\\', '?', '#'])
    {
        return None;
    }
    let (host, path) = if let Some((scheme, rest)) = value.split_once("://") {
        let ssh = scheme.eq_ignore_ascii_case("ssh");
        if !ssh && !scheme.eq_ignore_ascii_case("https") {
            return None;
        }
        let (authority, path) = rest.split_once('/')?;
        if authority.matches('@').count() > 1 {
            return None;
        }
        let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
        let host = if let Some((h, port)) = host.split_once(':') {
            if port != if ssh { "22" } else { "443" } {
                return None;
            }
            h
        } else {
            host
        };
        (host, path)
    } else {
        let (authority, path) = value.split_once(':')?;
        if authority.contains('/') || authority.matches('@').count() > 1 {
            return None;
        }
        (
            authority.rsplit_once('@').map_or(authority, |(_, h)| h),
            path,
        )
    };
    if !host.contains('.')
        || host.starts_with('.')
        || host.ends_with('.')
        || host.contains("..")
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
    {
        return None;
    }
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    if path.is_empty()
        || path.contains('%')
        || path
            .split('/')
            .any(|p| p.is_empty() || matches!(p, "." | ".."))
    {
        return None;
    }
    Some(format!("{}/{}", host.to_ascii_lowercase(), path))
}
pub(crate) fn origin_alias(cx: &Execution<'_>, checkout: &Path) -> Option<String> {
    let bytes = crate::git::git(
        cx,
        checkout,
        &["config", "--local", "--get", "remote.origin.url"],
    )?;
    canonical_remote(std::str::from_utf8(&bytes).ok()?.trim_end_matches('\n'))
}
pub(crate) fn path_value(path: &Path) -> Value {
    if let Some(text) = path.to_str() {
        return Value::String(text.into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        serde_json::json!({"unix_bytes":path.as_os_str().as_bytes()})
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        serde_json::json!({"windows_wide":path.as_os_str().encode_wide().collect::<Vec<_>>()})
    }
    #[cfg(not(any(unix, windows)))]
    {
        Value::Null
    }
}
pub(crate) fn project_value(p: &Project) -> Value {
    serde_json::json!({"id":p.id.as_str(),"name":p.name,"mode":p.mode.as_str(),
        "main_checkout":path_value(&p.main_checkout),"checkouts":p.checkouts.iter().map(|c|serde_json::json!({"path":path_value(&c.path),"common_dir":c.common_dir.as_deref().map(path_value)})).collect::<Vec<_>>(),"remote_aliases":p.remote_aliases})
}
pub(crate) fn branch_value(b: &Branch) -> Value {
    serde_json::json!({"id":b.id.as_str(),"code_ref":b.code_ref,"spec_ref":b.spec_ref,"parent_spec_revision":b.parent_spec_revision,"last_code_head":b.last_code_head,"last_code_ref":b.last_code_ref})
}
pub(crate) fn write_registry(snapshot: &RegistrySnapshot) -> Result<(), SpecError> {
    write_json(
        &snapshot.root,
        &snapshot.root.join("spec/registry.json"),
        &serde_json::json!({"v":1,"projects":snapshot.projects.iter().map(project_value).collect::<Vec<_>>()}),
    )
}
pub(crate) fn project_dir(snapshot: &RegistrySnapshot, id: &ProjectId) -> PathBuf {
    snapshot.root.join("spec/projects").join(id.as_str())
}
pub(crate) fn project_state_value(state: &ProjectState) -> Value {
    serde_json::json!({"v":1,"branches":state.branches.iter().map(branch_value).collect::<Vec<_>>(),"snapshots":state.snapshots.iter().map(crate::refresh::SnapshotBinding::value).collect::<Vec<_>>(),"observations":state.observations.iter().map(crate::refresh::Observation::value).collect::<Vec<_>>()})
}
pub(crate) fn write_project(
    snapshot: &RegistrySnapshot,
    id: &ProjectId,
    state: &ProjectState,
) -> Result<(), SpecError> {
    write_json(
        &snapshot.root,
        &project_dir(snapshot, id).join("project.json"),
        &project_state_value(state),
    )
}
pub(crate) fn write_json(anchor: &Path, path: &Path, value: &Value) -> Result<(), SpecError> {
    confined(anchor, path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| invalid(e.to_string()))?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| invalid(e.to_string()))?;
    vibe_core::fs::write_atomic(path, &bytes).map_err(|e| invalid(e.to_string()))
}
pub(crate) fn lock_controlled(
    root: &Path,
    path: &Path,
    control: Option<&crate::InvocationControl>,
) -> Result<vibe_core::lock::Lock, SpecError> {
    confined(root, path)?;
    for _ in 0..200 {
        if let Some(control) = control {
            control.check()?;
        }
        match vibe_core::lock::Lock::acquire(path, vibe_core::lock::STALE)
            .map_err(|e| invalid(e.to_string()))?
        {
            vibe_core::lock::Acquired::Got(lock) => return Ok(lock),
            vibe_core::lock::Acquired::Busy => {
                #[cfg(test)]
                tests::waiting(path);
                #[cfg(test)]
                crate::transaction::tests::waiting(path);
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
        }
    }
    Err(SpecError::new(
        "locked",
        "storage is locked; verify the holder before manual recovery",
    ))
}
pub(crate) fn expect(actual: &str, expected: &str) -> Result<(), SpecError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SpecError::new(
            "revision_changed",
            "registration changed; read a fresh revision before retrying",
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::{Execution, Path, PathBuf};
    static WAITING: std::sync::Mutex<Option<(PathBuf, std::sync::mpsc::Sender<()>)>> =
        std::sync::Mutex::new(None);
    pub(super) fn waiting(path: &Path) {
        let mut hook = WAITING.lock().unwrap();
        if hook.as_ref().is_some_and(|(target, _)| target == path)
            && let Some((_, sender)) = hook.take()
        {
            sender.send(()).unwrap();
        }
    }
    #[test]
    fn setup_observes_one_code_state_after_waiting_for_registry_lock() {
        // spec-projects-mcp/3: a real lock hold makes the branch switch ordering deterministic.
        use crate::{
            ContextSelector, InitProjectRequest, Runtime, SpecApi, SpecRequest, StorageMode,
        };
        use std::collections::BTreeMap;
        let owner = vibe_core::testscratch::scratch("registry", "setup-lock-switch");
        let code = owner.join("code");
        let store = owner.join("store");
        std::fs::create_dir_all(&code).unwrap();
        let runtime = Runtime {
            cwd: code.clone(),
            env: BTreeMap::from([
                ("PATH".into(), std::env::var_os("PATH").unwrap_or_default()),
                ("HOME".into(), owner.as_os_str().to_owned()),
                ("VIBE_STORE".into(), store.as_os_str().to_owned()),
                ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
            ]),
        };
        let run = |args: &[&str]| {
            let out = Execution::new(&runtime)
                .command("git")
                .args([
                    "-c",
                    "user.name=test",
                    "-c",
                    "user.email=t@t",
                    "-c",
                    "commit.gpgsign=false",
                    "-c",
                    "core.hooksPath=",
                ])
                .args(args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8(out.stdout).unwrap().trim().to_owned()
        };
        run(&["init", "-b", "main"]);
        run(&["commit", "--allow-empty", "-m", "main"]);
        let lock_path = store.join("spec/registry.lock");
        let held = match vibe_core::lock::Lock::acquire(&lock_path, vibe_core::lock::STALE).unwrap()
        {
            vibe_core::lock::Acquired::Got(lock) => lock,
            _ => panic!("unexpected lock"),
        };
        let (send, receive) = std::sync::mpsc::channel();
        *WAITING.lock().unwrap() = Some((lock_path, send));
        let api = SpecApi::new(runtime.clone()).unwrap();
        let pending = std::thread::spawn(move || {
            api.execute(SpecRequest::ProjectInit(InitProjectRequest {
                selector: ContextSelector::default(),
                mode: StorageMode::GlobalFirst,
                name: None,
                expected_revision: "absent".into(),
            }))
        });
        receive
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        run(&["switch", "-c", "topic"]);
        run(&["commit", "--allow-empty", "-m", "topic"]);
        let head = run(&["rev-parse", "HEAD"]);
        drop(held);
        let result = pending.join().unwrap();
        match result {
            Ok(response) => {
                assert_eq!(response.data["branches"][0]["code_ref"], "refs/heads/topic");
                assert_eq!(
                    response.data["branches"][0]["last_code_ref"],
                    "refs/heads/topic"
                );
                assert_eq!(response.data["branches"][0]["last_code_head"], head);
                assert_eq!(
                    response.context.unwrap().code_ref.as_deref(),
                    Some("refs/heads/topic")
                );
            }
            Err(error) => {
                assert!(
                    !store.join("spec/registry.json").exists(),
                    "{error}: failed setup published mixed registration"
                );
            }
        }
    }
    #[test]
    fn canonical_remote_strips_credentials_but_preserves_repository_boundaries() {
        // spec-projects-mcp/3: independently specified equivalent URLs and refusal controls.
        for value in [
            "git@github.com:Org/Repo.git",
            "https://u:secret@GITHUB.COM:443/Org/Repo.git/",
            "HTTPS://u:p%40ss@github.com/Org/Repo",
            "ssh://git@github.com:22/Org/Repo.git",
        ] {
            assert_eq!(
                super::canonical_remote(value).as_deref(),
                Some("github.com/Org/Repo"),
                "{value}"
            );
        }
        for value in [
            "https://u@evil@github.com/Org/Repo",
            "ssh://git@github.com:2222/Org/Repo",
            "file:///Org/Repo",
            "git@host-alias:Org/Repo",
            "https://github.com/Org/../Repo",
            "https://github.com/Org%2fRepo",
            "https://github.com/Org/Repo?token=secret",
        ] {
            assert_eq!(super::canonical_remote(value), None, "{value}");
        }
        assert_eq!(
            super::canonical_remote("https://github.com/org/Repo").as_deref(),
            Some("github.com/org/Repo")
        );
    }
    #[cfg(unix)]
    #[test]
    fn native_path_codec_preserves_non_utf8_bytes() {
        use std::os::unix::ffi::OsStrExt;
        // spec-projects-mcp/2: this check does not require native filesystem support.
        let decoded =
            super::path(&serde_json::json!({"unix_bytes":[47,116,109,112,47,255]})).unwrap();
        assert_eq!(decoded.as_os_str().as_bytes(), b"/tmp/\xff");
        assert!(super::path(&serde_json::json!({"unix_bytes":[47,0,255]})).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn native_path_codec_preserves_unpaired_windows_surrogates() {
        use std::os::windows::ffi::OsStrExt;
        // spec-projects-mcp/2: registered paths retain native UTF-16 units.
        let decoded =
            super::path(&serde_json::json!({"windows_wide":[67,58,92,116,109,112,92,55296]}))
                .unwrap();
        assert_eq!(
            decoded.as_os_str().encode_wide().collect::<Vec<_>>(),
            vec![67, 58, 92, 116, 109, 112, 92, 55296]
        );
        assert!(super::path(&serde_json::json!({"windows_wide":[67,58,92,0]})).is_err());
    }
}
