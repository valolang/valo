use std::collections::HashMap;

use crate::runtime::Diagnostic;
use crate::{Function, Procedure, Program};

use super::records::RuntimeType;
use super::values::key;
use super::{ControlFlow, Frame, RuntimeClass, RuntimeEnum};

#[derive(Debug, Default)]
pub struct Interpreter {
    pub(crate) types: HashMap<String, RuntimeType>,
    pub(crate) enums: HashMap<String, RuntimeEnum>,
    pub(crate) enum_members: HashMap<String, i64>,
    pub(crate) classes: HashMap<String, RuntimeClass>,
    pub(crate) procedures: HashMap<String, Procedure>,
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) output: Vec<String>,
    pub(crate) option_base: i64,
    pub(crate) option_compare: crate::OptionCompare,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            option_compare: crate::OptionCompare::Binary,
            ..Self::default()
        }
    }

    pub fn run(mut self, program: &Program) -> Result<Vec<String>, Diagnostic> {
        self.option_base = program.option_base;
        self.option_compare = program.option_compare;
        for type_decl in &program.types {
            self.types
                .insert(key(&type_decl.name), RuntimeType::from(type_decl));
        }
        for enum_decl in &program.enums {
            let mut members = HashMap::new();
            let mut previous = -1;
            for member in &enum_decl.members {
                let value = if let Some(expr) = &member.value {
                    self.eval_enum_const_expr(expr, &members)?
                } else {
                    previous + 1
                };
                previous = value;
                members.insert(key(&member.name), value);
                self.enum_members.insert(key(&member.name), value);
            }
            self.enums.insert(
                key(&enum_decl.name),
                RuntimeEnum {
                    name: enum_decl.name.clone(),
                    members,
                },
            );
        }
        for class_decl in &program.classes {
            self.classes
                .insert(key(&class_decl.name), RuntimeClass::from(class_decl));
        }
        for procedure in &program.procedures {
            self.procedures
                .insert(key(&procedure.name), procedure.clone());
        }
        for function in &program.functions {
            self.functions.insert(key(&function.name), function.clone());
        }

        let mut frame = Frame::default();
        for var in &program.module_vars {
            frame.declare_module(
                &var.name,
                var.ty.clone(),
                var.array.clone(),
                self.option_base,
                false,
                None,
                var.span,
                &self.types,
                &self.enums,
            )?;
        }
        for const_decl in &program.module_consts {
            let value = self.eval_expr(&const_decl.value, &mut frame)?;
            let ty = const_decl.ty.clone().unwrap_or_else(|| value.type_name());
            frame.declare_module(
                &const_decl.name,
                ty,
                None,
                self.option_base,
                true,
                Some(value),
                const_decl.span,
                &self.types,
                &self.enums,
            )?;
        }

        let Some(main) = program
            .procedures
            .iter()
            .find(|procedure| procedure.name.eq_ignore_ascii_case("main"))
        else {
            return Err(Diagnostic::new("Program must contain Sub Main()", None));
        };

        match self.exec_block(&main.body, &mut frame)? {
            ControlFlow::Continue | ControlFlow::ExitSub => Ok(self.output),
            ControlFlow::Return(_) => Err(Diagnostic::new(
                "Return is only allowed inside Function",
                Some(main.span),
            )),
            ControlFlow::ExitFunction => Err(Diagnostic::new(
                "Exit Function is only valid inside Function",
                Some(main.span),
            )),
            ControlFlow::ExitFor | ControlFlow::ExitWhile | ControlFlow::ExitDo => Err(
                Diagnostic::new("Exit statement escaped its block", Some(main.span)),
            ),
        }
    }
}

pub fn run(program: &Program) -> Result<Vec<String>, Diagnostic> {
    Interpreter::new().run(program)
}

