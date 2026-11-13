//! `vibe spec`: embedded spec argv/stdio adapter.
use std::ffi::OsString;

pub fn spec(args: &[OsString]) -> u8 {
    if std::env::var_os("VIBE_SPEC_BIN").is_some() {
        eprintln!("vibe: VIBE_SPEC_BIN is deprecated and ignored; spec runs inside vibe");
    }
    let mut argv = vec![OsString::from("vibe-spec")];
    argv.extend_from_slice(args);
    vibe_spec::run_process(&argv)
}
