//! Integration tests for `FileOpsTool::resolve_path` behavior.
//!
//! `resolve_path` is the single seam between a caller-supplied path string
//! and actual filesystem I/O. Its *documented* responsibility is to anchor
//! relative paths against `context.working_directory` and canonicalise the
//! result. It does **not** enforce a sandbox: absolute paths and parent-dir
//! traversals (`..`) are passed through. These tests pin that current
//! behaviour so a later sandboxing change is a deliberate decision with a
//! visible diff, not a silent contract break.

use brainwires_core::ToolContext;
use brainwires_tools::FileOpsTool;
use proptest::prelude::*;
use std::fs;
use tempfile::TempDir;

fn ctx(dir: &std::path::Path) -> ToolContext {
    ToolContext {
        working_directory: dir.to_string_lossy().to_string(),
        ..Default::default()
    }
}

#[test]
fn relative_path_resolves_against_working_directory() {
    let tmp = TempDir::new().unwrap();
    let wd = tmp.path().canonicalize().unwrap();
    let child = wd.join("data.txt");
    fs::write(&child, "hi").unwrap();

    let resolved = FileOpsTool::resolve_path("data.txt", &ctx(&wd)).unwrap();
    assert_eq!(resolved, child);
}

#[test]
fn absolute_path_is_returned_verbatim_when_canonical() {
    let tmp = TempDir::new().unwrap();
    let wd = tmp.path().canonicalize().unwrap();
    let target = wd.join("abs.txt");
    fs::write(&target, "x").unwrap();

    let resolved = FileOpsTool::resolve_path(target.to_str().unwrap(), &ctx(&wd)).unwrap();
    assert_eq!(resolved, target);
}

#[test]
fn nonexistent_relative_path_still_anchors_against_working_directory() {
    // canonicalize() fails on a missing path; resolve_path documents it
    // returns the naive join as a fallback so callers can decide whether
    // to create the parent directory.
    let tmp = TempDir::new().unwrap();
    let wd = tmp.path().canonicalize().unwrap();

    let resolved = FileOpsTool::resolve_path("missing_subdir/new.txt", &ctx(&wd)).unwrap();
    assert_eq!(resolved, wd.join("missing_subdir").join("new.txt"));
}

#[test]
fn dotdot_traversal_is_not_blocked_current_behaviour() {
    // resolve_path does NOT enforce a sandbox. A relative `..` sequence that
    // escapes the working directory is canonicalised and returned. This test
    // pins that fact so any future sandbox-confinement change is explicit.
    //
    // If this test starts failing because a `..` path now errors, that's the
    // *intended* outcome — delete this test in the same commit.
    let tmp_root = TempDir::new().unwrap();
    let root = tmp_root.path().canonicalize().unwrap();
    let inner = root.join("inner");
    fs::create_dir_all(&inner).unwrap();
    let sibling = root.join("sibling.txt");
    fs::write(&sibling, "stolen").unwrap();

    // working_directory = inner; `../sibling.txt` escapes to root/sibling.txt.
    let resolved = FileOpsTool::resolve_path("../sibling.txt", &ctx(&inner)).unwrap();
    assert_eq!(
        resolved, sibling,
        "FYI: resolve_path currently permits `..` escapes — document in changelog if you fix",
    );
}

#[test]
fn nested_relative_path_resolves_correctly() {
    let tmp = TempDir::new().unwrap();
    let wd = tmp.path().canonicalize().unwrap();
    let nested = wd.join("a").join("b");
    fs::create_dir_all(&nested).unwrap();
    let file = nested.join("c.txt");
    fs::write(&file, "k").unwrap();

    let resolved = FileOpsTool::resolve_path("a/b/c.txt", &ctx(&wd)).unwrap();
    assert_eq!(resolved, file);
}

// ── Property: resolve_path is total (never panics) for arbitrary UTF-8 ───

proptest! {
    #[test]
    fn resolve_path_never_panics_on_arbitrary_string(input in ".{0,100}") {
        let tmp = TempDir::new().unwrap();
        let wd = tmp.path().canonicalize().unwrap();
        // Must succeed (resolve_path returns Result but is Ok(_) in all
        // currently-observed code paths). What matters: no panic on weird
        // input like embedded NULs, lone surrogates, control chars.
        let _ = FileOpsTool::resolve_path(&input, &ctx(&wd));
    }

    #[test]
    fn unicode_path_roundtrips_through_resolution(
        prefix in "[a-z]{1,5}",
        suffix in "[a-z]{1,5}",
    ) {
        let tmp = TempDir::new().unwrap();
        let wd = tmp.path().canonicalize().unwrap();
        // é / ü are 2-byte UTF-8 sequences — make sure resolve_path
        // doesn't mangle them when the file doesn't exist yet.
        let name = format!("{prefix}é{suffix}.txt");
        let resolved = FileOpsTool::resolve_path(&name, &ctx(&wd)).unwrap();
        prop_assert_eq!(resolved, wd.join(&name));
    }
}
