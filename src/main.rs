use std::process::ExitCode;

use devspace::DsError;

fn main() -> ExitCode {
    match devspace::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(DsError::NoSpaceToList) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("ERROR: {err}");
            ExitCode::FAILURE
        }
    }
}
