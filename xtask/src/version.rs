use std::path::{Path, PathBuf};
use std::process::ExitCode;
use walkdir::WalkDir;

/// Bump all version references across the workspace.
///
/// Updates:
/// 1. `[workspace.package].version` in root Cargo.toml
/// 2. `version = "X.Y.Z"` on internal crate deps in `[workspace.dependencies]`
/// 3. Hardcoded version strings in `*.rs` source files
/// 4. `version = "X.Y"` dependency examples in `*.md` files
/// 5. `## [Unreleased]` → `## [X.Y.Z]` in CHANGELOG.md (adds fresh Unreleased above)
pub fn bump_version(args: &[String]) -> ExitCode {
    let new_version = match args.first() {
        Some(v) => v.as_str(),
        None => {
            eprintln!("Usage: cargo xtask bump-version <VERSION>");
            eprintln!("Example: cargo xtask bump-version 0.3.0");
            return ExitCode::FAILURE;
        }
    };

    // Validate semver format
    let parts: Vec<&str> = new_version.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.parse::<u32>().is_err()) {
        eprintln!("Error: version must be semver (e.g. 0.3.0), got: {new_version}");
        return ExitCode::FAILURE;
    }

    let major_minor = format!("{}.{}", parts[0], parts[1]);

    let workspace_root = workspace_root();
    println!("Workspace root: {}", workspace_root.display());
    println!("Bumping to version {new_version} (short: {major_minor})");
    println!();

    let mut changes = 0u32;

    // 1. Update root Cargo.toml (workspace.package + workspace.dependencies)
    changes += update_workspace_cargo_toml(&workspace_root, new_version);

    // 2. Update member Cargo.toml files with direct path deps (e.g. brainwires-wasm)
    changes += update_member_cargo_tomls(&workspace_root, new_version);

    // 3. Update hardcoded versions in *.rs files
    changes += update_rs_files(&workspace_root, new_version);

    // 4. Update version examples in *.md files
    changes += update_md_files(&workspace_root, &major_minor);

    // 5. Stamp CHANGELOG.md: [Unreleased] → [X.Y.Z] with fresh Unreleased above
    changes += update_changelog(&workspace_root, new_version);

    println!();
    if changes > 0 {
        println!("Done! Updated {changes} file(s).");
        println!();
        println!("Next steps:");
        println!("  1. Review changes: git diff");
        println!("  2. Run: cargo check --workspace");
        println!("  3. Commit the version bump");
    } else {
        println!("No files needed updating.");
    }

    ExitCode::SUCCESS
}

fn workspace_root() -> PathBuf {
    // xtask binary is at <root>/target/..., but we run via `cargo xtask`
    // which sets CWD to the workspace root. Use CARGO_MANIFEST_DIR of the
    // workspace (xtask's parent).
    let xtask_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    xtask_dir
        .parent()
        .expect("xtask should be inside workspace")
        .to_path_buf()
}

/// Update the root Cargo.toml:
/// - `[workspace.package].version`
/// - All `version = "..."` on internal brainwires-* deps in `[workspace.dependencies]`
fn update_workspace_cargo_toml(root: &Path, new_version: &str) -> u32 {
    let cargo_path = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_path).expect("Failed to read root Cargo.toml");

    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .expect("Failed to parse root Cargo.toml");

    let mut changed = false;

    // Update [workspace.package].version
    if let Some(pkg) = doc.get_mut("workspace").and_then(|w| w.get_mut("package"))
        && let Some(v) = pkg.get_mut("version")
    {
        let old = v.as_str().unwrap_or("").to_string();
        if old != new_version {
            *v = toml_edit::value(new_version);
            println!("  [workspace.package].version: {old} -> {new_version}");
            changed = true;
        }
    }

    // Update [workspace.dependencies].brainwires-* version fields
    if let Some(deps) = doc
        .get_mut("workspace")
        .and_then(|w| w.get_mut("dependencies"))
        && let Some(table) = deps.as_table_like_mut()
    {
        for (key, value) in table.iter_mut() {
            if !key.starts_with("brainwires") {
                continue;
            }
            // Only update inline tables with a `path` key (internal crates)
            if let Some(tbl) = value.as_inline_table_mut()
                && tbl.contains_key("path")
                && let Some(v) = tbl.get_mut("version")
            {
                let old = v.as_str().unwrap_or("").to_string();
                if old != new_version {
                    *v = toml_edit::value(new_version)
                        .into_value()
                        .expect("string is a value");
                    println!("  [workspace.dependencies].{key}: {old} -> {new_version}");
                    changed = true;
                }
            }
        }
    }

    if changed {
        std::fs::write(&cargo_path, doc.to_string()).expect("Failed to write root Cargo.toml");
        println!("  Updated: {}", cargo_path.display());
        1
    } else {
        println!("  Root Cargo.toml: already at {new_version}");
        0
    }
}

