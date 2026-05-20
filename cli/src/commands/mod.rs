use std::{
    fs,
    io::{self, Write},
};
use valo_core::{Frame, Interpreter, Parser, run_file, validate};

pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let Some(path) = args.next() else {
        return Err("usage: valo run <file>".to_string());
    };

    if args.next().is_some() {
        return Err("usage: valo run <file>".to_string());
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

pub fn check(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let Some(path) = args.next() else {
        return Err("usage: valo check <file>".to_string());
    };

    let source = fs::read_to_string(&path).map_err(|_| format!("Could not read file: {}", path))?;
    let program = Parser::parse_source(&source).map_err(|err| err.render(&path, &source))?;
    validate(&program).map_err(|err| err.render(&path, &source))?;

    println!("File validated successfully.");
    Ok(())
}

pub fn repl() -> Result<(), String> {
    println!("Valo REPL v0.1.0 (Type 'exit' to quit)");
    let mut stdout = io::stdout();
    let mut input = String::new();

    let mut interpreter = Interpreter::new();
    let mut global_frame = Frame::default();

    loop {
        print!("> ");
        stdout.flush().unwrap();

        input.clear();
        io::stdin().read_line(&mut input).unwrap();
        let line = input.trim();

        if line == "exit" || line == "quit" {
            break;
        }
        if line.is_empty() {
            continue;
        }

        let source = format!("Sub Main()\n{}\nEnd Sub", line);
        match Parser::parse_source(&source) {
            Ok(program) => {
                if let Err(err) = validate(&program) {
                    eprintln!("Validation error: {:?}", err);
                } else {
                    match interpreter.run_repl_snippet(&program, &mut global_frame) {
                        Ok(output) => {
                            for out_line in output {
                                println!("{}", out_line);
                            }
                        }
                        Err(err) => eprintln!("Runtime error: {:?}", err),
                    }
                }
            }
            Err(err) => eprintln!("Parse error: {:?}", err),
        }
    }
    Ok(())
}
