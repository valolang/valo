use std::{env, process};

mod commands;

fn main() {
    if let Err(error) = real_main() {
        if error.starts_with('\u{1b}')
            || error.contains("error[")
            || error.starts_with("error")
            || error.starts_with("warning")
            || error.starts_with("note")
            || error.starts_with("help")
        {
            eprintln!("{error}");
        } else {
            eprintln!("error: {error}");
        }
        process::exit(1);
    }
}

fn real_main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut color = commands::ColorChoice::Auto;
    let first = args.next();
    let Some(command) = (if first.as_deref() == Some("--color") {
        let choice = args.next().ok_or_else(|| {
            "usage: valo --color <auto|always|never> <command> [args]".to_string()
        })?;
        color = commands::ColorChoice::parse(&choice)?;
        args.next()
    } else {
        first
    }) else {
        println!("{}", usage());
        return Ok(());
    };

    match command.as_str() {
        "run" => commands::run(args, color),
        "repl" => commands::repl(color),
        "check" => commands::check(args, color),
        "build" => commands::build(args, color),
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
    r#"Valo 0.1.0 - VB.NET-inspired syntax, interpreter and experimental native backend.

Usage: valo <command> [args]

Commands:
    run <file>      Run a Valo file (.valo)
    repl            Start an interactive REPL
    check <file>    Validate a Valo file without running
    build <file>    Compile the supported subset to a native executable
    version         Print version information
    help            Print this help message
Options:
    --color auto|always|never
    build: --emit=llvm-ir|obj|exe  --release  -o <output>
    build: VALO_LLVM_BIN can name the LLVM tool directory
"#
    .to_string()
}
