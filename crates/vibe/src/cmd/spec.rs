//! `vibe spec`: hand over to the `vibe-spec` binary.
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use vibe_core::dirs::Env;

use super::doctor::find_on_path;

/// The advice printed when `vibe-spec` cannot be found.
pub const MISSING: &str = "vibe: vibe-spec is not installed \u{2014} build it with `cargo build -p vibe-spec` or install the vibe-spec binary";

fn locate(env: &Env) -> Option<PathBuf> {
    if let Some(bin) = env.var("VIBE_SPEC_BIN").filter(|b| !b.is_empty()) {
        let path = PathBuf::from(bin);
        return path.is_file().then_some(path);
    }
    find_on_path("vibe-spec", env.var("PATH"))
}

/// Runs `$VIBE_SPEC_BIN`, else `vibe-spec` from `PATH`, with `args` and inherited stdio; its
/// exit code is the result. A missing binary prints one advice line and exits 0.
pub fn spec(args: &[OsString], env: &Env) -> u8 {
    let Some(bin) = locate(env) else {
        println!("{MISSING}");
        return 0;
    };
    match Command::new(&bin).args(args).status() {
        Ok(status) => status
            .code()
            .map_or(1, |c| u8::try_from(c & 0xff).unwrap_or(1)),
        Err(e) => {
            eprintln!("vibe: {}: {e}", bin.display());
            1
        }
    }
}
