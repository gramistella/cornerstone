use common::{ContactDto, Credentials, LoginResponse};
use std::fs;
use std::path::Path;
use ts_rs::TS;

fn main() {
    // Collect all exported types
    let a = ContactDto::export_to_string().unwrap();
    let b = Credentials::export_to_string().unwrap();
    let c = LoginResponse::export_to_string().unwrap();
    
    let all_types = format!("{}{}{}", a, b, c);
    let cleaned_types = remove_duplicate_comments(&all_types);
    
    // Define the output path relative to the workspace root
    let out_path = Path::new("frontend_svelte/src/lib/types.ts");
    
    // Create the directory if it doesn't exist
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    
    fs::write(out_path, cleaned_types).unwrap();
    println!("✅ TypeScript definitions generated at: {}", out_path.display());
}

fn remove_duplicate_comments(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut found_first_comment = false;
    
    for line in lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            if !found_first_comment {
                result.push(line);
                found_first_comment = true;
            }
            // Skip duplicate comment lines but don't affect spacing
        } else if line.trim().is_empty() {
            // Always preserve empty lines for proper spacing
            result.push(line);
        } else {
            result.push(line);
        }
    }
    
    result.join("\n") + "\n"
}