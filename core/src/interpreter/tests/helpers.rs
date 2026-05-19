use crate::ast::Program;
use crate::runtime::Diagnostic;
use crate::parser::Parser;
use crate::semantics::validate;
use crate::interpreter::run;

pub fn parse_and_validate(source: &str) -> Result<Program, Diagnostic> {
    let program = Parser::parse_source(source)?;
    validate(&program)?;
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
