use anyhow::Result;
use async_trait::async_trait;
use walkdir::WalkDir;

use super::{ImprovementCategory, ImprovementStrategy, ImprovementTask};
use crate::self_improve::config::StrategyConfig;

pub struct RefactoringStrategy;

struct CodeSmell {
    file: String,
    kind: SmellKind,
    detail: String,
}

enum SmellKind {
    LargeFile(usize),
    LongFunction(String, usize),
}

#[async_trait]
impl ImprovementStrategy for RefactoringStrategy {
    fn name(&self) -> &str {
        "refactoring"
    }

    fn category(&self) -> ImprovementCategory {
        ImprovementCategory::Refactoring
    }

    async fn generate_tasks(
        &self,
        repo_path: &str,
        config: &StrategyConfig,
    ) -> Result<Vec<ImprovementTask>> {
        let src_path = format!("{repo_path}/src");
        let mut smells: Vec<CodeSmell> = Vec::new();

        for entry in WalkDir::new(&src_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "rs")
            })
        {
            let path = entry.path();
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let rel_path = path
                .strip_prefix(repo_path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let line_count = content.lines().count();

            // Large file detection
            if line_count > 500 {
                smells.push(CodeSmell {
                    file: rel_path.clone(),
                    kind: SmellKind::LargeFile(line_count),
                    detail: format!("File has {line_count} lines (threshold: 500)"),
                });
            }

            // Long function detection
            let mut fn_name = String::new();
            let mut fn_start: Option<usize> = None;
            let mut brace_depth: i32 = 0;
            let mut in_function = false;

            for (i, line) in content.lines().enumerate() {
                let trimmed = line.trim();

                if !in_function {
                    if (trimmed.starts_with("pub fn ")
                        || trimmed.starts_with("pub async fn ")
                        || trimmed.starts_with("fn ")
                        || trimmed.starts_with("async fn "))
                        && trimmed.contains('{')
                    {
                        fn_name = trimmed
                            .replace("pub async fn ", "")
                            .replace("pub fn ", "")
                            .replace("async fn ", "")
                            .replace("fn ", "")
                            .split('(')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        fn_start = Some(i);
                        in_function = true;
                        brace_depth = 0;
                    }
                }

                if in_function {
                    for ch in trimmed.chars() {
                        match ch {
                            '{' => brace_depth += 1,
                            '}' => brace_depth -= 1,
                            _ => {}
                        }
                    }

                    if brace_depth == 0 && fn_start.is_some() {
                        let start = fn_start.unwrap();
                        let fn_lines = i - start + 1;
                        if fn_lines > 60 {
                            smells.push(CodeSmell {
                                file: rel_path.clone(),
                                kind: SmellKind::LongFunction(fn_name.clone(), fn_lines),
                                detail: format!(
                                    "Function '{fn_name}' is {fn_lines} lines (threshold: 60), starts at line {}",
                                    start + 1
                                ),
                            });
                        }
                        in_function = false;
                        fn_start = None;
                    }
                }
            }
        }

        let mut tasks: Vec<ImprovementTask> = smells
            .into_iter()
            .take(config.max_tasks_per_strategy)
            .enumerate()
            .map(|(i, smell)| {
                let (description, priority, estimated_diff) = match &smell.kind {
                    SmellKind::LargeFile(lines) => (
                        format!(
                            "Refactor {} ({lines} lines) by extracting logical sections \
                             into separate modules or helper functions. Identify cohesive \
                             groups of functionality that can be extracted.",
                            smell.file
                        ),
                        2, // Lower priority - big refactors are risky
                        50u32,
                    ),
                    SmellKind::LongFunction(name, lines) => (
                        format!(
                            "Refactor function '{name}' in {} ({lines} lines) by extracting \
                             logical steps into smaller helper functions. Each extracted \
                             function should have a clear single responsibility.",
                            smell.file
                        ),
                        4,
                        30u32,
                    ),
                };

                ImprovementTask {
                    id: format!("refactor-{i}"),
                    strategy: "refactoring".to_string(),
                    category: ImprovementCategory::Refactoring,
                    description,
                    target_files: vec![smell.file],
                    priority,
                    estimated_diff_lines: estimated_diff,
                    context: smell.detail,
                }
            })
            .collect();

        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(tasks)
    }
}