impl Interpreter {
    fn eval_enum_const_expr(
        &self,
        expr: &crate::Expr,
        members: &HashMap<String, i64>,
    ) -> Result<i64, Diagnostic> {
        use crate::{BinaryOp, ExprKind, UnaryOp};
        match &expr.kind {
            ExprKind::Integer(value) => Ok(*value),
            ExprKind::Variable(name) => members.get(&key(name)).copied().ok_or_else(|| {
                Diagnostic::new(
                    format!("Enum member '{}' is not defined", name),
                    Some(expr.span),
                )
            }),
            ExprKind::Unary {
                op: UnaryOp::Negate,
                expr,
            } => Ok(-self.eval_enum_const_expr(expr, members)?),
            ExprKind::Binary { left, op, right } => {
                let left = self.eval_enum_const_expr(left, members)?;
                let right = self.eval_enum_const_expr(right, members)?;
                match op {
                    BinaryOp::Add => Ok(left + right),
                    BinaryOp::Subtract => Ok(left - right),
                    BinaryOp::Multiply => Ok(left * right),
                    BinaryOp::Divide => {
                        if right == 0 {
                            Err(Diagnostic::new("Division by zero", Some(expr.span)))
                        } else {
                            Ok(left / right)
                        }
                    }
                    _ => Err(Diagnostic::new(
                        "Enum value expression must be numeric",
                        Some(expr.span),
                    )),
                }
            }
            _ => Err(Diagnostic::new(
                "Enum value expression must be numeric",
                Some(expr.span),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    use crate::semantics::validate;

    fn parse_and_validate(source: &str) -> Result<Program, Diagnostic> {
        let program = Parser::parse_source(source)?;
        validate(&program)?;
        Ok(program)
    }

    fn run_source(source: &str) -> Vec<String> {
        let program = parse_and_validate(source).unwrap();
        run(&program).unwrap()
    }

    fn source_error(source: &str) -> String {
        match parse_and_validate(source) {
            Ok(program) => run(&program).unwrap_err().to_string(),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn runs_variables_and_console_writeline() {
        let output = run_source(
            r#"
Sub Main()
    Dim name As String
    Dim count As Integer
    name = "Valo"
    count = 40 + 2
    Console.WriteLine("Hello, " & name & " " & count)
End Sub
"#,
        );

        assert_eq!(output, vec!["Hello, Valo 42"]);
    }

    #[test]
    fn runs_control_flow() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    Dim total As Integer
    i = 1
    total = 0
    While i <= 5
        total = total + i
        i = i + 1
    Wend
    If total = 15 Then
        Console.WriteLine("ok")
    Else
        Console.WriteLine("bad")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["ok"]);
    }

    #[test]
    fn reports_line_and_column_for_parse_errors() {
        let error = Parser::parse_source(
            r#"
Sub Main()
    Dim x As Integer
    x = 
End Sub
"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("line 4, column"));
    }

    #[test]
    fn variable_names_are_case_insensitive() {
        let output = run_source(
            r#"
Sub Main()
    Dim Name As String
    name = "Valo"
    Console.WriteLine(NAME)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn runs_nested_if_and_while_blocks() {
        let output = run_source(
            r#"
Sub Main()
    Dim outer As Integer
    Dim inner As Integer
    outer = 0

    While outer < 2
        inner = 0
        While inner < 2
            If outer = 1 Then
                Console.WriteLine("outer " & outer & ", inner " & inner)
            Else
                Console.WriteLine("skip")
            End If
            inner = inner + 1
        Wend
        outer = outer + 1
    Wend
End Sub
"#,
        );

        assert_eq!(
            output,
            vec!["skip", "skip", "outer 1, inner 0", "outer 1, inner 1"]
        );
    }

    #[test]
    fn reports_division_by_zero() {
        let error = source_error(
            r#"
Sub Main()
    Dim x As Integer
    x = 1 / 0
End Sub
"#,
        );

        assert!(error.contains("Division by zero"));
        assert!(error.contains("line 4, column"));
    }

    #[test]
    fn reports_undefined_variables() {
        let error = source_error(
            r#"
Sub Main()
    missing = 1
End Sub
"#,
        );

        assert!(error.contains("Variable 'missing' is not declared"));
    }

    #[test]
    fn reports_type_mismatch_errors() {
        let error = source_error(
            r#"
Sub Main()
    Dim x As Integer
    x = "nope"
End Sub
"#,
        );

        assert!(error.contains("Cannot assign String value to Integer variable"));
        assert!(error.contains("line 4, column"));
    }

    #[test]
    fn runs_simple_ascending_for_loop() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 3
        Console.WriteLine(i)
    Next
End Sub
"#,
        );

        assert_eq!(output, vec!["1", "2", "3"]);
    }

    #[test]
    fn runs_descending_for_loop_with_negative_step() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    For i = 3 To 1 Step -1
        Console.WriteLine(i)
    Next
End Sub
"#,
        );

        assert_eq!(output, vec!["3", "2", "1"]);
    }

    #[test]
    fn reports_undeclared_for_loop_variable() {
        let error = source_error(
            r#"
Sub Main()
    For i = 1 To 3
        Console.WriteLine(i)
    Next
End Sub
"#,
        );

        assert!(error.contains("Variable 'i' is not declared"));
    }

    #[test]
    fn reports_non_integer_for_loop_variable() {
        let error = source_error(
            r#"
Sub Main()
    Dim i As String
    For i = 1 To 3
        Console.WriteLine(i)
    Next
End Sub
"#,
        );

        assert!(error.contains("For loop variable 'i' must be Integer"));
    }

    #[test]
    fn runs_simple_integer_function() {
        let output = run_source(
            r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Console.WriteLine(Add(10, 20))
End Sub
"#,
        );

        assert_eq!(output, vec!["30"]);
    }

    #[test]
    fn runs_string_returning_function() {
        let output = run_source(
            r#"
Function Greeting(ByVal name As String) As String
    Return "Hello, " & name
End Function

Sub Main()
    Console.WriteLine(Greeting("Valo"))
End Sub
"#,
        );

        assert_eq!(output, vec!["Hello, Valo"]);
    }

    #[test]
    fn uses_function_call_inside_expression() {
        let output = run_source(
            r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Console.WriteLine(Add(1, 2) + Add(3, 4))
End Sub
"#,
        );

        assert_eq!(output, vec!["10"]);
    }

    #[test]
    fn reports_wrong_function_argument_count() {
        let error = source_error(
            r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Console.WriteLine(Add(1))
End Sub
"#,
        );

        assert!(error.contains("Function 'Add' expects 2 argument(s), got 1"));
    }

    #[test]
    fn reports_unknown_function() {
        let error = source_error(
            r#"
Sub Main()
    Console.WriteLine(Missing())
End Sub
"#,
        );

        assert!(error.contains("Function 'Missing' is not defined"));
    }

    #[test]
    fn reports_duplicate_parameter() {
        let error = source_error(
            r#"
Function Bad(ByVal value As Integer, ByVal VALUE As Integer) As Integer
    Return value
End Function

Sub Main()
    Console.WriteLine(Bad(1, 2))
End Sub
"#,
        );

        assert!(error.contains("Parameter 'VALUE' is already declared"));
    }

    #[test]
    fn reports_return_outside_function() {
        let error = source_error(
            r#"
Sub Main()
    Return 1
End Sub
"#,
        );

        assert!(error.contains("Return is only allowed inside Function"));
    }

    #[test]
    fn reports_missing_return() {
        let error = source_error(
            r#"
Function MissingReturn() As Integer
    Dim x As Integer
    x = 1
End Function

Sub Main()
    Console.WriteLine(MissingReturn())
End Sub
"#,
        );

        assert!(error.contains("Function 'MissingReturn' must return a value"));
    }

    #[test]
    fn reports_type_mismatch_return() {
        let error = source_error(
            r#"
Function Bad() As Integer
    Return "nope"
End Function

Sub Main()
    Console.WriteLine(Bad())
End Sub
"#,
        );

        assert!(error.contains("Cannot assign String value to Integer variable"));
    }

    #[test]
    fn isolates_main_and_function_local_variables() {
        let output = run_source(
            r#"
Function GetValue() As Integer
    Dim value As Integer
    value = 99
    Return value
End Function

Sub Main()
    Dim value As Integer
    value = 1
    Console.WriteLine(GetValue())
    Console.WriteLine(value)
End Sub
"#,
        );

        assert_eq!(output, vec!["99", "1"]);
    }

    #[test]
    fn runs_simple_sub_call() {
        let output = run_source(
            r#"
Sub SayHello()
    Console.WriteLine("Hello")
End Sub

Sub Main()
    SayHello()
End Sub
"#,
        );

        assert_eq!(output, vec!["Hello"]);
    }

    #[test]
    fn runs_sub_call_with_byval_parameter() {
        let output = run_source(
            r#"
Sub Show(ByVal value As String)
    Console.WriteLine(value)
End Sub

Sub Main()
    Show("Valo")
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn byref_sub_parameter_mutates_caller_variable() {
        let output = run_source(
            r#"
Sub Increment(ByRef value As Integer)
    value = value + 1
End Sub

Sub Main()
    Dim x As Integer
    x = 10
    Increment(x)
    Console.WriteLine(x)
End Sub
"#,
        );

        assert_eq!(output, vec!["11"]);
    }

    #[test]
    fn reports_byref_literal_argument() {
        let error = source_error(
            r#"
Sub Increment(ByRef value As Integer)
    value = value + 1
End Sub

Sub Main()
    Increment(10)
End Sub
"#,
        );

        assert!(error.contains("ByRef argument must be a variable"));
    }

    #[test]
    fn reports_byref_expression_argument() {
        let error = source_error(
            r#"
Sub Increment(ByRef value As Integer)
    value = value + 1
End Sub

Sub Main()
    Dim x As Integer
    x = 10
    Increment(x + 1)
End Sub
"#,
        );

        assert!(error.contains("ByRef argument must be a variable"));
    }

    #[test]
    fn reports_wrong_sub_argument_count() {
        let error = source_error(
            r#"
Sub Show(ByVal value As String)
    Console.WriteLine(value)
End Sub

Sub Main()
    Show()
End Sub
"#,
        );

        assert!(error.contains("Sub 'Show' expects 1 argument(s), got 0"));
    }

    #[test]
    fn reports_unknown_sub() {
        let error = source_error(
            r#"
Sub Main()
    Missing()
End Sub
"#,
        );

        assert!(error.contains("Sub 'Missing' is not defined"));
    }

    #[test]
    fn reports_duplicate_sub_function_name_conflict() {
        let error = source_error(
            r#"
Sub Same()
End Sub

Function Same() As Integer
    Return 1
End Function

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Name 'Same' conflicts with existing Sub"));
    }

    #[test]
    fn reports_duplicate_sub_name() {
        let error = source_error(
            r#"
Sub Same()
End Sub

Sub SAME()
End Sub

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Name 'SAME' conflicts with existing Sub"));
    }

    #[test]
    fn rejects_main_with_parameters() {
        let error = source_error(
            r#"
Sub Main(ByVal value As Integer)
    Console.WriteLine(value)
End Sub
"#,
        );

        assert!(error.contains("Sub Main() cannot have parameters"));
    }

    #[test]
    fn reports_sub_used_in_expression() {
        let error = source_error(
            r#"
Sub SayHello()
    Console.WriteLine("Hello")
End Sub

Sub Main()
    Dim value As Integer
    value = SayHello()
End Sub
"#,
        );

        assert!(error.contains("Sub 'SayHello' cannot be used as an expression"));
    }

    #[test]
    fn reports_function_called_as_statement() {
        let error = source_error(
            r#"
Function Add(ByVal a As Integer, ByVal b As Integer) As Integer
    Return a + b
End Function

Sub Main()
    Add(1, 2)
End Sub
"#,
        );

        assert!(error.contains("Function 'Add' cannot be called as a statement"));
    }

    #[test]
    fn sub_calls_can_be_nested_and_call_functions() {
        let output = run_source(
            r#"
Function Label(ByVal value As Integer) As String
    Return "Value: " & value
End Function

Sub PrintLabel(ByVal value As Integer)
    Console.WriteLine(Label(value))
End Sub

Sub Outer(ByRef value As Integer)
    value = value + 1
    PrintLabel(value)
End Sub

Sub Main()
    Dim x As Integer
    x = 4
    Outer(x)
    Console.WriteLine(x)
End Sub
"#,
        );

        assert_eq!(output, vec!["Value: 5", "5"]);
    }

    #[test]
    fn declares_type_dims_variable_and_reads_default_fields() {
        let output = run_source(
            r#"
Type User
    Name As String
    Age As Integer
    Active As Boolean
End Type

Sub Main()
    Dim user As User
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Age)
    Console.WriteLine(user.Active)
End Sub
"#,
        );

        assert_eq!(output, vec!["", "0", "False"]);
    }

    #[test]
    fn assigns_and_reads_members() {
        let output = run_source(
            r#"
Type User
    Name As String
    Age As Integer
End Type

Sub Main()
    Dim user As User
    user.Name = "Valo"
    user.Age = 1
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Age)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo", "1"]);
    }

    #[test]
    fn returns_user_defined_type_from_function() {
        let output = run_source(
            r#"
Type User
    Name As String
    Age As Integer
    Active As Boolean
End Type

Function CreateUser(ByVal name As String, ByVal age As Integer) As User
    Dim u As User
    u.Name = name
    u.Age = age
    u.Active = True
    Return u
End Function

Sub Main()
    Dim user As User
    user = CreateUser("Valo", 1)
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Age)
    Console.WriteLine(user.Active)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo", "1", "True"]);
    }

    #[test]
    fn reports_unknown_type() {
        let error = source_error(
            r#"
Sub Main()
    Dim user As Missing
End Sub
"#,
        );

        assert!(error.contains("Type 'Missing' is not defined"));
    }

    #[test]
    fn reports_duplicate_type() {
        let error = source_error(
            r#"
Type User
    Name As String
End Type

Type user
    Age As Integer
End Type

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Type 'user' is already defined"));
    }

    #[test]
    fn reports_duplicate_field() {
        let error = source_error(
            r#"
Type User
    Name As String
    NAME As String
End Type

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Field 'NAME' is already declared in Type 'User'"));
    }

    #[test]
    fn reports_unknown_field() {
        let error = source_error(
            r#"
Type User
    Name As String
End Type

Sub Main()
    Dim user As User
    Console.WriteLine(user.Age)
End Sub
"#,
        );

        assert!(error.contains("Type 'User' has no field 'Age'"));
    }

    #[test]
    fn reports_field_type_mismatch() {
        let error = source_error(
            r#"
Type User
    Age As Integer
End Type

Sub Main()
    Dim user As User
    user.Age = "old"
End Sub
"#,
        );

        assert!(error.contains("Cannot assign String value to Integer variable"));
    }

    #[test]
    fn type_and_field_names_are_case_insensitive() {
        let output = run_source(
            r#"
Type User
    Name As String
End Type

Sub Main()
    Dim user As user
    USER.name = "Valo"
    Console.WriteLine(user.NAME)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn byref_type_parameter_can_mutate_field() {
        let output = run_source(
            r#"
Type User
    Name As String
    Active As Boolean
End Type

Sub Activate(ByRef user As User)
    user.Active = True
End Sub

Sub Main()
    Dim user As User
    user.Name = "Valo"
    Activate(user)
    Console.WriteLine(user.Name)
    Console.WriteLine(user.Active)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo", "True"]);
    }

    #[test]
    fn declares_fixed_integer_array() {
        let output = run_source(
            r#"
Sub Main()
    Dim numbers(3) As Integer
    Console.WriteLine(numbers(0))
    Console.WriteLine(numbers(3))
End Sub
"#,
        );

        assert_eq!(output, vec!["0", "0"]);
    }

    #[test]
    fn assigns_and_reads_array_elements() {
        let output = run_source(
            r#"
Sub Main()
    Dim numbers(3) As Integer
    numbers(0) = 10
    numbers(1) = 20
    numbers(2) = 30
    Console.WriteLine(numbers(0))
    Console.WriteLine(numbers(1))
    Console.WriteLine(numbers(2))
End Sub
"#,
        );

        assert_eq!(output, vec!["10", "20", "30"]);
    }

    #[test]
    fn supports_expression_array_index() {
        let output = run_source(
            r#"
Sub Main()
    Dim numbers(3) As Integer
    Dim i As Integer
    i = 1
    numbers(i + 1) = 42
    Console.WriteLine(numbers(2))
End Sub
"#,
        );

        assert_eq!(output, vec!["42"]);
    }

    #[test]
    fn reports_array_bounds_error() {
        let error = source_error(
            r#"
Sub Main()
    Dim numbers(1) As Integer
    Console.WriteLine(numbers(2))
End Sub
"#,
        );

        assert!(error.contains("Array index 2 is out of bounds for length 2"));
    }

    #[test]
    fn reports_scalar_used_as_array() {
        let error = source_error(
            r#"
Sub Main()
    Dim number As Integer
    Console.WriteLine(number(0))
End Sub
"#,
        );

        assert!(error.contains("Variable 'number' is not an array"));
    }

    #[test]
    fn reports_wrong_array_element_type() {
        let error = source_error(
            r#"
Sub Main()
    Dim numbers(1) As Integer
    numbers(0) = "nope"
End Sub
"#,
        );

        assert!(error.contains("Cannot assign String value to Integer variable"));
    }

    #[test]
    fn supports_array_of_user_defined_type() {
        let output = run_source(
            r#"
Type User
    Name As String
    Age As Integer
End Type

Sub Main()
    Dim users(2) As User
    users(0).Name = "Valo"
    users(0).Age = 1
    Console.WriteLine(users(0).Name)
    Console.WriteLine(users(0).Age)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo", "1"]);
    }

    #[test]
    fn reports_array_used_as_scalar() {
        let error = source_error(
            r#"
Sub Main()
    Dim numbers(1) As Integer
    Console.WriteLine(numbers)
End Sub
"#,
        );

        assert!(error.contains("Array variable 'numbers' cannot be used as a scalar"));
    }

    #[test]
    fn creates_class_instance_and_calls_constructor() {
        let output = run_source(
            r#"
Class User
    Public Name As String

    Public Sub Initialize(ByVal name As String)
        Me.Name = name
    End Sub
End Class

Sub Main()
    Dim user As User
    user = New User("Valo")
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn class_method_mutation_persists_and_assignment_is_reference_like() {
        let output = run_source(
            r#"
Class User
    Public Name As String

    Public Sub Rename(ByVal name As String)
        Me.Name = name
    End Sub
End Class

Sub Main()
    Dim a As User
    Dim b As User
    a = New User()
    b = a
    b.Rename("Changed")
    Console.WriteLine(a.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Changed"]);
    }

    #[test]
    fn class_function_method_returns_value() {
        let output = run_source(
            r#"
Class User
    Private Age As Integer

    Public Sub Initialize(ByVal age As Integer)
        Me.Age = age
    End Sub

    Public Function IsAdult() As Boolean
        Return Me.Age >= 18
    End Function
End Class

Sub Main()
    Dim user As User
    user = New User(20)
    Console.WriteLine(user.IsAdult())
End Sub
"#,
        );

        assert_eq!(output, vec!["True"]);
    }

    #[test]
    fn private_field_access_outside_class_is_rejected() {
        let error = source_error(
            r#"
Class User
    Private Age As Integer
End Class

Sub Main()
    Dim user As User
    user = New User()
    Console.WriteLine(user.Age)
End Sub
"#,
        );

        assert!(error.contains("Member 'Age' is Private in Class 'User'"));
    }

    #[test]
    fn private_method_call_through_me_is_allowed() {
        let output = run_source(
            r#"
Class User
    Private Active As Boolean

    Private Sub SetActive(ByVal value As Boolean)
        Me.Active = value
    End Sub

    Public Sub Activate()
        Me.SetActive(True)
    End Sub

    Public Function IsActive() As Boolean
        Return Me.Active
    End Function
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Activate()
    Console.WriteLine(user.IsActive())
End Sub
"#,
        );

        assert_eq!(output, vec!["True"]);
    }

    #[test]
    fn private_method_call_outside_class_is_rejected() {
        let error = source_error(
            r#"
Class User
    Private Sub Hide()
    End Sub
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Hide()
End Sub
"#,
        );

        assert!(error.contains("Member 'Hide' is Private in Class 'User'"));
    }

    #[test]
    fn me_outside_class_is_rejected() {
        let error = source_error(
            r#"
Sub Main()
    Console.WriteLine(Me)
End Sub
"#,
        );

        assert!(error.contains("Me is only valid inside class methods"));
    }

    #[test]
    fn type_record_assignment_remains_value_copy() {
        let output = run_source(
            r#"
Type User
    Name As String
End Type

Sub Main()
    Dim a As User
    Dim b As User
    a.Name = "Original"
    b = a
    b.Name = "Changed"
    Console.WriteLine(a.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Original"]);
    }

    #[test]
    fn evaluates_and_behavior() {
        let output = run_source(
            r#"
Sub Main()
    Console.WriteLine(True And True)
    Console.WriteLine(True And False)
End Sub
"#,
        );

        assert_eq!(output, vec!["True", "False"]);
    }

    #[test]
    fn evaluates_or_behavior() {
        let output = run_source(
            r#"
Sub Main()
    Console.WriteLine(False Or True)
    Console.WriteLine(False Or False)
End Sub
"#,
        );

        assert_eq!(output, vec!["True", "False"]);
    }

    #[test]
    fn evaluates_not_behavior() {
        let output = run_source(
            r#"
Sub Main()
    Console.WriteLine(Not True)
    Console.WriteLine(Not False)
End Sub
"#,
        );

        assert_eq!(output, vec!["False", "True"]);
    }

    #[test]
    fn evaluates_mod_result() {
        let output = run_source(
            r#"
Sub Main()
    Console.WriteLine(10 Mod 3)
End Sub
"#,
        );

        assert_eq!(output, vec!["1"]);
    }

    #[test]
    fn reports_mod_by_zero() {
        let error = source_error(
            r#"
Sub Main()
    Console.WriteLine(10 Mod 0)
End Sub
"#,
        );

        assert!(error.contains("Modulo by zero"));
    }

    #[test]
    fn elseif_uses_first_matching_branch() {
        let output = run_source(
            r#"
Sub Main()
    Dim age As Integer
    Dim active As Boolean
    age = 20
    active = False

    If age < 18 Then
        Console.WriteLine("Denied")
    ElseIf age >= 18 And active Then
        Console.WriteLine("Allowed")
    ElseIf age >= 18 Then
        Console.WriteLine("Inactive")
    Else
        Console.WriteLine("Other")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["Inactive"]);
    }

    #[test]
    fn elseif_falls_through_to_else() {
        let output = run_source(
            r#"
Sub Main()
    Dim age As Integer
    age = 12

    If age > 20 Then
        Console.WriteLine("adult")
    ElseIf age = 18 Then
        Console.WriteLine("exact")
    Else
        Console.WriteLine("minor")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["minor"]);
    }

    #[test]
    fn logical_operator_precedence() {
        let output = run_source(
            r#"
Sub Main()
    Console.WriteLine(True Or False And False)
    Console.WriteLine(Not False And False)
    Console.WriteLine(Not (False And False))
End Sub
"#,
        );

        assert_eq!(output, vec!["True", "False", "True"]);
    }

    #[test]
    fn existing_if_without_elseif_still_works() {
        let output = run_source(
            r#"
Sub Main()
    If True Then
        Console.WriteLine("ok")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["ok"]);
    }

    #[test]
    fn property_read_calls_get() {
        let output = run_source(
            r#"
Class User
    Private mName As String

    Public Property Get Name() As String
        Return Me.mName & "!"
    End Property

    Public Sub SetName(ByVal value As String)
        Me.mName = value
    End Sub
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.SetName("Valo")
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo!"]);
    }

    #[test]
    fn property_assignment_calls_let() {
        let output = run_source(
            r#"
Class User
    Private mName As String

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value & " Runtime"
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Name = "Valo"
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo Runtime"]);
    }

    #[test]
    fn property_validation_logic_mutates_backing_field() {
        let output = run_source(
            r#"
Class User
    Private mAge As Integer

    Public Property Get Age() As Integer
        Return Me.mAge
    End Property

    Public Property Let Age(ByVal value As Integer)
        If value < 0 Then
            Me.mAge = 0
        Else
            Me.mAge = value
        End If
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Age = -1
    Console.WriteLine(user.Age)
End Sub
"#,
        );

        assert_eq!(output, vec!["0"]);
    }

    #[test]
    fn private_property_access_outside_class_is_rejected() {
        let error = source_error(
            r#"
Class User
    Private mName As String

    Private Property Get Name() As String
        Return Me.mName
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert!(error.contains("Member 'Name' is Private in Class 'User'"));
    }

    #[test]
    fn private_property_access_inside_class_is_allowed() {
        let output = run_source(
            r#"
Class User
    Private mName As String

    Private Property Get Name() As String
        Return Me.mName
    End Property

    Private Property Let Name(ByVal value As String)
        Me.mName = value
    End Property

    Public Sub Rename(ByVal value As String)
        Me.Name = value
    End Sub

    Public Function Label() As String
        Return Me.Name
    End Function
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Rename("Valo")
    Console.WriteLine(user.Label())
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn missing_get_when_reading_property_produces_error() {
        let error = source_error(
            r#"
Class User
    Public Property Let Name(ByVal value As String)
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert!(error.contains("Property 'Name' has no Get accessor"));
    }

    #[test]
    fn missing_let_or_set_when_assigning_property_produces_error() {
        let error = source_error(
            r#"
Class User
    Public Property Get Name() As String
        Return "Valo"
    End Property
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Name = "Runtime"
End Sub
"#,
        );

        assert!(error.contains("Property 'Name' has no Let or Set accessor"));
    }

    #[test]
    fn duplicate_property_get_is_rejected() {
        let error = source_error(
            r#"
Class User
    Public Property Get Name() As String
        Return "a"
    End Property

    Public Property Get Name() As String
        Return "b"
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property Get 'Name' is already declared"));
    }

    #[test]
    fn duplicate_property_let_is_rejected() {
        let error = source_error(
            r#"
Class User
    Public Property Let Name(ByVal value As String)
    End Property

    Public Property Let Name(ByVal value As String)
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property Let 'Name' is already declared"));
    }

    #[test]
    fn property_conflicts_with_field_name() {
        let error = source_error(
            r#"
Class User
    Public Name As String

    Public Property Get Name() As String
        Return "Valo"
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property 'Name' conflicts with another member"));
    }

    #[test]
    fn property_conflicts_with_method_name() {
        let error = source_error(
            r#"
Class User
    Public Sub Name()
    End Sub

    Public Property Get Name() As String
        Return "Valo"
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property 'Name' conflicts with another member"));
    }

    #[test]
    fn property_get_missing_return_is_rejected() {
        let error = source_error(
            r#"
Class User
    Public Property Get Name() As String
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property Get 'Name' must return a value"));
    }

    #[test]
    fn property_let_with_wrong_parameter_count_is_rejected() {
        let error = source_error(
            r#"
Class User
    Public Property Let Name()
    End Property
End Class

Sub Main()
End Sub
"#,
        );

        assert!(error.contains("Property Let 'Name' must have exactly one parameter"));
    }

    #[test]
    fn property_set_assigns_object_reference() {
        let output = run_source(
            r#"
Class Owner
    Public Name As String
End Class

Class Item
    Private mOwner As Owner

    Public Property Get Owner() As Owner
        Return Me.mOwner
    End Property

    Public Property Set Owner(ByVal value As Owner)
        Me.mOwner = value
    End Property
End Class

Sub Main()
    Dim owner As Owner
    Dim item As Item
    owner = New Owner()
    owner.Name = "Valo"
    item = New Item()
    item.Owner = owner
    Console.WriteLine(item.Owner.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn set_object_assignment_works() {
        let output = run_source(
            r#"
Class User
    Public Name As String

    Public Sub Initialize(ByVal name As String)
        Me.Name = name
    End Sub
End Class

Sub Main()
    Dim user As User
    Set user = New User("Valo")
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn set_me_field_assignment_works() {
        let output = run_source(
            r#"
Class Room
    Public North As String
End Class

Class Game
    Private mHall As Room

    Public Sub Initialize()
        Set Me.mHall = New Room()
        Me.mHall.North = "library"
    End Sub

    Public Function NorthExit() As String
        Return Me.mHall.North
    End Function
End Class

Sub Main()
    Dim game As Game
    game = New Game()
    Console.WriteLine(game.NorthExit())
End Sub
"#,
        );

        assert_eq!(output, vec!["library"]);
    }

    #[test]
    fn set_object_field_and_nested_member_assignment_work() {
        let output = run_source(
            r#"
Class Child
    Public Name As String
End Class

Class Holder
    Public Child As Child
End Class

Sub Main()
    Dim holder As Holder
    holder = New Holder()
    Set holder.Child = New Child()
    holder.Child.Name = "Valo"
    Console.WriteLine(holder.Child.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn set_object_property_dispatches_property_set() {
        let output = run_source(
            r#"
Class Child
    Public Name As String
End Class

Class Holder
    Private mChild As Child

    Public Property Get Child() As Child
        Return Me.mChild
    End Property

    Public Property Set Child(ByVal value As Child)
        Me.mChild = value
        Me.mChild.Name = "set"
    End Property
End Class

Sub Main()
    Dim holder As Holder
    holder = New Holder()
    Set holder.Child = New Child()
    holder.Child.Name = holder.Child.Name & " ok"
    Console.WriteLine(holder.Child.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["set ok"]);
    }

    #[test]
    fn nested_object_field_chain_assignment_works() {
        let output = run_source(
            r#"
Class Inner
    Public Value As String
End Class

Class Child
    Public Inner As Inner
End Class

Class Holder
    Public Child As Child
End Class

Sub Main()
    Dim holder As Holder
    holder = New Holder()
    holder.Child = New Child()
    holder.Child.Inner = New Inner()
    holder.Child.Inner.Value = "deep"
    Console.WriteLine(holder.Child.Inner.Value)
End Sub
"#,
        );

        assert_eq!(output, vec!["deep"]);
    }

    #[test]
    fn chained_assignment_reports_nothing_intermediate() {
        let error = source_error(
            r#"
Class Child
    Public Name As String
End Class

Class Holder
    Public Child As Child
End Class

Sub Main()
    Dim holder As Holder
    holder = New Holder()
    holder.Child.Name = "Valo"
End Sub
"#,
        );

        assert!(error.contains("Object reference is Nothing"));
    }

    #[test]
    fn not_is_precedence_matches_vba_style() {
        let output = run_source(
            r#"
Class User
    Public Name As String
End Class

Sub Main()
    Dim user As User
    user = New User()

    If Not user Is Nothing Then
        user.Name = "Valo"
    End If

    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn not_is_other_object_precedence_works() {
        let output = run_source(
            r#"
Class User
End Class

Sub Main()
    Dim user As User
    Dim otherUser As User
    user = New User()
    otherUser = New User()

    If Not user Is otherUser Then
        Console.WriteLine("different")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["different"]);
    }

    #[test]
    fn parenthesized_not_is_still_works() {
        let output = run_source(
            r#"
Class User
End Class

Sub Main()
    Dim user As User
    user = New User()

    If Not (user Is Nothing) Then
        Console.WriteLine("not nothing")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["not nothing"]);
    }

    #[test]
    fn set_rejects_boolean_and_type_record_targets() {
        let boolean_error = source_error(
            r#"
Sub Main()
    Dim active As Boolean
    Set active = Nothing
End Sub
"#,
        );
        assert!(boolean_error.contains("Set target must be a class type"));

        let record_error = source_error(
            r#"
Type Point
    X As Integer
End Type

Sub Main()
    Dim point As Point
    Set point = Nothing
End Sub
"#,
        );
        assert!(record_error.contains("Set target must be a class type"));
    }

    #[test]
    fn normal_object_assignment_still_works() {
        let output = run_source(
            r#"
Class User
    Public Name As String
End Class

Sub Main()
    Dim user As User
    Dim aliasUser As User
    user = New User()
    user.Name = "Valo"
    aliasUser = user
    Console.WriteLine(aliasUser.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo"]);
    }

    #[test]
    fn class_variable_defaults_to_nothing() {
        let output = run_source(
            r#"
Class User
End Class

Sub Main()
    Dim user As User
    If user Is Nothing Then
        Console.WriteLine("empty")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["empty"]);
    }

    #[test]
    fn field_access_on_nothing_errors_clearly() {
        let error = source_error(
            r#"
Class User
    Public Name As String
End Class

Sub Main()
    Dim user As User
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert!(error.contains("Object reference is Nothing"));
    }

    #[test]
    fn method_call_on_nothing_errors_clearly() {
        let error = source_error(
            r#"
Class User
    Public Sub Rename(ByVal value As String)
    End Sub
End Class

Sub Main()
    Dim user As User
    user.Rename("Valo")
End Sub
"#,
        );

        assert!(error.contains("Object reference is Nothing"));
    }

    #[test]
    fn set_user_to_nothing() {
        let output = run_source(
            r#"
Class User
End Class

Sub Main()
    Dim user As User
    Set user = New User()
    Set user = Nothing
    If user Is Nothing Then
        Console.WriteLine("empty")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["empty"]);
    }

    #[test]
    fn is_nothing_false_after_new() {
        let output = run_source(
            r#"
Class User
End Class

Sub Main()
    Dim user As User
    Set user = New User()
    If Not (user Is Nothing) Then
        Console.WriteLine("present")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["present"]);
    }

    #[test]
    fn object_identity_with_is() {
        let output = run_source(
            r#"
Class User
End Class

Sub Main()
    Dim user As User
    Dim aliasUser As User
    Set user = New User()
    Set aliasUser = user
    If user Is aliasUser Then
        Console.WriteLine("same")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["same"]);
    }

    #[test]
    fn set_rejected_for_integer() {
        let error = source_error(
            r#"
Sub Main()
    Dim value As Integer
    Set value = 1
End Sub
"#,
        );

        assert!(error.contains("Set target must be a class type"));
    }

    #[test]
    fn set_rejected_for_string() {
        let error = source_error(
            r#"
Sub Main()
    Dim value As String
    Set value = "Valo"
End Sub
"#,
        );

        assert!(error.contains("Set target must be a class type"));
    }

    #[test]
    fn nothing_rejected_for_builtin_values() {
        for source in [
            r#"
Sub Main()
    Dim value As Integer
    value = Nothing
End Sub
"#,
            r#"
Sub Main()
    Dim value As String
    value = Nothing
End Sub
"#,
            r#"
Sub Main()
    Dim value As Boolean
    value = Nothing
End Sub
"#,
        ] {
            let error = source_error(source);
            assert!(error.contains("Nothing requires a class object type"));
        }
    }

    #[test]
    fn nothing_rejected_for_type_records() {
        let error = source_error(
            r#"
Type User
    Name As String
End Type

Sub Main()
    Dim user As User
    user = Nothing
End Sub
"#,
        );

        assert!(error.contains("Nothing requires a class object type"));
    }

    #[test]
    fn select_case_matches_integer_case() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 2

    Select Case value
        Case 1
            Console.WriteLine("one")
        Case 2
            Console.WriteLine("two")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["two"]);
    }

    #[test]
    fn select_case_matches_string_case() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As String
    value = "b"

    Select Case value
        Case "a"
            Console.WriteLine("a")
        Case "b"
            Console.WriteLine("b")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["b"]);
    }

    #[test]
    fn select_case_supports_multiple_values() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 4

    Select Case value
        Case 1, 2
            Console.WriteLine("low")
        Case 3, 4
            Console.WriteLine("high")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["high"]);
    }

    #[test]
    fn next_without_variable_still_works() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 3
        Console.WriteLine(i)
    Next
End Sub
"#,
        );

        assert_eq!(output, vec!["1", "2", "3"]);
    }

    #[test]
    fn next_with_variable_works() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 3
        Console.WriteLine(i)
    Next i
End Sub
"#,
        );

        assert_eq!(output, vec!["1", "2", "3"]);
    }

    #[test]
    fn next_variable_is_case_insensitive() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 2
        Console.WriteLine(i)
    Next I
End Sub
"#,
        );

        assert_eq!(output, vec!["1", "2"]);
    }

    #[test]
    fn nested_next_variables_match_nearest_loop() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    Dim j As Integer
    For i = 1 To 2
        For j = 1 To 2
            Console.WriteLine(i & "," & j)
        Next j
    Next i
End Sub
"#,
        );

        assert_eq!(output, vec!["1,1", "1,2", "2,1", "2,2"]);
    }

    #[test]
    fn mismatched_next_variable_is_rejected() {
        let error = source_error(
            r#"
Sub Main()
    Dim i As Integer
    Dim j As Integer
    For i = 1 To 3
        Console.WriteLine(i)
    Next j
End Sub
"#,
        );

        assert!(error.contains("Next variable 'j' does not match For variable 'i'"));
    }

    #[test]
    fn select_case_integer_range_matches() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 4
    Select Case value
        Case 1 To 5
            Console.WriteLine("small")
        Case Else
            Console.WriteLine("other")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["small"]);
    }

    #[test]
    fn select_case_integer_range_falls_through() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 8
    Select Case value
        Case 1 To 5
            Console.WriteLine("small")
        Case Else
            Console.WriteLine("other")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["other"]);
    }

    #[test]
    fn select_case_string_range_matches() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As String
    value = "m"
    Select Case value
        Case "a" To "z"
            Console.WriteLine("letter")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["letter"]);
    }

    #[test]
    fn select_case_mixes_values_and_ranges() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 7
    Select Case value
        Case 1, 5 To 8
            Console.WriteLine("match")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["match"]);
    }

    #[test]
    fn select_case_is_comparisons_work() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 11
    Select Case value
        Case Is < 0
            Console.WriteLine("negative")
        Case Is >= 10
            Console.WriteLine("large")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["large"]);
    }

    #[test]
    fn select_case_is_all_operators_work() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 5
    Select Case value
        Case Is > 10
            Console.WriteLine("gt")
        Case Is <= 4
            Console.WriteLine("lte")
        Case Is <> 5
            Console.WriteLine("ne")
        Case Is = 5
            Console.WriteLine("eq")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["eq"]);
    }

    #[test]
    fn select_case_is_with_strings_works() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As String
    value = "m"
    Select Case value
        Case Is > "z"
            Console.WriteLine("after")
        Case Is <= "m"
            Console.WriteLine("up to m")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["up to m"]);
    }

    #[test]
    fn malformed_case_is_has_readable_diagnostic() {
        let error = Parser::parse_source(
            r#"
Sub Main()
    Dim value As Integer
    Select Case value
        Case Is
            Console.WriteLine("bad")
    End Select
End Sub
"#,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Expected comparison operator after 'Case Is'"));
    }

    #[test]
    fn case_colon_single_line_body_works() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 1
    Select Case value
        Case 1: Console.WriteLine("one")
        Case Else: Console.WriteLine("other")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["one"]);
    }

    #[test]
    fn case_else_colon_single_line_body_works() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 9
    Select Case value
        Case 1: Console.WriteLine("one")
        Case Else: Console.WriteLine("other")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["other"]);
    }

    #[test]
    fn case_range_with_colon_works() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 4
    Select Case value
        Case 1 To 5: Console.WriteLine("small")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["small"]);
    }

    #[test]
    fn case_is_with_colon_works() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 11
    Select Case value
        Case Is > 10: Console.WriteLine("large")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["large"]);
    }

    #[test]
    fn colon_outside_case_is_not_supported() {
        let error = Parser::parse_source(
            r#"
Sub Main()
    Console.WriteLine("a"): Console.WriteLine("b")
End Sub
"#,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Expected newline after statement"));
    }

    #[test]
    fn select_case_else_fallback() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 9

    Select Case value
        Case 1
            Console.WriteLine("one")
        Case Else
            Console.WriteLine("other")
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["other"]);
    }

    #[test]
    fn select_case_no_match_without_else_does_nothing() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 9

    Select Case value
        Case 1
            Console.WriteLine("one")
    End Select
    Console.WriteLine("done")
End Sub
"#,
        );

        assert_eq!(output, vec!["done"]);
    }

    #[test]
    fn select_case_else_not_last_is_rejected() {
        let error = source_error(
            r#"
Sub Main()
    Dim value As Integer
    value = 1

    Select Case value
        Case Else
            Console.WriteLine("other")
        Case 1
            Console.WriteLine("one")
    End Select
End Sub
"#,
        );

        assert!(error.contains("Case Else must be last"));
    }

    #[test]
    fn nested_select_case_works() {
        let output = run_source(
            r#"
Sub Main()
    Dim outer As Integer
    Dim inner As Integer
    outer = 1
    inner = 2

    Select Case outer
        Case 1
            Select Case inner
                Case 2
                    Console.WriteLine("nested")
            End Select
    End Select
End Sub
"#,
        );

        assert_eq!(output, vec!["nested"]);
    }

    #[test]
    fn return_inside_select_case_inside_function_works() {
        let output = run_source(
            r#"
Function Label(ByVal value As Integer) As String
    Select Case value
        Case 1
            Return "one"
        Case Else
            Return "other"
    End Select
End Function

Sub Main()
    Console.WriteLine(Label(1))
End Sub
"#,
        );

        assert_eq!(output, vec!["one"]);
    }

    #[test]
    fn do_while_runs() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    i = 0
    Do While i < 3
        Console.WriteLine(i)
        i = i + 1
    Loop
End Sub
"#,
        );

