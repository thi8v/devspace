use std::process::ExitCode;

fn main() -> ExitCode {
    match devspace::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("ERR: {err}");
            ExitCode::FAILURE
        }
    }
}
