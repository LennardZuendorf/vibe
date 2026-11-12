#![allow(clippy::unwrap_used)]
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use vibe_spec::{ContextSelector, InvocationControl, Runtime, SpecApi, SpecRequest};
#[test]
fn cancelled_call_refuses_before_registry_or_project_effects() {
    let scratch = vibe_core::testscratch::scratch("vibe-spec", "spec-control");
    let runtime = Runtime {
        cwd: scratch.as_ref().to_owned(),
        env: BTreeMap::new(),
    };
    let cancelled = Arc::new(AtomicBool::new(true));
    let token = cancelled.clone();
    let api = SpecApi::new(runtime)
        .unwrap()
        .with_control(InvocationControl::new(move || token.load(Ordering::SeqCst)));
    assert_eq!(
        api.execute(SpecRequest::Context(ContextSelector::default()))
            .unwrap_err()
            .code,
        "cancelled"
    );
    assert!(
        std::fs::read_dir(scratch.as_ref())
            .unwrap()
            .next()
            .is_none()
    );
}