        assert_eq!(output, vec!["0", "1", "2"]);
    }

    #[test]
    fn do_until_runs() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    i = 0
    Do Until i = 3
        Console.WriteLine(i)
        i = i + 1
    Loop
End Sub
"#,
        );

        assert_eq!(output, vec!["0", "1", "2"]);
    }

    #[test]
    fn loop_while_runs_body_before_condition() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    i = 0
    Do
        Console.WriteLine(i)
        i = i + 1
    Loop While i < 3
End Sub
"#,
        );

        assert_eq!(output, vec!["0", "1", "2"]);
    }

    #[test]
    fn loop_until_runs_body_before_condition() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    i = 0
    Do
        Console.WriteLine(i)
        i = i + 1
    Loop Until i = 3
End Sub
"#,
        );

        assert_eq!(output, vec!["0", "1", "2"]);
    }

    #[test]
    fn do_loop_with_exit_do_breaks() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    i = 0
    Do
        If i = 3 Then
            Exit Do
        End If
        Console.WriteLine(i)
        i = i + 1
    Loop
End Sub
"#,
        );

        assert_eq!(output, vec!["0", "1", "2"]);
    }

    #[test]
    fn nested_do_loops_exit_nearest_loop() {
        let output = run_source(
            r#"
Sub Main()
    Dim outer As Integer
    Dim inner As Integer
    outer = 0
    Do While outer < 2
        inner = 0
        Do
            Exit Do
            Console.WriteLine("inner")
        Loop
        Console.WriteLine(outer)
        outer = outer + 1
    Loop
End Sub
"#,
        );

        assert_eq!(output, vec!["0", "1"]);
    }

    #[test]
    fn missing_loop_reports_readable_error() {
        let error = Parser::parse_source(
            r#"
Sub Main()
    Do While True
        Console.WriteLine("open")
End Sub
"#,
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Expected 'Loop'"));
    }

    #[test]
    fn do_loop_condition_type_mismatch_is_rejected() {
        let error = source_error(
            r#"
Sub Main()
    Dim i As Integer
    i = 1
    Do While i
        Exit Do
    Loop
End Sub
"#,
        );

        assert!(error.contains("Cannot assign Integer value to Boolean variable"));
    }

    #[test]
    fn exit_sub_skips_remaining_statements() {
        let output = run_source(
            r#"
Sub StopEarly()
    Console.WriteLine("before")
    Exit Sub
    Console.WriteLine("after")
End Sub

Sub Main()
    StopEarly()
End Sub
"#,
        );

        assert_eq!(output, vec!["before"]);
    }

    #[test]
    fn exit_for_breaks_loop() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    For i = 1 To 5
        If i = 3 Then
            Exit For
        End If
        Console.WriteLine(i)
    Next
End Sub
"#,
        );

        assert_eq!(output, vec!["1", "2"]);
    }

    #[test]
    fn exit_while_breaks_loop() {
        let output = run_source(
            r#"
Sub Main()
    Dim i As Integer
    i = 1
    While i <= 5
        If i = 3 Then
            Exit While
        End If
        Console.WriteLine(i)
        i = i + 1
    Wend
End Sub
"#,
        );

        assert_eq!(output, vec!["1", "2"]);
    }

    #[test]
    fn exit_inside_select_case_works() {
        let output = run_source(
            r#"
Sub Main()
    Dim value As Integer
    value = 1
    Select Case value
        Case 1
            Exit Sub
    End Select
    Console.WriteLine("after")
End Sub
"#,
        );

        assert_eq!(output, Vec::<String>::new());
    }

    #[test]
    fn exit_for_outside_for_is_rejected() {
        let error = source_error(
            r#"
Sub Main()
    Exit For
End Sub
"#,
        );

        assert!(error.contains("Exit For is only valid inside For"));
    }

    #[test]
    fn exit_while_outside_while_is_rejected() {
        let error = source_error(
            r#"
Sub Main()
    Exit While
End Sub
"#,
        );

        assert!(error.contains("Exit While is only valid inside While"));
    }

    #[test]
    fn exit_do_outside_do_is_rejected() {
        let error = source_error(
            r#"
Sub Main()
    Exit Do
End Sub
"#,
        );

        assert!(error.contains("Exit Do is only valid inside Do"));
    }

    #[test]
    fn exit_sub_outside_sub_is_rejected() {
        let error = source_error(
            r#"
Function Value() As Integer
    Exit Sub
    Return 1
End Function

Sub Main()
    Console.WriteLine(Value())
End Sub
"#,
        );

        assert!(error.contains("Exit Sub is only valid inside Sub"));
    }

    #[test]
    fn exit_function_in_sub_is_rejected() {
        let error = source_error(
            r#"
Sub Main()
    Exit Function
End Sub
"#,
        );

        assert!(error.contains("Exit Function is only valid inside Function"));
    }

    #[test]
    fn exit_function_in_integer_function_returns_zero() {
        let output = run_source(
            r#"
Function Value() As Integer
    Exit Function
    Return 1
End Function

Sub Main()
    Console.WriteLine(Value())
End Sub
"#,
        );

        assert_eq!(output, vec!["0"]);
    }

    #[test]
    fn exit_function_in_string_function_returns_empty_string() {
        let output = run_source(
            r#"
Function Value() As String
    Exit Function
    Return "after"
End Function

Sub Main()
    Console.WriteLine("value:" & Value())
End Sub
"#,
        );

        assert_eq!(output, vec!["value:"]);
    }

    #[test]
    fn exit_function_in_boolean_function_returns_false() {
        let output = run_source(
            r#"
Function Value() As Boolean
    Exit Function
    Return True
End Function

Sub Main()
    Console.WriteLine(Value())
End Sub
"#,
        );

        assert_eq!(output, vec!["False"]);
    }

    #[test]
    fn exit_function_in_object_function_returns_nothing() {
        let output = run_source(
            r#"
Class User
End Class

Function Value() As User
    Exit Function
    Return New User()
End Function

Sub Main()
    If Value() Is Nothing Then
        Console.WriteLine("nothing")
    End If
End Sub
"#,
        );

        assert_eq!(output, vec!["nothing"]);
    }

    #[test]
    fn return_expression_overrides_exit_function_default() {
        let output = run_source(
            r#"
Function Value() As Integer
    Return 42
    Exit Function
End Function

Sub Main()
    Console.WriteLine(Value())
End Sub
"#,
        );

        assert_eq!(output, vec!["42"]);
    }

    #[test]
    fn nested_loops_exit_nearest_matching_loop() {
        let output = run_source(
            r#"
Sub Main()
    Dim outer As Integer
    Dim inner As Integer
    For outer = 1 To 2
        For inner = 1 To 3
            If inner = 2 Then
                Exit For
            End If
            Console.WriteLine(outer & ":" & inner)
        Next
    Next
End Sub
"#,
        );

        assert_eq!(output, vec!["1:1", "2:1"]);
    }

    #[test]
    fn enum_values_support_implicit_explicit_qualified_and_select_case() {
        let output = run_source(
            r#"
Public Enum DaysOfWeek
    Monday
    Tuesday
    Wednesday
End Enum

Public Enum FilePermissions
    Read = 1
    Write = 2
    Execute = 4
    All = Read + Write + Execute
End Enum

Public Enum WindowState
    [_First] = 1
    Normal = 1
    Minimized = 2
    Maximized = 3
    [_Last] = 3
End Enum

Sub Main()
    Dim day As DaysOfWeek
    day = Wednesday
    Select Case day
        Case Monday
            Console.WriteLine("Monday")
        Case Wednesday
            Console.WriteLine("Wednesday")
    End Select

    Dim access As FilePermissions
    access = All
    If (access And Write) = Write Then
        Console.WriteLine("Write access")
    End If

    Dim i As Integer
    For i = WindowState.[_First] To WindowState.[_Last]
        Console.WriteLine(i)
    Next i
End Sub
"#,
        );

        assert_eq!(output, vec!["Wednesday", "Write access", "1", "2", "3"]);
    }

    #[test]
    fn enum_auto_increment_after_explicit_value() {
        let output = run_source(
            r#"
Enum Numbers
    Zero
    Five = 5
    Six
End Enum

Sub Main()
    Console.WriteLine(Zero)
    Console.WriteLine(Six)
End Sub
"#,
        );

        assert_eq!(output, vec!["0", "6"]);
    }

    #[test]
    fn rejects_duplicate_and_unknown_enum_members() {
        let duplicate = source_error(
            r#"
Enum Bad
    One
    one
End Enum

Sub Main()
End Sub
"#,
        );
        assert!(duplicate.contains("Enum member 'one' is already declared"));

        let unknown = source_error(
            r#"
Enum Bad
    Two = One + 1
End Enum

Sub Main()
End Sub
"#,
        );
        assert!(unknown.contains("Enum member 'One' is not defined"));
    }

    #[test]
    fn dynamic_arrays_redim_bounds_and_for_each_work() {
        let output = run_source(
            r#"
Sub Main()
    Dim values() As Integer
    ReDim values(2)
    values(0) = 10
    values(1) = 20
    values(2) = 30
    Console.WriteLine(LBound(values))
    Console.WriteLine(UBound(values))

    ReDim Preserve values(4)
    values(3) = 40
    values(4) = 50
    ReDim Preserve values(1)
    Console.WriteLine(UBound(values))

    Dim item As Variant
    For Each item In values
        Console.WriteLine(item)
    Next item
End Sub
"#,
        );

        assert_eq!(output, vec!["0", "2", "1", "10", "20"]);
    }

    #[test]
    fn redim_without_preserve_discards_contents() {
        let output = run_source(
            r#"
Sub Main()
    Dim values() As Integer
    ReDim values(1)
    values(0) = 99
    ReDim values(1)
    Console.WriteLine(values(0))
End Sub
"#,
        );

        assert_eq!(output, vec!["0"]);
    }

    #[test]
    fn dynamic_arrays_support_class_and_type_defaults() {
        let output = run_source(
            r#"
Class User
    Public Name As String
End Class

Type Point
    X As Integer
End Type

Sub Main()
    Dim users() As User
    ReDim users(0)
    If users(0) Is Nothing Then
        Console.WriteLine("nothing")
    End If

    Dim points() As Point
    ReDim points(0)
    Console.WriteLine(points(0).X)
End Sub
"#,
        );

        assert_eq!(output, vec!["nothing", "0"]);
    }

    #[test]
    fn dynamic_array_errors_are_clear() {
        let unallocated = source_error(
            r#"
Sub Main()
    Dim values() As Integer
    Console.WriteLine(values(0))
End Sub
"#,
        );
        assert!(unallocated.contains("Dynamic array is unallocated"));

        let negative = source_error(
            r#"
Sub Main()
    Dim values() As Integer
    ReDim values(-1)
End Sub
"#,
        );
        assert!(negative.contains("ReDim upper bound must be non-negative"));

        let fixed = source_error(
            r#"
Sub Main()
    Dim values(1) As Integer
    ReDim values(2)
End Sub
"#,
        );
        assert!(fixed.contains("ReDim target must be a dynamic array"));
    }

    #[test]
    fn lbound_ubound_reject_unallocated_scalar_and_wrong_count() {
        let unallocated = source_error(
            r#"
Sub Main()
    Dim values() As Integer
    Console.WriteLine(UBound(values))
End Sub
"#,
        );
        assert!(unallocated.contains("Dynamic array is unallocated"));

        let scalar = source_error(
            r#"
Sub Main()
    Dim value As Integer
    Console.WriteLine(LBound(value))
End Sub
"#,
        );
        assert!(scalar.contains("Variable 'value' is not an array"));

        let wrong_count = source_error(
            r#"
Sub Main()
    Dim values(1) As Integer
    Console.WriteLine(UBound(values, 1))
End Sub
"#,
        );
        assert!(wrong_count.contains("UBound expects exactly one argument"));
    }

    #[test]
    fn for_each_supports_fixed_arrays_exit_for_nested_and_next_validation() {
        let output = run_source(
            r#"
Sub Main()
    Dim values(2) As Integer
    values(0) = 1
    values(1) = 2
    values(2) = 3

    Dim item As Integer
    For Each item In values
        If item = 3 Then
            Exit For
        End If
        Console.WriteLine(item)
    Next item

    Dim other As Integer
    For Each item In values
        For Each other In values
            If other = 2 Then
                Exit For
            End If
            Console.WriteLine(item & ":" & other)
        Next other
    Next item
End Sub
"#,
        );

        assert_eq!(output, vec!["1", "2", "1:1", "2:1", "3:1"]);

        let mismatch = source_error(
            r#"
Sub Main()
    Dim values(1) As Integer
    Dim item As Integer
    For Each item In values
    Next other
End Sub
"#,
        );
        assert!(mismatch.contains("Next variable 'other' does not match For Each variable 'item'"));
    }

    #[test]
    fn with_blocks_support_members_methods_nesting_and_control_flow() {
        let output = run_source(
            r#"
Class Profile
    Public Name As String
End Class

Class User
    Private mName As String
    Public Profile As Profile

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value
    End Property

    Public Sub Activate()
        Me.mName = Me.mName & "!"
    End Sub

    Public Function Label() As String
        With Me
            Return .Name
        End With
        Return "bad"
    End Function

    Public Sub StopEarly()
        With Me
            Exit Sub
        End With
        Me.mName = "bad"
    End Sub
End Class

Sub Main()
    Dim user As User
    user = New User()
    user.Profile = New Profile()
    With user
        .Name = "Valo"
        Call .Activate()
        .Profile.Name = .Name
        With .Profile
            .Name = .Name & " Runtime"
        End With
        Console.WriteLine(.Profile.Name)
    End With
    Console.WriteLine(user.Label())
    user.StopEarly()
    Console.WriteLine(user.Name)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo! Runtime", "Valo!", "Valo!"]);
    }

    #[test]
    fn with_reports_dot_outside_nothing_and_evaluates_target_once() {
        let dot_error = source_error(
            r#"
Sub Main()
    Console.WriteLine(.Name)
End Sub
"#,
        );
        assert!(dot_error.contains("Dot member access requires an active With block"));

        let nothing_error = source_error(
            r#"
Class User
    Public Name As String
End Class

Sub Main()
    Dim user As User
    With user
        .Name = "Valo"
    End With
End Sub
"#,
        );
        assert!(nothing_error.contains("Object reference is Nothing"));

        let output = run_source(
            r#"
Private calls As Integer

Class User
    Public Name As String
End Class

Function MakeUser() As User
    calls = calls + 1
    Return New User()
End Function

Sub Main()
    With MakeUser()
        .Name = "Valo"
        Console.WriteLine(.Name)
    End With
    Console.WriteLine(calls)
End Sub
"#,
        );

        assert_eq!(output, vec!["Valo", "1"]);
    }

    #[test]
    fn const_declarations_are_immutable_and_work_in_expressions() {
        let output = run_source(
            r#"
Public Const AppName As String = "Valo"
Private Const MaxRetries As Integer = 3
Const DebugMode As Boolean = True

Sub Main()
    Const Local As Integer = MaxRetries + 2
    Console.WriteLine(AppName & " " & Local)
    If DebugMode Then
        Console.WriteLine("debug")
    End If
    Dim i As Integer
    For i = 1 To MaxRetries
    Next i
    Select Case Local
        Case 5
            Console.WriteLine("five")
    End Select
End Sub
"#,
        );
        assert_eq!(output, vec!["Valo 5", "debug", "five"]);

        let assign_error = source_error(
            r#"
Const MaxRetries As Integer = 3
Sub Main()
    MaxRetries = 4
End Sub
"#,
        );
        assert!(assign_error.contains("Constant 'MaxRetries' cannot be assigned"));

        let duplicate_error = source_error(
            r#"
Const Name As String = "a"
Const name As String = "b"
Sub Main()
End Sub
"#,
        );
        assert!(duplicate_error.contains("conflicts with existing"));

        let mismatch_error = source_error(
            r#"
Const Count As Integer = "bad"
Sub Main()
End Sub
"#,
        );
        assert!(mismatch_error.contains("Cannot assign String value to Integer variable"));

        let non_const_error = source_error(
            r#"
Function Value() As Integer
    Return 1
End Function

Const Count As Integer = Value()
Sub Main()
End Sub
"#,
        );
        assert!(non_const_error.contains("compile-time constant"));
    }

    #[test]
    fn let_and_call_statements_reuse_existing_assignment_and_sub_logic() {
        let output = run_source(
            r#"
Class User
    Private mName As String

    Public Property Get Name() As String
        Return Me.mName
    End Property

    Public Property Let Name(ByVal value As String)
        Me.mName = value
    End Property

    Private Sub Mark()
        Me.mName = Me.mName & "!"
    End Sub

    Public Sub Touch()
        Call Me.Mark()
    End Sub
End Class

Sub PrintMessage(ByVal value As String)
    Console.WriteLine(value)
End Sub

Function Bad() As Integer
    Return 1
End Function

Sub Main()
    Dim values(0) As String
    Dim user As User
    user = New User()
    Let values(0) = "Valo"
    Let user.Name = values(0)
    With user
        Let .Name = .Name & " Runtime"
        Call .Touch()
    End With
    Call PrintMessage(user.Name)
    PrintMessage("plain")
End Sub
"#,
        );
        assert_eq!(output, vec!["Valo Runtime!", "plain"]);

        let call_function = source_error(
            r#"
Function Bad() As Integer
    Return 1
End Function

Sub Main()
    Call Bad()
End Sub
"#,
        );
        assert!(call_function.contains("Function 'Bad' cannot be called as a statement"));

        let unknown = source_error(
            r#"
Sub Main()
    Call Missing()
End Sub
"#,
        );
        assert!(unknown.contains("Sub 'Missing' is not defined"));
    }

    #[test]
    fn option_explicit_is_recognized() {
        let output = run_source(
            r#"
Option Explicit

Sub Main()
    Dim x As Integer
    x = 10
    Console.WriteLine(x)
End Sub
"#,
        );
        assert_eq!(output, vec!["10"]);

        let after_decl = source_error(
            r#"
Sub Main()
End Sub
Option Explicit
"#,
        );
        assert!(after_decl.contains("Option statements must appear before declarations"));

        let duplicate = source_error(
            r#"
Option Explicit
Option Explicit
Sub Main()
End Sub
"#,
        );
        assert!(duplicate.contains("Option Explicit is already declared"));
    }

    #[test]
    fn option_base_controls_fixed_arrays_redim_lbound_ubound_and_for_each() {
        let default_output = run_source(
            r#"
Sub Main()
    Dim a(3) As Integer
    a(0) = 5
    a(3) = 8
    Console.WriteLine(LBound(a) & ":" & UBound(a) & ":" & a(0) & ":" & a(3))
End Sub
"#,
        );
        assert_eq!(default_output, vec!["0:3:5:8"]);

        let fixed_output = run_source(
            r#"
Option Base 1
Sub Main()
    Dim a(3) As Integer
    Dim item As Integer
    Dim total As Integer
    a(1) = 10
    a(3) = 30
    For Each item In a
        total = total + item
    Next item
    Console.WriteLine(LBound(a) & ":" & UBound(a) & ":" & total)
End Sub
"#,
        );
        assert_eq!(fixed_output, vec!["1:3:40"]);

        let redim_output = run_source(
            r#"
Option Base 1
Sub Main()
    Dim a() As Integer
    ReDim a(2)
    a(1) = 7
    a(2) = 9
    Console.WriteLine(LBound(a) & ":" & UBound(a) & ":" & a(1) & ":" & a(2))
End Sub
"#,
        );
        assert_eq!(redim_output, vec!["1:2:7:9"]);
    }

    #[test]
    fn option_base_rejects_invalid_duplicate_and_late_declarations() {
        let invalid = source_error(
            r#"
Option Base 2
Sub Main()
End Sub
"#,
        );
        assert!(invalid.contains("Option Base must be 0 or 1"));

        let duplicate = source_error(
            r#"
Option Base 0
Option Base 1
Sub Main()
End Sub
"#,
        );
        assert!(duplicate.contains("Option Base is already declared"));

        let late = source_error(
            r#"
Sub Main()
End Sub
Option Base 1
"#,
        );
        assert!(late.contains("Option statements must appear before declarations"));
    }

    #[test]
    fn option_compare_controls_string_comparisons_and_select_case() {
        let binary = run_source(
            r#"
Sub Main()
    Console.WriteLine("a" = "A")
    Console.WriteLine("a" > "A")
End Sub
"#,
        );
        assert_eq!(binary, vec!["False", "True"]);

        let text = run_source(
            r#"
Option Compare Text
Sub Main()
    Console.WriteLine("a" = "A")
    Console.WriteLine("b" > "A")
    Select Case "B"
    Case "a" To "c"
        Console.WriteLine("range")
    Case Else
        Console.WriteLine("else")
    End Select
    Select Case "alpha"
    Case Is = "ALPHA"
        Console.WriteLine("is")
    End Select
End Sub
"#,
        );
        assert_eq!(text, vec!["True", "True", "range", "is"]);
    }

    #[test]
    fn option_compare_rejects_duplicate_unknown_and_late_declarations() {
        let duplicate = source_error(
            r#"
Option Compare Binary
Option Compare Text
Sub Main()
End Sub
"#,
        );
        assert!(duplicate.contains("Option Compare is already declared"));

        let unknown = source_error(
            r#"
Option Compare Database
Sub Main()
End Sub
"#,
        );
        assert!(unknown.contains("Option Compare must be Binary or Text"));

        let late = source_error(
            r#"
Sub Main()
End Sub
Option Compare Text
"#,
        );
        assert!(late.contains("Option statements must appear before declarations"));
    }

    #[test]
    fn conditional_compilation_selects_active_branches_and_ignores_inactive_code() {
        let output = run_source(
            r#"
#Const Enabled = True
#Const Version = 2
#Const Target = "web"
Sub Main()
#If Enabled Then
    Console.WriteLine("enabled")
#Else
    this is not valid Valo
#End If
#If Target = "native" Then
    Console.WriteLine("native")
#ElseIf Target = "web" Then
    Console.WriteLine("web")
#Else
    Console.WriteLine("unknown")
#End If
#If Version > 1 And Not (Target = "native") Then
    Console.WriteLine("expr")
#End If
End Sub
"#,
        );
        assert_eq!(output, vec!["enabled", "web", "expr"]);
    }

    #[test]
    fn conditional_compilation_supports_nested_blocks_and_else_branch() {
        let output = run_source(
            r#"
#Const Outer = True
#Const Inner = False
Sub Main()
#If Outer Then
#If Inner Then
    Console.WriteLine("inner")
#Else
    Console.WriteLine("nested else")
#End If
#Else
    Console.WriteLine("outer else")
#End If
End Sub
"#,
        );
        assert_eq!(output, vec!["nested else"]);
    }

    #[test]
    fn conditional_compilation_reports_structure_errors() {
        let missing = source_error(
            r#"
#If True Then
Sub Main()
End Sub
"#,
        );
        assert!(missing.contains("Missing '#End If'"));

        let unexpected_else = source_error(
            r#"
#Else
Sub Main()
End Sub
"#,
        );
        assert!(unexpected_else.contains("Unexpected '#Else'"));

        let unexpected_end = source_error(
            r#"
#End If
Sub Main()
End Sub
"#,
        );
        assert!(unexpected_end.contains("Unexpected '#End If'"));
    }

    #[test]
    fn module_level_state_defaults_persists_and_rejects_conflicts() {
        let output = run_source(
            r#"
Private counter As Integer
Public title As String
Private enabled As Boolean
Private values() As Integer
Private currentUser As User
Const Limit As Integer = 2

Class User
End Class

Sub Increment()
    counter = counter + 1
End Sub

Function NextValue() As Integer
    counter = counter + 1
    Return counter
End Function

Sub Main()
    Console.WriteLine(counter)
    Console.WriteLine("title:" & title)
    Console.WriteLine(enabled)
    If currentUser Is Nothing Then
        Console.WriteLine("nothing")
    End If
    Call Increment()
    Call Increment()
    Console.WriteLine(NextValue())
    ReDim values(Limit)
    values(2) = counter
    Console.WriteLine(values(2))
End Sub
"#,
        );
        assert_eq!(output, vec!["0", "title:", "False", "nothing", "3", "3"]);

        let const_assign = source_error(
            r#"
Const Limit As Integer = 2
Sub Main()
    Limit = 3
End Sub
"#,
        );
        assert!(const_assign.contains("Constant 'Limit' cannot be assigned"));

        let duplicate = source_error(
            r#"
Private counter As Integer
Private Counter As Integer
Sub Main()
End Sub
"#,
        );
        assert!(duplicate.contains("conflicts with existing"));

        let type_conflict = source_error(
            r#"
Type Point
    X As Integer
End Type
Private Point As Integer
Sub Main()
End Sub
"#,
        );
        assert!(type_conflict.contains("conflicts with existing Type"));
    }
}
