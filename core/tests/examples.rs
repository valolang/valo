use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn test_official_examples() {
    let examples_dir = examples_dir();
    let entries = runnable_examples(&examples_dir);

    let mut failures = Vec::new();
    let mut count = 0;
    let mut skipped = 0;

    for path in entries {
        let file_name = path.file_name().and_then(|s| s.to_str());

        if should_skip_example(file_name) {
            skipped += 1;
            continue;
        }

        count += 1;
        match valo_core::run_file(&path) {
            Ok(output) => {
                if is_hello_example(file_name) && output != vec!["Hello, Valo"] {
                    failures.push(format!(
                        "Example {:?} produced incorrect output: expected [\"Hello, Valo\"], got {:?}",
                        path, output
                    ));
                }
            }
            Err(diag) => {
                failures.push(format!("{:?}: Failed with error: {}", path, diag));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Integration tests failed for {}/{} examples:\n\n{}",
            failures.len(),
            count,
            failures.join("\n")
        );
    }

    println!("Successfully ran {} examples ({} skipped).", count, skipped);
}

fn examples_dir() -> PathBuf {
    let path = Path::new("examples");
    if path.exists() {
        return path.to_path_buf();
    }
    Path::new("../examples").to_path_buf()
}

fn runnable_examples(examples_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_runnable_examples(examples_dir, examples_dir, &mut paths);
    paths.sort();
    paths
}

fn collect_runnable_examples(root: &Path, dir: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("Failed to read examples directory {:?}: {err}", dir))
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if path.is_dir() {
            collect_runnable_examples(root, &path, paths);
        } else if is_runnable_example(root, &path) {
            paths.push(path);
        }
    }
}

fn is_runnable_example(root: &Path, path: &Path) -> bool {
    if !is_source_file(path) {
        return false;
    }
    if !has_sub_main(path) {
        return false;
    }

    match path.parent() {
        Some(parent) if parent == root => true,
        _ => path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("main")),
    }
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("valo" | "bas" | "cls")
    )
}

fn has_sub_main(path: &Path) -> bool {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("Failed to read example {:?}: {err}", path));
    source.lines().any(|line| {
        line.trim_start()
            .to_ascii_lowercase()
            .starts_with("sub main")
    })
}

fn is_hello_example(file_name: Option<&str>) -> bool {
    matches!(file_name, Some("hello.valo" | "hello.bas"))
}

fn should_skip_example(file_name: Option<&str>) -> bool {
    if let Some(name) = file_name
        && cfg!(not(windows))
        && name.starts_with("com_")
    {
        return true;
    }
    false
}
