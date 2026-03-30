//! Context/Working Set Command Handlers
//!
//! Handlers for managing the working set (files in context).

use anyhow::Result;
use std::path::PathBuf;

use crate::commands::executor::CommandAction;
use crate::types::agent::AgentContext;
use crate::types::working_set::estimate_tokens;
use crate::utils::logger::Logger;

/// Handle context-related command actions
pub async fn handle_context_action(
    action: CommandAction,
    context: &mut AgentContext,
) -> Result<bool> {
    match action {
        CommandAction::ContextShow => {
            show_working_set(context);
            Ok(true)
        }
        CommandAction::ContextAdd(path, pinned) => {
            add_to_working_set(context, &path, pinned)?;
            Ok(true)
        }
        CommandAction::ContextRemove(path) => {
            remove_from_working_set(context, &path);
            Ok(true)
        }
        CommandAction::ContextPin(path) => {
            pin_in_working_set(context, &path);
            Ok(true)
        }
        CommandAction::ContextUnpin(path) => {
            unpin_in_working_set(context, &path);
            Ok(true)
        }
        CommandAction::ContextClear(keep_pinned) => {
            clear_working_set(context, keep_pinned);
            Ok(true)
        }
        _ => Ok(true),
    }
}

/// Display the current working set
fn show_working_set(context: &AgentContext) {
    println!("{}\n", console::style("Working Set").cyan().bold());
    println!("{}\n", context.working_set.display());

    // Show stale files warning
    let stale = context.working_set.stale_files();
    if !stale.is_empty() {
        println!("{}", console::style("⏳ Stale files will be evicted on next turn unless accessed").dim());
    }
}

/// Add a file to the working set
fn add_to_working_set(context: &mut AgentContext, path: &str, pinned: bool) -> Result<()> {
    // Resolve the path
    let file_path = resolve_path(path, &context.working_directory)?;

    // Check if file exists
    if !file_path.exists() {
        println!("{}: File not found: {}\n",
            console::style("Error").red().bold(),
            file_path.display());
        return Ok(());
    }

    // Read file to estimate tokens
    let content = std::fs::read_to_string(&file_path)?;
    let tokens = estimate_tokens(&content);

    let file_name = file_path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());

    if pinned {
        context.working_set.add_pinned(file_path.clone(), tokens, Some(&file_name));
        println!("{} {} (~{} tokens, 📌 pinned)\n",
            console::style("Added and pinned:").green(),
            file_path.display(),
            tokens);
    } else {
        let eviction = context.working_set.add(file_path.clone(), tokens);
        println!("{} {} (~{} tokens)\n",
            console::style("Added:").green(),
            file_path.display(),
            tokens);

        if let Some(reason) = eviction {
            println!("{}: {}\n", console::style("Note").yellow(), reason);
        }
    }

    Logger::info(&format!("Added to working set: {} ({} tokens)", path, tokens));
    Ok(())
}

/// Remove a file from the working set
fn remove_from_working_set(context: &mut AgentContext, path: &str) {
    let file_path = match resolve_path(path, &context.working_directory) {
        Ok(p) => p,
        Err(_) => PathBuf::from(path),
    };

    if context.working_set.remove(&file_path) {
        println!("{} {}\n",
            console::style("Removed:").green(),
            file_path.display());
        Logger::info(&format!("Removed from working set: {}", path));
    } else {
        println!("{}: {} is not in the working set\n",
            console::style("Not found").yellow(),
            file_path.display());
    }
}

/// Pin a file in the working set
fn pin_in_working_set(context: &mut AgentContext, path: &str) {
    let file_path = match resolve_path(path, &context.working_directory) {
        Ok(p) => p,
        Err(_) => PathBuf::from(path),
    };

    if context.working_set.pin(&file_path) {
        println!("{} {} 📌\n",
            console::style("Pinned:").green(),
            file_path.display());
        Logger::info(&format!("Pinned in working set: {}", path));
    } else {
        println!("{}: {} is not in the working set. Add it first with /context:add\n",
            console::style("Not found").yellow(),
            file_path.display());
    }
}

/// Unpin a file in the working set
fn unpin_in_working_set(context: &mut AgentContext, path: &str) {
    let file_path = match resolve_path(path, &context.working_directory) {
        Ok(p) => p,
        Err(_) => PathBuf::from(path),
    };

    if context.working_set.unpin(&file_path) {
        println!("{} {}\n",
            console::style("Unpinned:").green(),
            file_path.display());
        Logger::info(&format!("Unpinned in working set: {}", path));
    } else {
        println!("{}: {} is not in the working set\n",
            console::style("Not found").yellow(),
            file_path.display());
    }
}

/// Clear the working set
fn clear_working_set(context: &mut AgentContext, keep_pinned: bool) {
    let count_before = context.working_set.len();
    context.working_set.clear(keep_pinned);
    let count_after = context.working_set.len();
    let removed = count_before - count_after;

    if keep_pinned && count_after > 0 {
        println!("{} Cleared {} file(s), kept {} pinned file(s)\n",
            console::style("✓").green(),
            removed,
            count_after);
    } else {
        println!("{} Cleared {} file(s) from working set\n",
            console::style("✓").green(),
            removed);
    }

    Logger::info(&format!("Cleared working set: {} files removed, {} kept", removed, count_after));
}

/// Resolve a path relative to the working directory
fn resolve_path(path: &str, working_directory: &str) -> Result<PathBuf> {
    let path_buf = PathBuf::from(path);

    if path_buf.is_absolute() {
        Ok(path_buf)
    } else {
        let base = PathBuf::from(working_directory);
        Ok(base.join(path_buf).canonicalize()?)
    }
}
