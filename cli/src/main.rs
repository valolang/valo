use std::{env, fs, process};

use valo_core::run_source;

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
        "run" => {
            let Some(path) = args.next() else {
                return Err(usage());
            };

            if args.next().is_some() {
                return Err(usage());
            }

            let source =
                fs::read_to_string(&path).map_err(|err| format!("failed to read {path}: {err}"))?;
            let output = run_source(&source).map_err(|err| err.to_string())?;

            for line in output {
                println!("{line}");
            }

            Ok(())
        }
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: valo run <file>".to_string()
}
