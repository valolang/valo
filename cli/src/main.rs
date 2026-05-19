use std::{env, fs, process};

use valo_core::run_file;

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

            let output = run_file(&path).map_err(|err| {
                let source = fs::read_to_string(&path).unwrap_or_default();
                err.render(&path, &source)
            })?;

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
