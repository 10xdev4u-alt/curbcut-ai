//! The `curbcut-cli` binary.
//!
//! Thin wrapper over [`curbcut_cli::run`]. Arguments are collected with
//! `args_os` and converted lossily, so a non-UTF-8 argument becomes a usage
//! error instead of a panic in the argument iterator. The `panic` command
//! unwinds through here, so the Rust runtime exits with 101.

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let code = curbcut_cli::run(&args);
    std::process::ExitCode::from(code as u8)
}
