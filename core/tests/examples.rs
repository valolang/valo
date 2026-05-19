use std::fs;
use std::path::Path;
use valo_core::run_source;

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

    for entry in entries {
        let path = entry.path();
        let extension = path.extension().and_then(|s| s.to_str());

        if extension == Some("valo") || extension == Some("bas") {
            count += 1;
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("Failed to read example file: {:?}", path));

            match run_source(&source) {
                Ok(output) => {
                    // Optional: Simple output verification for stable examples
                    if (path.file_name().and_then(|s| s.to_str()) == Some("hello.valo")
                        || path.file_name().and_then(|s| s.to_str()) == Some("hello.bas"))
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

    println!("Successfully ran {} examples.", count);
}
