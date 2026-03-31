pub mod clippy;
pub mod dead_code;
pub mod doc_gaps;
pub mod eval_strategy;
pub mod refactoring;
pub mod test_coverage;
pub mod todo_scanner;

use anyhow::Result;
use async_trait::async_trait;

use super::config::StrategyConfig;

#[derive(Debug, Clone, PartialEq)]
pub enum ImprovementCategory {
    Linting,
    Testing,
    Documentation,
    Refactoring,
    DeadCode,
    /// Tasks generated from eval suite fault reports.
    EvalDriven,
}

impl std::fmt::Display for ImprovementCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImprovementCategory::Linting => write!(f, "linting"),
            ImprovementCategory::Testing => write!(f, "testing"),
            ImprovementCategory::Documentation => write!(f, "documentation"),
            ImprovementCategory::Refactoring => write!(f, "refactoring"),
            ImprovementCategory::DeadCode => write!(f, "dead_code"),
            ImprovementCategory::EvalDriven => write!(f, "eval_driven"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImprovementTask {
    pub id: String,
    pub strategy: String,
    pub category: ImprovementCategory,
    pub description: String,
    pub target_files: Vec<String>,
    pub priority: u8,
    pub estimated_diff_lines: u32,
    pub context: String,
}

#[async_trait]
pub trait ImprovementStrategy: Send + Sync {
    fn name(&self) -> &str;
    fn category(&self) -> ImprovementCategory;
    async fn generate_tasks(
        &self,
        repo_path: &str,
        config: &StrategyConfig,
    ) -> Result<Vec<ImprovementTask>>;
}

pub fn all_strategies() -> Vec<Box<dyn ImprovementStrategy>> {
    vec![
        Box::new(clippy::ClippyStrategy),
        Box::new(todo_scanner::TodoScannerStrategy),
        Box::new(doc_gaps::DocGapsStrategy),
        Box::new(test_coverage::TestCoverageStrategy),
        Box::new(refactoring::RefactoringStrategy),
        Box::new(dead_code::DeadCodeStrategy),
    ]
}
