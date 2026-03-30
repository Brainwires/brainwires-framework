//! BRAINWIRES.md Parser
//!
//! Parses project-specific instruction files with @file.md import support

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum depth for recursive imports to prevent infinite loops
const MAX_IMPORT_DEPTH: usize = 10;

/// Parse a BRAINWIRES.md file and resolve all @file.md imports
pub fn load_brainwires_instructions(base_path: &Path) -> Result<String> {
    let brainwires_path = base_path.join("BRAINWIRES.md");

    if !brainwires_path.exists() {
        return Ok(String::new());
    }

    let mut visited = HashSet::new();
    parse_file_with_imports(&brainwires_path, &mut visited, 0)
}

/// Recursively parse a markdown file and resolve @file.md imports
fn parse_file_with_imports(
    file_path: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<String> {
    // Check recursion depth
    if depth > MAX_IMPORT_DEPTH {
        anyhow::bail!("Maximum import depth exceeded ({}). Possible circular dependency.", MAX_IMPORT_DEPTH);
    }

    // Check for circular imports
    let canonical_path = file_path.canonicalize()
        .with_context(|| format!("Failed to resolve path: {}", file_path.display()))?;

    if visited.contains(&canonical_path) {
        anyhow::bail!("Circular import detected: {}", file_path.display());
    }

    visited.insert(canonical_path.clone());

    // Read file content
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Process imports
    let mut result = String::new();
    let base_dir = file_path.parent()
        .with_context(|| format!("Failed to get parent directory for: {}", file_path.display()))?;

    for line in content.lines() {
        if let Some(import_path) = parse_import_line(line) {
            // Resolve relative path
            let imported_file = base_dir.join(import_path);

            if !imported_file.exists() {
                anyhow::bail!("Import file not found: {} (referenced in {})",
                    imported_file.display(), file_path.display());
            }

            // Recursively parse imported file
            let imported_content = parse_file_with_imports(&imported_file, visited, depth + 1)?;
            result.push_str(&imported_content);
            result.push('\n');
        } else {
            // Regular line
            result.push_str(line);
            result.push('\n');
        }
    }

    // Remove from visited set to allow the same file to be imported from different paths
    visited.remove(&canonical_path);

    Ok(result)
}

/// Parse an import line and extract the file path
/// Supports formats:
/// - @file.md
/// - @path/to/file.md
/// - @../relative/path.md
fn parse_import_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();

    if trimmed.starts_with('@') && !trimmed.starts_with("@@") {
        // Extract path after @
        let path = &trimmed[1..];

        // Skip if it looks like a mention or other @ syntax
        if path.contains(' ') || path.is_empty() {
            return None;
        }

        Some(path)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_import_line() {
        assert_eq!(parse_import_line("@file.md"), Some("file.md"));
        assert_eq!(parse_import_line("@path/to/file.md"), Some("path/to/file.md"));
        assert_eq!(parse_import_line("  @../relative.md  "), Some("../relative.md"));
        assert_eq!(parse_import_line("@@not-an-import"), None);
        assert_eq!(parse_import_line("@mention with spaces"), None);
        assert_eq!(parse_import_line("regular text"), None);
    }
}
