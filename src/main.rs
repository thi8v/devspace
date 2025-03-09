use std::process::ExitCode;

// use devspace::config::Config;

fn main() -> ExitCode {
    match devspace::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("ERR: {err}");
            ExitCode::FAILURE
        }
    }
}
