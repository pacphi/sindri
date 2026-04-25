use std::fs;
use std::path::Path;

fn main() {
    let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // tools/
        .unwrap()
        .parent() // v4/
        .unwrap()
        .join("schemas");

    fs::create_dir_all(&schema_dir).expect("create schemas dir");
    println!("Schema generation placeholder — implement after schemars derives are added.");
    println!("Schema dir: {}", schema_dir.display());
}
