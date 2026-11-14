//! The `curbcut` binary.
//!
//! Thin wrapper over [`curbcut_cli::run`]. The panic command unwinds through
//! here, so the Rust runtime exits with 101.

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = curbcut_cli::run(&args);
    std::process::ExitCode::from(code as u8)
}
