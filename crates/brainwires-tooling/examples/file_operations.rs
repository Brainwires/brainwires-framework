//! File Operations Example
//!
//! This example shows how to register file system tools and use them
//! in orchestrated scripts.
//!
//! Run with: `cargo run -p brainwires-tooling --features orchestrator --example file_operations`

use std::fs;
use brainwires_tooling::orchestrator::{ExecutionLimits, ToolOrchestrator};

fn main() {
    println!("=== File Operations Example ===\n");

    let mut orchestrator = ToolOrchestrator::new();

    orchestrator.register_executor("list_files", |input| {
        let path = input.as_str().unwrap_or(".");

        match fs::read_dir(path) {
            Ok(entries) => {
                let files: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| {
                        let path = e.path();
                        let is_dir = path.is_dir();
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        if is_dir {
                            format!("{}/ (dir)", name)
                        } else {
                            let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                            format!("{} ({}b)", name, size)
                        }
                    })
                    .collect();
                Ok(files.join("\n"))
            }
            Err(e) => Err(format!("Failed to list directory: {}", e)),
        }
    });

    orchestrator.register_executor("read_file", |input| {
        let path = input.as_str().unwrap_or("");

        match fs::read_to_string(path) {
            Ok(content) => {
                let truncated = if content.len() > 1000 {
                    format!("{}... (truncated)", &content[..1000])
                } else {
                    content
                };
                Ok(truncated)
            }
            Err(e) => Err(format!("Failed to read file: {}", e)),
        }
    });

    orchestrator.register_executor("file_info", |input| {
        let path = input.as_str().unwrap_or("");

        match fs::metadata(path) {
            Ok(meta) => {
                let info = format!(
                    r#"{{"size":{},"is_file":{},"is_dir":{},"readonly":{}}}"#,
                    meta.len(),
                    meta.is_file(),
                    meta.is_dir(),
                    meta.permissions().readonly()
                );
                Ok(info)
            }
            Err(e) => Err(format!("Failed to get file info: {}", e)),
        }
    });

    let script = r#"
        fn join_array(arr, sep) {
            let result = "";
            for i in 0..arr.len() {
                if i > 0 {
                    result += sep;
                }
                result += arr[i];
            }
            result
        }

        let files = list_files(".");
        let lines = files.split("\n");

        let rust_files = [];
        let directories = [];
        let total_size = 0;

        for line in lines {
            if line.contains("(dir)") {
                let name_parts = line.split("/");
                if name_parts.len() > 0 {
                    let name = name_parts[0];
                    if name != () && !name.starts_with(".") {
                        directories.push(name);
                    }
                }
            } else if line.ends_with(".rs") || line.contains(".rs (") {
                rust_files.push(line);

                let size_parts = line.split("(");
                if size_parts.len() > 1 {
                    let size_part = size_parts[size_parts.len() - 1];
                    let size_str_parts = size_part.split("b");
                    if size_str_parts.len() > 0 {
                        let size_str = size_str_parts[0];
                        let size = size_str.parse_int();
                        if size != () {
                            total_size += size;
                        }
                    }
                }
            }
        }

        let rust_files_list = join_array(rust_files, "\n");
        let directories_list = join_array(directories, ", ");

        `Directory Analysis:
- Total items: ${lines.len()}
- Rust files: ${rust_files.len()}
- Subdirectories: ${directories.len()}
- Total Rust code size: ${total_size} bytes

Rust files found:
${rust_files_list}

Subdirectories:
${directories_list}`
    "#;

    println!("Analyzing current directory...\n");

    let result = orchestrator
        .execute(script, ExecutionLimits::default())
        .expect("Script execution failed");

    println!("=== Analysis Result ===");
    println!("{}", result.output);
    println!("\n=== Execution Metrics ===");
    println!("Tool calls: {}", result.tool_calls.len());
    println!("Execution time: {}ms", result.execution_time_ms);

    println!("\n=== Reading Cargo.toml ===");

    let read_script = r#"
        let content = read_file("Cargo.toml");
        let lines = content.split("\n");
        let name_line = "";
        let version_line = "";

        for line in lines {
            if line.starts_with("name") {
                name_line = line;
            }
            if line.starts_with("version") {
                version_line = line;
            }
        }

        `Package Info:
${name_line}
${version_line}`
    "#;

    let result2 = orchestrator
        .execute(read_script, ExecutionLimits::default())
        .expect("Script execution failed");

    println!("{}", result2.output);
}
