use std::path::PathBuf;

fn main() {
    // Get the path to the directory containing the Cargo.toml for this crate.
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    // Construct the path to the common UI directory relative to the manifest directory.
    // This navigates up one level from `frontend` and then into `common/ui`.
    let common_ui_path = manifest_dir.join("../common/ui");

    // Create a configuration object to manually specify the include path.
    let config = slint_build::CompilerConfiguration::new()
        // The .with_include_paths() function expects a Vec<PathBuf>.
        .with_include_paths(vec![common_ui_path]);

    // Compile your UI file using the custom configuration.
    slint_build::compile_with_config("ui/app.slint", config).unwrap();
}