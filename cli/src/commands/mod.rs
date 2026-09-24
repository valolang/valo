use std::io::{self, Write};
use valo_core::backend::llvm::{EmitKind, LlvmTools, NativeOptions};
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

    valo_core::Interpreter::new()
        .with_output_sink(|line| println!("{line}"))
        .run_project(&project)
        .map_err(|err| err.render_colored(&project.source_map, color.enabled()))?;

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
            if let Err(err) = valo_core::validate_project_for_check(&project) {
                return Err(err.render_colored(&project.source_map, color.enabled()));
            }
        }
        Err((err, map)) => return Err(err.render_colored(&map, color.enabled())),
    }

    println!("File validated successfully.");
    Ok(())
}

pub fn build(args: impl Iterator<Item = String>, color: ColorChoice) -> Result<(), String> {
    let mut source_path = None;
    let mut output = None;
    let mut kind = EmitKind::Executable;
    let mut optimize = false;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--release" => optimize = true,
            "-o" | "--output" => {
                output = Some(std::path::PathBuf::from(
                    args.next()
                        .ok_or("build: missing output path".to_string())?,
                ))
            }
            "--emit=llvm-ir" => kind = EmitKind::LlvmIr,
            "--emit=obj" => kind = EmitKind::Object,
            "--emit=exe" => kind = EmitKind::Executable,
            _ if arg.starts_with('-') => return Err(format!("build: unknown option {arg}")),
            _ if source_path.is_none() => source_path = Some(std::path::PathBuf::from(arg)),
            _ => {
                return Err(
                    "usage: valo build <file> [--emit=llvm-ir|obj|exe] [--release] [-o output]"
                        .into(),
                );
            }
        }
    }
    let source_path = source_path.ok_or(
        "usage: valo build <file> [--emit=llvm-ir|obj|exe] [--release] [-o output]".to_string(),
    )?;
    let entry = valo_core::resolve_entrypoint(&source_path).map_err(|e| {
        let map = valo_core::SourceMap::new();
        format!(
            "source discovery: {}",
            e.render_colored(&map, color.enabled())
        )
    })?;
    let project = valo_core::load_project(&entry).map_err(|(e, map)| {
        format!(
            "source discovery: {}",
            e.render_colored(&map, color.enabled())
        )
    })?;
    let compilation = valo_core::modules::Compilation::for_native(&project).map_err(|e| {
        format!(
            "semantic analysis: {}",
            e.render_colored(&project.source_map, color.enabled())
        )
    })?;
    let program = &compilation.program;
    let entry_point = compilation.resolve_entry().map_err(|e| {
        format!(
            "entry resolution: {}",
            e.render_colored(&project.source_map, color.enabled())
        )
    })?;
    let mut bodies = (0..program.functions.len())
        .map(|index| {
            compilation.lower_function_body(index).map_err(|e| {
                format!(
                    "typed HIR: {}",
                    e.render_colored(&project.source_map, color.enabled())
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    bodies.extend(
        (0..program.procedures.len())
            .map(|index| {
                compilation.lower_procedure_body(index).map_err(|e| {
                    format!(
                        "typed HIR: {}",
                        e.render_colored(&project.source_map, color.enabled())
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
    );
    let mut mir =
        valo_core::mir::lower_module(&bodies).map_err(|e| format!("MIR lowering: {e:?}"))?;
    mir.entry = Some(entry_point.body_id(program.functions.len()));
    let tools = LlvmTools::discover().map_err(|e| e.to_string())?;
    let output = output.unwrap_or_else(|| {
        let name = source_path.file_stem().unwrap_or_default();
        std::path::PathBuf::from("target/valo-native")
            .join(name)
            .with_extension(match kind {
                EmitKind::LlvmIr => "ll",
                EmitKind::Object => {
                    if cfg!(windows) {
                        "obj"
                    } else {
                        "o"
                    }
                }
                EmitKind::Executable => {
                    if cfg!(windows) {
                        "exe"
                    } else {
                        "out"
                    }
                }
            })
    });
    let artifact = valo_core::backend::llvm::build(
        &mir,
        &tools,
        &NativeOptions {
            output,
            kind,
            optimize,
        },
    )
    .map_err(|e| e.to_string())?;
    println!(
        "Built {} ({})",
        artifact.path.display(),
        artifact.target.triple
    );
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
