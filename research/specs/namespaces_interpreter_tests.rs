//! Future interpreter behavior specs for namespace-qualified runtime imports.
//!
//! These are intentionally parked outside `core/src/backend/interpreter/tests.rs`
//! until namespace imports, aliases, privacy checks, and qualified construction
//! are implemented end to end.

use super::modules::{temp_project, write};
use crate::run_file;
use std::fs;

#[test]
fn class_in_namespace_different_from_module_name() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import LibFile

Sub Main()
    Dim s As New Game.Graphics.Sprite()
    Console.WriteLine("Success")
End Sub
"#,
    );
    write(
        &dir,
        "LibFile.valo",
        r#"
Namespace Game.Graphics
Public Class Sprite
End Class
End Namespace
"#,
    );

    let result = run_file(dir.join("main.valo"));
    match result {
        Ok(output) => assert_eq!(output, vec!["Success"]),
        Err(err) => panic!("Should have succeeded, but got error: {}", err),
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn namespace_import_works() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import LibFile
Import Game.Graphics

Sub Main()
    Dim s As New Sprite()
    Console.WriteLine("Success")
End Sub
"#,
    );
    write(
        &dir,
        "LibFile.valo",
        r#"
Namespace Game.Graphics
Public Class Sprite
End Class
End Namespace
"#,
    );

    let result = run_file(dir.join("main.valo"));
    match result {
        Ok(output) => assert_eq!(output, vec!["Success"]),
        Err(err) => panic!("Should have succeeded, but got error: {}", err),
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn namespace_alias_import_works() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import LibFile
Import Game.Graphics As G

Sub Main()
    Dim s As New G.Sprite()
    Console.WriteLine("Success")
End Sub
"#,
    );
    write(
        &dir,
        "LibFile.valo",
        r#"
Namespace Game.Graphics
Public Class Sprite
End Class
End Namespace
"#,
    );

    let result = run_file(dir.join("main.valo"));
    match result {
        Ok(output) => assert_eq!(output, vec!["Success"]),
        Err(err) => panic!("Should have succeeded, but got error: {}", err),
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn private_namespace_member_rejected() {
    let dir = temp_project();
    write(
        &dir,
        "main.valo",
        r#"
Import LibFile

Sub Main()
    Dim s As New Game.Graphics.PrivateSprite()
End Sub
"#,
    );
    write(
        &dir,
        "LibFile.valo",
        r#"
Namespace Game.Graphics
Private Class PrivateSprite
End Class
End Namespace
"#,
    );

    let result = run_file(dir.join("main.valo"));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("is Private"));
    fs::remove_dir_all(dir).unwrap();
}
