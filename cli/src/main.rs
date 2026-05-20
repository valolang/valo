use std::{env, process};

mod commands;

fn main() {
    if let Err(error) = real_main() {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

fn real_main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err(usage());
    };

    match command.as_str() {
        "run" => commands::run(env::args().skip(2)),
        "repl" => commands::repl(),
        "check" => commands::check(env::args().skip(2)),
        "version" => {
            println!("Valo 0.1.0");
            Ok(())
        }
        "--help" | "-h" | "help" => {
            println!("{}", usage());
            Ok(())
        }
        _ => Err(format!("Unknown command: {}", command)),
    }
}

fn usage() -> String {
    r#"Valo 0.1.0 - A modern, Basic-inspired language.

Usage: valo <command> [args]

Commands:
    run <file>      Run a Valo file (.valo, .bas, .cls)
    repl            Start an interactive REPL
    check <file>    Validate a Valo file without running
    version         Print version information
    help            Print this help message
"#
    .to_string()
}