/// Scan member Cargo.toml files for direct `path = "..."` deps on brainwires crates
/// that have a hardcoded `version` field (e.g. brainwires-wasm which can't use workspace
/// inheritance due to `default-features = false` override limitation).
fn update_member_cargo_tomls(root: &Path, new_version: &str) -> u32 {
    let mut count = 0u32;

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            name != "target" && name != ".git" && name != "node_modules"
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()) != Some("Cargo.toml") {
            continue;
        }
        // Skip the root Cargo.toml (already handled)
        if path == root.join("Cargo.toml") {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut doc = match content.parse::<toml_edit::DocumentMut>() {
            Ok(d) => d,
            Err(_) => continue,
        };

        let mut changed = false;

        // Check [dependencies] and [dev-dependencies]
        for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
            let Some(deps) = doc.get_mut(section) else {
                continue;
            };
            let Some(table) = deps.as_table_like_mut() else {
                continue;
            };

            for (key, value) in table.iter_mut() {
                if !key.starts_with("brainwires") {
                    continue;
                }
                if let Some(tbl) = value.as_inline_table_mut()
                    && tbl.contains_key("path")
                    && !tbl.contains_key("workspace")
                    && let Some(v) = tbl.get_mut("version")
                {
                    let old = v.as_str().unwrap_or("").to_string();
                    if old != new_version {
                        *v = toml_edit::value(new_version)
                            .into_value()
                            .expect("string is a value");
                        println!("  [{section}].{key}: {old} -> {new_version}");
                        changed = true;
                    }
                }
            }
        }

        if changed {
            std::fs::write(path, doc.to_string()).expect("Failed to write member Cargo.toml");
            println!("  Updated: {}", path.display());
            count += 1;
        }
    }

    if count == 0 {
        println!("  No member Cargo.toml files needed updating.");
    }
    count
}

/// Find and update hardcoded version strings in Rust source files.
/// Looks for patterns like `"version": "X.Y.Z"` and `"0.2.0"` in brainwires contexts.
fn update_rs_files(root: &Path, new_version: &str) -> u32 {
    let mut count = 0u32;

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            name != "target" && name != ".git" && name != "node_modules"
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Replace "version": "X.Y.Z" patterns (JSON-style in Rust string literals)
        // and version: "X.Y.Z".into() patterns
        let new_content = replace_version_in_rs(&content, new_version);

        if new_content != content {
            std::fs::write(path, &new_content).expect("Failed to write .rs file");
            println!("  Updated: {}", path.display());
            count += 1;
        }
    }

    if count == 0 {
        println!("  No .rs files needed updating.");
    }
    count
}

