use std::io::{Stdout, Write, stdin, stdout};

use clap::{Command, CommandFactory, FromArgMatches};

use crate::{Cli, Context, DsError, Result, run_command};

pub fn run() -> Result {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let mut stdout = stdout();
    let mut buffer = String::new();

    let cli = Cli::command()
        .subcommand(
            Command::new("quit")
                .about("Quit the REPL")
                .visible_alias("exit"),
        )
        // we remove the need to put the program's name because we are in REPL mode.
        .no_binary_name(true);

    loop {
        buffer.clear();
        readline(&mut stdout, &mut buffer)?;

        match respond(cli.clone(), buffer.trim()) {
            Ok(quit) if quit => {
                break;
            }
            Ok(_) => {}
            Err(DsError::ClapError(clap_err)) => {
                println!("{clap_err}");
            }
            Err(err) => {
                eprintln!("ERROR: {err}");
            }
        }
    }
    Ok(())
}

pub fn readline(stdout: &mut Stdout, buffer: &mut String) -> Result {
    write!(stdout, "ds> ")?;
    stdout.flush()?;

    stdin().read_line(buffer)?;
    Ok(())
}

pub fn respond(cli: Command, cmd: &str) -> Result<bool> {
    let args = shlex::split(cmd).ok_or(DsError::InvalidREPL)?;
    let matches = cli.try_get_matches_from(args)?;

    match matches.subcommand() {
        Some(("quit", _)) => {
            return Ok(true);
        }
        _ => {}
    }
    let args = Cli::from_arg_matches(&matches)?;
    let mut ctx = Context::new(args.dir()?)?;

    run_command(args, &mut ctx, true)?;

    ctx.terminate()?;

    Ok(false)
}
