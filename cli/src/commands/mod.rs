use std::io::{self, Write};
use valo_core::{Frame, Interpreter, Stmt, validate_snippet};

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

pub fn run(mut args: impl Iterator<Item = String>, color: ColorChoice) -> Result<(), String> {
    let Some(path) = args.next() else {
        return Err("usage: valo run <file>".to_string());
    };

    if args.next().is_some() {
        return Err("usage: valo run <file>".to_string());
    }

    let path = valo_core::resolve_entrypoint(&path).map_err(|err| {
        let map = valo_core::SourceMap::new();
        err.render_colored(&map, color.enabled())
    })?;

    let project = match valo_core::load_project(&path) {
        Ok(project) => project,
        Err((err, map)) => return Err(err.render_colored(&map, color.enabled())),
    };

    if let Err(err) = valo_core::validate_project(&project) {
        return Err(err.render_colored(&project.source_map, color.enabled()));
    }

    let output = valo_core::Interpreter::new()
        .run_project(&project)
        .map_err(|err| err.render_colored(&project.source_map, color.enabled()))?;

    for line in output {
        println!("{line}");
    }

    Ok(())
}

pub fn check(mut args: impl Iterator<Item = String>, color: ColorChoice) -> Result<(), String> {
    let Some(path) = args.next() else {
        return Err("usage: valo check <file>".to_string());
    };
    let path = valo_core::resolve_entrypoint(&path).map_err(|err| {
        let map = valo_core::SourceMap::new();
        err.render_colored(&map, color.enabled())
    })?;

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

pub fn repl(color: ColorChoice) -> Result<(), String> {
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

        let lower = line.to_lowercase();
        let is_decl = lower.starts_with("dim ")
            || lower.starts_with("private ")
            || lower.starts_with("public ")
            || lower.starts_with("static ")
            || lower.starts_with("const ")
            || lower.starts_with("sub ")
            || lower.starts_with("function ")
            || lower.starts_with("type ")
            || lower.starts_with("enum ")
            || lower.starts_with("class ")
            || lower.starts_with("interface ")
            || lower.starts_with("declare ")
            || lower.starts_with("option ")
            || lower.starts_with("imports ");

        let source_content = if is_decl {
            line.to_string()
        } else {
            format!("Sub Main()\n{}\nEnd Sub", line)
        };

        let mut source_map = valo_core::SourceMap::new();
        let file_id = source_map.add("repl".to_string(), source_content.clone());
        let mut program = match valo_core::parse_source_with_id(&source_content, file_id) {
            Ok(p) => p,
            Err(err) => {
                eprintln!("{}", err.render_colored(&source_map, color.enabled()));
                continue;
            }
        };

        if !is_decl
            && let Some(main) = program
                .procedures
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case("main"))
            && main.body.len() == 1
            && let Stmt::SubCall { args, .. } = &main.body[0]
            && args.is_empty()
        {
            let expr_content = format!("Sub Main()\nDebug.Print {}\nEnd Sub", line);
            let test_file_id = source_map.add("repl".to_string(), expr_content.clone());
            if let Ok(expr_program) = valo_core::parse_source_with_id(&expr_content, test_file_id)
                && validate_snippet(&expr_program).is_ok()
            {
                program = expr_program;
            }
        }

        if let Err(err) = validate_snippet(&program) {
            eprintln!("{}", err.render_colored(&source_map, color.enabled()));
        } else {
            match interpreter.run_repl_snippet(&program, &mut global_frame) {
                Ok(output) => {
                    for out_line in output {
                        println!("{}", out_line);
                    }
                }
                Err(err) => eprintln!("{}", err.render_colored(&source_map, color.enabled())),
            }
        }
    }
    Ok(())
}
