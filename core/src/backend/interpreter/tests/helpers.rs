use crate::backend::interpreter::run;
use crate::frontend::ast::Program;
use crate::frontend::parser::Parser;
use crate::runtime::{Diagnostic, FileId};

pub fn parse_and_validate(source: &str) -> Result<Program, Diagnostic> {
    let program = Parser::parse_source(source, FileId::default())?;
    crate::frontend::semantics::validate(&program)?;
    Ok(program)
}

pub fn parse_and_validate_snippet(source: &str) -> Result<Program, Diagnostic> {
    let program = Parser::parse_source(source, FileId::default())?;
    crate::frontend::semantics::validate_snippet(&program)?;
    Ok(program)
}

pub fn run_source(source: &str) -> Vec<String> {
    let program = parse_and_validate(source).unwrap();
    run(&program).unwrap()
}

pub fn source_error(source: &str) -> String {
    match parse_and_validate(source) {
        Ok(program) => run(&program).unwrap_err().to_string(),
        Err(error) => error.to_string(),
    }
}

pub fn source_diagnostic(source: &str) -> Diagnostic {
    match parse_and_validate(source) {
        Ok(program) => run(&program).unwrap_err(),
        Err(error) => error,
    }
}

pub fn run_file_diagnostic(path: impl AsRef<std::path::Path>) -> Diagnostic {
    match crate::frontend::modules::load_project(path) {
        Ok(project) => {
            if let Err(err) = crate::frontend::semantics::validate_project(&project) {
                return err;
            }
            crate::backend::interpreter::Interpreter::new()
                .run_project(&project)
                .unwrap_err()
        }
        Err((err, _)) => err,
    }
}
