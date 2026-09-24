use std::fs;
use std::path::PathBuf;

use valo_core::{load_project, validate_project_for_check};

struct ProjectFixture(PathBuf);

impl ProjectFixture {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("valo-project-{name}-{}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, source: &str) {
        fs::write(self.0.join(name), source).unwrap();
    }

    fn check(&self) -> Result<(), String> {
        let project = load_project(self.0.join("main.valo"))
            .map_err(|(diagnostic, map)| diagnostic.render(&map))?;
        validate_project_for_check(&project)
            .map_err(|diagnostic| diagnostic.render(&project.source_map))
    }
}

impl Drop for ProjectFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn option_infer_is_owned_by_each_source_file() {
    let fixture = ProjectFixture::new("infer");
    fixture.write("main.valo", "Imports A\nImports B\nFunction Main() As Integer\nReturn AValue() + BValue() - 2\nEnd Function\n");
    fixture.write(
        "A.valo",
        "Option Infer On\nFunction AValue() As Integer\nDim X = 1\nReturn X\nEnd Function\n",
    );
    fixture.write("B.valo", "Option Infer Off\nFunction BValue() As Integer\nDim X As Integer = 1\nReturn X\nEnd Function\n");
    assert!(fixture.check().is_ok());
    fixture.write(
        "B.valo",
        "Option Infer Off\nFunction BValue() As Integer\nDim X = 1\nReturn X\nEnd Function\n",
    );
    let error = fixture.check().unwrap_err();
    assert!(error.contains("Option Infer Off"), "{error}");
    assert!(error.contains("B.valo"), "{error}");
}

#[test]
fn strict_and_explicit_options_do_not_leak_between_files_or_compilations() {
    let fixture = ProjectFixture::new("strict-explicit");
    fixture.write("main.valo", "Imports A\nImports B\nFunction Main() As Integer\nReturn AValue() + BValue() - 2\nEnd Function\n");
    fixture.write("A.valo", "Option Strict Off\nOption Explicit Off\nFunction AValue() As Integer\nX = 1.5\nReturn 1\nEnd Function\n");
    fixture.write("B.valo", "Option Strict On\nOption Explicit On\nFunction BValue() As Integer\nDim X As Integer = 1\nReturn X\nEnd Function\n");
    assert!(fixture.check().is_ok());
    fixture.write("B.valo", "Option Strict On\nOption Explicit On\nFunction BValue() As Integer\nDim X As Integer = 1.5\nReturn X\nEnd Function\n");
    let error = fixture.check().unwrap_err();
    assert!(error.contains("Option Strict"), "{error}");
    assert!(error.contains("B.valo"), "{error}");
    fixture.write("B.valo", "Option Strict On\nOption Explicit On\nFunction BValue() As Integer\nY = 1\nReturn 1\nEnd Function\n");
    let error = fixture.check().unwrap_err();
    assert!(error.contains("not declared"), "{error}");
    fixture.write("B.valo", "Option Strict On\nOption Explicit On\nFunction BValue() As Integer\nDim X As Integer = 1\nReturn X\nEnd Function\n");
    assert!(fixture.check().is_ok());
}

#[test]
fn imported_simple_type_and_call_names_report_ambiguity() {
    let fixture = ProjectFixture::new("ambiguity");
    fixture.write("A.valo", "Namespace A\nStructure Vector\nPublic X As Integer\nEnd Structure\nFunction Value() As Integer\nReturn 10\nEnd Function\nEnd Namespace\n");
    fixture.write("B.valo", "Namespace B\nStructure Vector\nPublic X As Integer\nEnd Structure\nFunction Value() As Integer\nReturn 20\nEnd Function\nEnd Namespace\n");
    fixture.write("main.valo", "Imports A\nImports B\nFunction Main() As Integer\nDim V As Vector\nReturn 0\nEnd Function\n");
    assert!(
        fixture
            .check()
            .unwrap_err()
            .contains("ambiguous between a.vector and b.vector")
    );
    fixture.write(
        "main.valo",
        "Imports A\nImports B\nFunction Main() As Integer\nReturn Value()\nEnd Function\n",
    );
    assert!(
        fixture
            .check()
            .unwrap_err()
            .contains("ambiguous between imports a and b")
    );
    fixture.write("main.valo", "Imports A\nImports B\nFunction Main() As Integer\nDim VA As A.Vector\nDim VB As B.Vector\nReturn A.Value() + B.Value() - 30\nEnd Function\n");
    assert!(fixture.check().is_ok());
}

#[test]
fn repeated_namespace_contributes_distinct_declarations_and_rejects_true_duplicates() {
    let fixture = ProjectFixture::new("namespace-merge");
    fixture.write(
        "A.valo",
        "Namespace Game.Math\nStructure First\nPublic X As Integer\nEnd Structure\nEnd Namespace\n",
    );
    fixture.write("B.valo", "Namespace Game.Math\nStructure Second\nPublic X As Integer\nEnd Structure\nEnd Namespace\n");
    fixture.write("main.valo", "Imports A\nImports B\nFunction Main() As Integer\nDim AValue As Game.Math.First\nDim BValue As Game.Math.Second\nReturn 0\nEnd Function\n");
    assert!(fixture.check().is_ok());
    fixture.write(
        "B.valo",
        "Namespace Game.Math\nStructure First\nPublic X As Integer\nEnd Structure\nEnd Namespace\n",
    );
    assert!(fixture.check().unwrap_err().contains("already declared"));
}

#[test]
fn duplicate_cross_file_callable_signature_is_rejected_but_overloads_are_distinct() {
    let fixture = ProjectFixture::new("function-overloads");
    fixture.write("A.valo", "Namespace Math\nFunction F(X As Integer) As Integer\nReturn X\nEnd Function\nEnd Namespace\n");
    fixture.write("B.valo", "Namespace Math\nFunction F(X As Double) As Integer\nReturn 1\nEnd Function\nEnd Namespace\n");
    fixture.write(
        "main.valo",
        "Imports A\nImports B\nFunction Main() As Integer\nReturn 0\nEnd Function\n",
    );
    assert!(fixture.check().is_ok());
    fixture.write("B.valo", "Namespace Math\nFunction F(X As Integer) As Integer\nReturn X\nEnd Function\nEnd Namespace\n");
    let error = fixture.check().unwrap_err();
    assert!(
        error.contains("with this signature is already declared"),
        "{error}"
    );
    assert!(error.contains("B.valo"), "{error}");
}

#[test]
fn option_compare_is_kept_by_each_interpreted_source() {
    let fixture = ProjectFixture::new("compare");
    fixture.write("main.valo", "Option Compare Text\nImports A\nImports B\nSub Main()\nConsole.WriteLine(A.Same())\nConsole.WriteLine(B.Same())\nConsole.WriteLine(\"a\" = \"A\")\nEnd Sub\n");
    fixture.write(
        "A.valo",
        "Option Compare Text\nFunction Same() As Boolean\nReturn \"a\" = \"A\"\nEnd Function\n",
    );
    fixture.write(
        "B.valo",
        "Option Compare Binary\nFunction Same() As Boolean\nReturn \"a\" = \"A\"\nEnd Function\n",
    );
    assert!(fixture.check().is_ok());
    let project = load_project(fixture.0.join("main.valo")).unwrap();
    assert_eq!(
        project.modules[1].program.option_compare,
        valo_core::OptionCompare::Text
    );
    let output = valo_core::run_file(fixture.0.join("main.valo")).unwrap();
    assert_eq!(output, vec!["True", "False", "True"]);
}
