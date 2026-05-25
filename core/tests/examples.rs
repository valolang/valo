use std::fs;
use std::path::Path;

#[test]
fn test_official_examples() {
    let mut examples_dir = Path::new("examples");
    if !examples_dir.exists() {
        examples_dir = Path::new("../examples");
    }

    let mut entries: Vec<_> = fs::read_dir(examples_dir)
        .expect("Failed to read examples directory")
        .filter_map(|entry| entry.ok())
        .collect();

    // Sort entries to ensure deterministic test order
    entries.sort_by_key(|e| e.path());

    let mut failures = Vec::new();
    let mut count = 0;
    let mut skipped = 0;

    for entry in entries {
        let path = entry.path();
        let extension = path.extension().and_then(|s| s.to_str());
        let file_name = path.file_name().and_then(|s| s.to_str());

        if extension == Some("valo") || extension == Some("bas") {
            #[cfg(not(windows))]
            if file_name == Some("com_dictionary.valo") {
                skipped += 1;
                continue;
            }

            count += 1;
            match valo_core::run_file(&path) {
                Ok(output) => {
                    // Optional: Simple output verification for stable examples
                    if (file_name == Some("hello.valo") || file_name == Some("hello.bas"))
                        && output != vec!["Hello, Valo"]
                    {
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
