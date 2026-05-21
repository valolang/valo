use std::io::{self, Write};
use valo_core::{Frame, Interpreter, run_file, validate};

#[derive(Debug, Clone, Copy)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

impl ColorChoice {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            _ => Err("usage: --color must be auto, always, or never".to_string()),
        }
    }

    fn enabled(self) -> bool {
        match self {
            Self::Auto => valo_core::terminal_supports_color(),
            Self::Always => true,
            Self::Never => false,
        }
    }
}

pub fn run(mut args: impl Iterator<Item = String>, _color: ColorChoice) -> Result<(), String> {
    let Some(path) = args.next() else {
        return Err("usage: valo run <file>".to_string());
    };

    if args.next().is_some() {
        return Err("usage: valo run <file>".to_string());
    }

    let output = run_file(&path)?;

    for line in output {
        println!("{line}");
    }

    Ok(())
}

pub fn check(mut args: impl Iterator<Item = String>, color: ColorChoice) -> Result<(), String> {
    let Some(path) = args.next() else {
        return Err("usage: valo check <file>".to_string());
    };

    match valo_core::load_project(&path) {
        Ok(project) => {
            if let Err(err) = valo_core::validate_project(&project) {
                return Err(err.render_colored(&project.source_map, color.enabled()));
            }
        }
        Err((err, map)) => return Err(err.render_colored(&map, color.enabled())),
    }

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

        let source_content = format!("Sub Main()\n{}\nEnd Sub", line);
        let mut source_map = valo_core::SourceMap::new();
        let file_id = source_map.add("repl".to_string(), source_content.clone());
        match valo_core::parse_source_with_id(&source_content, file_id) {
            Ok(program) => {
                if let Err(err) = validate(&program) {
                    eprintln!("{}", err.render(&source_map));
                } else {
                    match interpreter.run_repl_snippet(&program, &mut global_frame) {
                        Ok(output) => {
                            for out_line in output {
                                println!("{}", out_line);
                            }
                        }
                        Err(err) => eprintln!("{}", err.render(&source_map)),
                    }
                }
            }
            Err(err) => eprintln!("{}", err.render(&source_map)),
        }
    }
    Ok(())
}
