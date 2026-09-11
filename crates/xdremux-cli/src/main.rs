#![forbid(unsafe_code)]

use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    let code = xdremux_cli::run_from(std::env::args_os().skip(1), &mut stdout, &mut stderr);
    ExitCode::from(code)
}
