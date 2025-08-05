use std::fs;
use toml::Value;

fn main() {
    // The path is relative to the workspace root, where `just` is executed.
    let cargo_toml_path = "backend/Cargo.toml";

    let content = fs::read_to_string(cargo_toml_path)
        .unwrap_or_else(|e| panic!("💥 Failed to read {cargo_toml_path}: {e}"));

    let value: Value = content
        .parse()
        .unwrap_or_else(|e| panic!("💥 Failed to parse TOML from {cargo_toml_path}: {e}"));

    // Find the 'default' features array in the [features] table.
    let default_features = value
        .get("features")
        .and_then(|features| features.get("default"))
        .and_then(Value::as_array)
        .unwrap_or_else(|| {
            panic!("❌ Could not find '[features].default' array in {cargo_toml_path}")
        });

    // Check which database feature is present in the default set.
    let has_postgres = default_features
        .iter()
        .any(|v| v.as_str() == Some("db-postgres"));
    let has_sqlite = default_features
        .iter()
        .any(|v| v.as_str() == Some("db-sqlite"));

    if has_postgres {
        print!("postgres");
    } else if has_sqlite {
        print!("sqlite");
    } else {
        // Fallback if neither is found in the default features.
        eprintln!("⚠️ Warning: Neither 'db-postgres' nor 'db-sqlite' found in default features of backend/Cargo.toml. Defaulting to sqlite.");
        print!("sqlite");
    }

    // This is crucial: it prints the space separator for the Dockerfile script.
    print!(" ");

    // Check which UI feature is present in the default set.
    let has_svelte = default_features
        .iter()
        .any(|v| v.as_str() == Some("svelte-ui"));
    let has_slint = default_features
        .iter()
        .any(|v| v.as_str() == Some("slint-ui"));

    if has_svelte {
        print!("svelte");
    } else if has_slint {
        print!("slint");
    } else {
        // Fallback if neither is found in the default features.
        eprintln!("⚠️ Warning: Neither 'svelte-ui' nor 'slint-ui' found in default features of backend/Cargo.toml. Defaulting to svelte.");
        print!("svelte");
    }
}
