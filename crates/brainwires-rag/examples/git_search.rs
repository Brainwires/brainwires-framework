//! Git history search example
//!
//! This example demonstrates how to use the git module to:
//! 1. Walk a git repository and extract recent commits
//! 2. Convert commits into searchable chunks for embedding
//!
//! Run with: cargo run --example git_search

use brainwires_rag::git::{CommitChunker, GitWalker};
use std::collections::HashSet;

fn main() {
    println!("=== Brainwires RAG - Git History Search Example ===\n");

    let walker = GitWalker::discover(".").expect("Should discover repo");
    println!("Repository: {}", walker.repo_path().display());

    let skip = HashSet::new();
    let commits = walker
        .iter_commits(None, Some(5), None, None, &skip)
        .expect("Should get commits");

    println!("Found {} recent commits\n", commits.len());

    for (i, commit) in commits.iter().enumerate() {
        println!(
            "{}. {} - {}",
            i + 1,
            &commit.hash[..8],
            commit.message.lines().next().unwrap_or("(no message)")
        );
        println!("   Author: {}", commit.author_name);
        println!("   Files: {}\n", commit.files_changed.len());
    }

    let chunker = CommitChunker::new();
    let chunks = chunker
        .commits_to_chunks(&commits, ".", None)
        .expect("Should create chunks");

    println!("Created {} searchable chunks", chunks.len());
    println!("Git search implementation working!");
}
