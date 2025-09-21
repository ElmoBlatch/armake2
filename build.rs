use std::fs;
use std::path::Path;

fn main() {
    // Generate the parser files
    peg::cargo_build("src/config_grammar.rustpeg");
    peg::cargo_build("src/preprocess_grammar.rustpeg");

    // Fix deprecated range patterns in generated code for Rust 2021+ compatibility
    fix_generated_code();
}

fn fix_generated_code() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    // Fix both generated grammar files
    let files = [
        "config_grammar.rs",
        "preprocess_grammar.rs",
    ];

    for file in &files {
        let path = Path::new(&out_dir).join(file);
        if path.exists() {
            let content = fs::read_to_string(&path).unwrap();

            // Replace deprecated ... range patterns with ..=
            let fixed = content.replace("'0' ... '9'", "'0'..='9'")
                              .replace("'a' ... 'z'", "'a'..='z'")
                              .replace("'A' ... 'Z'", "'A'..='Z'")
                              .replace("'a' ... 'f'", "'a'..='f'")
                              .replace("'A' ... 'F'", "'A'..='F'");

            fs::write(&path, fixed).unwrap();

            println!("cargo:warning=Fixed deprecated patterns in {}", file);
        }
    }
}