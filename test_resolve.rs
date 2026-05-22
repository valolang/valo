use std::path::{Path, PathBuf};

fn main() {
    let p = Path::new("System/IO.valo");
    for c in p.components() {
        println!("{:?}", c);
    }
}
