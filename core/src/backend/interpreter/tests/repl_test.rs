use crate::backend::interpreter::tests::helpers::*;
use crate::backend::interpreter::{Frame, Interpreter};

#[test]
fn repl_persistence_test() {
    let mut interpreter = Interpreter::new();
    let mut frame = Frame::default();

    // 1. Dim x As Integer
    let snippet1 = parse_and_validate_snippet("Dim x As Integer").unwrap();
    interpreter.run_repl_snippet(&snippet1, &mut frame).unwrap();

    // 2. x = 42
    let snippet2 = parse_and_validate_snippet("Sub Main()\nx = 42\nEnd Sub").unwrap();
    interpreter.run_repl_snippet(&snippet2, &mut frame).unwrap();

    // 3. Debug.Print x
    let snippet3 = parse_and_validate_snippet("Sub Main()\nDebug.Print x\nEnd Sub").unwrap();
    let output = interpreter.run_repl_snippet(&snippet3, &mut frame).unwrap();

    assert_eq!(output, vec!["42"]);
}