/// Replace version strings in Rust source that match brainwires version patterns.
fn replace_version_in_rs(content: &str, new_version: &str) -> String {
    let mut result = content.to_string();

    // Pattern: "version": "X.Y.Z" (JSON in Rust strings)
    // We look for the specific pattern used in protocol.rs and similar
    let patterns = [
        // JSON-style: "version": "X.Y.Z"
        (r#""version": ""#, '"'),
        // Rust struct field: version: "X.Y.Z".into()
        (r#"version: ""#, '"'),
        // Rust assert/comparison: config.version, "X.Y.Z")
        (r#"version, ""#, '"'),
    ];

    for (prefix, terminator) in &patterns {
        let mut search_from = 0;
        loop {
            let Some(start) = result[search_from..].find(prefix) else {
                break;
            };
            let abs_start = search_from + start;
            let value_start = abs_start + prefix.len();
            let Some(end) = result[value_start..].find(*terminator) else {
                break;
            };
            let old_ver = &result[value_start..value_start + end];
            // Only replace if it looks like a brainwires version (0.x.y pattern)
            if old_ver.starts_with("0.") && old_ver.split('.').count() == 3 {
                let before = &result[..value_start];
                let after = &result[value_start + end..];
                result = format!("{before}{new_version}{after}");
                search_from = value_start + new_version.len();
            } else {
                search_from = value_start + end;
            }
        }
    }

    result
}

/// Update version references in Markdown files.
/// Replaces `brainwires[-*] = { version = "X.Y"` and `brainwires[-*] = "X.Y"` patterns.
fn update_md_files(root: &Path, new_major_minor: &str) -> u32 {
    let mut count = 0u32;

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            name != "target" && name != ".git" && name != "node_modules"
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        // Skip CHANGELOG files — version references there are historical
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if filename.to_ascii_uppercase().contains("CHANGELOG") {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let new_content = replace_version_in_md(&content, new_major_minor);

        if new_content != content {
            std::fs::write(path, &new_content).expect("Failed to write .md file");
            println!("  Updated: {}", path.display());
            count += 1;
        }
    }

    if count == 0 {
        println!("  No .md files needed updating.");
    }
    count
}

/// Update CHANGELOG.md: rename `## [Unreleased]` to `## [X.Y.Z]` and insert
/// a fresh empty `## [Unreleased]` section above it.
///
/// Looks for the first line matching `## [Unreleased]` (case-insensitive on the
/// word "Unreleased"). If the section has content, it becomes the new release
/// section. A blank `## [Unreleased]` header is inserted above it.
fn update_changelog(root: &Path, new_version: &str) -> u32 {
    let changelog_path = root.join("CHANGELOG.md");
    let content = match std::fs::read_to_string(&changelog_path) {
        Ok(c) => c,
        Err(_) => {
            println!("  CHANGELOG.md: not found, skipping");
            return 0;
        }
    };

    // Find the `## [Unreleased]` line (case-insensitive match on "unreleased").
    let mut lines: Vec<&str> = content.lines().collect();
    let unreleased_idx = lines.iter().position(|line| {
        let trimmed = line.trim();
        trimmed.to_ascii_lowercase().starts_with("## [unreleased]")
    });

    let Some(idx) = unreleased_idx else {
        println!("  CHANGELOG.md: no ## [Unreleased] section found, skipping");
        return 0;
    };

    // Build the today's date string for the release heading.
    let today = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Convert to YYYY-MM-DD without pulling in chrono.
        let days_since_epoch = now / 86400;
        let (y, m, d) = days_to_ymd(days_since_epoch);
        format!("{y:04}-{m:02}-{d:02}")
    };

    // Replace the existing Unreleased line with the versioned heading.
    let versioned_heading = format!("## [{new_version}] - {today}");

    // Insert a fresh Unreleased section above the old one.
    // Result: ## [Unreleased] / blank / ## [X.Y.Z] - YYYY-MM-DD / (original content)
    lines[idx] = &versioned_heading;
    let fresh_section = ["## [Unreleased]", ""];
    let mut new_lines: Vec<&str> = Vec::with_capacity(lines.len() + fresh_section.len());
    new_lines.extend_from_slice(&lines[..idx]);
    new_lines.extend_from_slice(&fresh_section);
    new_lines.extend_from_slice(&lines[idx..]);

    // Rebuild with trailing newline.
    let mut new_content = new_lines.join("\n");
    if content.ends_with('\n') {
        new_content.push('\n');
    }

    if new_content == content {
        println!("  CHANGELOG.md: already stamped for {new_version}");
        return 0;
    }

    std::fs::write(&changelog_path, &new_content).expect("Failed to write CHANGELOG.md");
    println!("  CHANGELOG.md: [Unreleased] -> [{new_version}] - {today}");
    println!("  Updated: {}", changelog_path.display());
    1
}

/// Convert days since Unix epoch to (year, month, day).
///
/// Simple civil date calculation — no leap-second precision needed for
/// changelog timestamps.
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant's `chrono`-compatible date conversion.
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Replace `brainwires* = { version = "X.Y"` and `brainwires* = "X.Y"` in markdown.
fn replace_version_in_md(content: &str, new_major_minor: &str) -> String {
    let mut result = String::with_capacity(content.len());

    for line in content.lines() {
        let new_line = replace_brainwires_version_in_line(line, new_major_minor);
        result.push_str(&new_line);
        result.push('\n');
    }

    // Preserve original trailing newline behavior
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Replace version in a single markdown line for brainwires crate references.
fn replace_brainwires_version_in_line(line: &str, new_mm: &str) -> String {
    // Pattern 1: brainwires[-*] = { version = "X.Y", ... }
    // Pattern 2: brainwires[-*] = "X.Y"
    if !line.contains("brainwires") {
        return line.to_string();
    }

    let mut result = line.to_string();

    // Pattern 1: version = "X.Y" (inside inline table or toml)
    let version_eq = "version = \"";
    let mut search_from = 0;
    loop {
        let Some(ver_pos) = result[search_from..].find(version_eq) else {
            break;
        };
        let abs_pos = search_from + ver_pos;

        if !result[..abs_pos].contains("brainwires") {
            search_from = abs_pos + version_eq.len();
            continue;
        }

        let value_start = abs_pos + version_eq.len();
        let Some(end) = result[value_start..].find('"') else {
            break;
        };
        let old_ver = &result[value_start..value_start + end];

        if old_ver.starts_with("0.") && old_ver != new_mm {
            let before = &result[..value_start].to_string();
            let after = &result[value_start + end..].to_string();
            result = format!("{before}{new_mm}{after}");
            search_from = value_start + new_mm.len();
        } else {
            search_from = value_start + end;
        }
    }

    // Pattern 2: brainwires[-*] = "X.Y" (simple form, no inline table)
    // Match: `brainwires` optionally followed by `-word` segments, then ` = "X.Y"`
    // Skip lines already handled by Pattern 1 (contain `version = "`)
    if !result.contains("version = \"") {
        let eq_quote = "= \"";
        search_from = 0;
        loop {
            let Some(eq_pos) = result[search_from..].find(eq_quote) else {
                break;
            };
            let abs_eq = search_from + eq_pos;

            // Check that a brainwires identifier immediately precedes ` = "`
            let before_eq = result[..abs_eq].trim_end();
            if !before_eq.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '-') {
                search_from = abs_eq + eq_quote.len();
                continue;
            }
            // Walk backwards to find the start of the identifier
            let ident_end = before_eq.len();
            let ident_start = before_eq
                .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                .map(|i| i + 1)
                .unwrap_or(0);
            let ident = &before_eq[ident_start..ident_end];
            if !ident.starts_with("brainwires") {
                search_from = abs_eq + eq_quote.len();
                continue;
            }

            let value_start = abs_eq + eq_quote.len();
            let Some(end) = result[value_start..].find('"') else {
                break;
            };
            let old_ver = &result[value_start..value_start + end];

            if old_ver.starts_with("0.") && old_ver != new_mm {
                let before = result[..value_start].to_string();
                let after = result[value_start + end..].to_string();
                result = format!("{before}{new_mm}{after}");
                search_from = value_start + new_mm.len();
            } else {
                search_from = value_start + end;
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rs_version_json_style() {
        // Test JSON-style version replacement in Rust source
        let input = concat!(
            r#"    "version": ""#,
            "0.1.0",
            r#"""#,
            "\n",
            r#"    version: ""#,
            "0.1.0",
            r#"".into()"#,
        );
        let result = replace_version_in_rs(input, "0.5.0");
        assert!(result.contains("0.5.0"), "should contain new version");
        assert!(!result.contains("0.1.0"), "should not contain old version");
    }

    #[test]
    fn test_md_inline_table() {
        let input = r#"brainwires = { version = "0.1", features = ["agents"] }"#;
        let result = replace_brainwires_version_in_line(input, "0.5");
        assert_eq!(
            result,
            r#"brainwires = { version = "0.5", features = ["agents"] }"#
        );
    }

    #[test]
    fn test_md_leaves_non_brainwires_alone() {
        let input = r#"tokio = { version = "1.43", features = ["full"] }"#;
        let result = replace_brainwires_version_in_line(input, "0.5");
        assert_eq!(result, input);
    }

    #[test]
    fn test_md_hyphenated_crate() {
        let input = r#"brainwires-agent-network = { version = "0.1", features = ["mesh"] }"#;
        let result = replace_brainwires_version_in_line(input, "0.5");
        assert_eq!(
            result,
            r#"brainwires-agent-network = { version = "0.5", features = ["mesh"] }"#
        );
    }

    #[test]
    fn test_md_simple_form() {
        let input = r#"brainwires-storage = "0.3""#;
        let result = replace_brainwires_version_in_line(input, "0.5");
        assert_eq!(result, r#"brainwires-storage = "0.5""#);
    }

    #[test]
    fn test_md_simple_form_with_comment() {
        let input = r#"brainwires = "0.2"  # default features: tools + agents"#;
        let result = replace_brainwires_version_in_line(input, "0.5");
        assert_eq!(
            result,
            r#"brainwires = "0.5"  # default features: tools + agents"#
        );
    }

    #[test]
    fn test_md_simple_form_leaves_non_brainwires() {
        let input = r#"tokio = "1.43""#;
        let result = replace_brainwires_version_in_line(input, "0.5");
        assert_eq!(result, input);
    }

    #[test]
    fn test_md_simple_form_no_change_when_current() {
        let input = r#"brainwires = "0.5""#;
        let result = replace_brainwires_version_in_line(input, "0.5");
        assert_eq!(result, input);
    }

    #[test]
    fn test_days_to_ymd_epoch() {
        // 1970-01-01
        let (y, m, d) = days_to_ymd(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_ymd_known_date() {
        // 2026-03-14 is day 20526 since epoch
        let (y, m, d) = days_to_ymd(20526);
        assert_eq!((y, m, d), (2026, 3, 14));
    }

    #[test]
    fn test_changelog_update() {
        let tmpdir = std::env::temp_dir().join("xtask_changelog_test");
        let _ = std::fs::create_dir_all(&tmpdir);
        let changelog = tmpdir.join("CHANGELOG.md");
        std::fs::write(
            &changelog,
            "# Changelog\n\n## [Unreleased]\n\n### Added\n- Cool feature\n\n## [0.3.0] - 2025-12-01\n",
        )
        .unwrap();

        let count = update_changelog(&tmpdir, "0.4.0");
        assert_eq!(count, 1);

        let result = std::fs::read_to_string(&changelog).unwrap();
        // Should have a fresh Unreleased section
        assert!(result.contains("## [Unreleased]\n\n## [0.4.0]"));
        // The release date should be today
        assert!(result.contains("## [0.4.0] - "));
        // Original content should be preserved under the new version heading
        assert!(result.contains("### Added\n- Cool feature"));
        // Old release should still be there
        assert!(result.contains("## [0.3.0] - 2025-12-01"));

        let _ = std::fs::remove_dir_all(&tmpdir);
    }

    #[test]
    fn test_changelog_no_unreleased() {
        let tmpdir = std::env::temp_dir().join("xtask_changelog_test_none");
        let _ = std::fs::create_dir_all(&tmpdir);
        let changelog = tmpdir.join("CHANGELOG.md");
        std::fs::write(&changelog, "# Changelog\n\n## [0.3.0]\n").unwrap();

        let count = update_changelog(&tmpdir, "0.4.0");
        assert_eq!(count, 0, "should not modify if no [Unreleased] section");

        let _ = std::fs::remove_dir_all(&tmpdir);
    }
}
