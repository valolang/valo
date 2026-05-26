
#[test]
fn dump_index_keys() {
    let dir = std::path::PathBuf::from("temp_test");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.valo"), r#"
Namespace Game.Graphics
Public Class Sprite
End Class
Sub Main()
End Sub
End Namespace
"#).unwrap();
    let project = valo_core::load_project(dir.join("main.valo")).unwrap();
    let index = valo_core::build_project_index(&project).unwrap();
    for key in index.by_qualified_name.keys() {
        println!("Key: {}", key);
    }
    std::fs::remove_dir_all(dir).unwrap();
}
