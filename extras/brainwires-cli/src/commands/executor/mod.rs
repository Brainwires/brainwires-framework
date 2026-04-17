//! Command Executor
//!
//! Executes slash commands and handles special built-in commands

mod agent_commands;
mod context_commands;
mod conversation_commands;
mod knowledge_commands;
mod mdap_commands;
mod misc_commands;
mod personal_commands;
mod plan_commands;
mod project_commands;
mod prompt_mode_commands;
mod skill_commands;
mod task_commands;
mod template_commands;
mod tools_commands;

#[cfg(test)]
mod tests;

use anyhow::{Context, Result};
use std::collections::HashMap;

use super::parser;
use super::registry::{Command, CommandRegistry};

/// Command execution result
#[derive(Debug)]
pub enum CommandResult {
    /// Command produced a text message to add to conversation
    Message(String),
    /// Command executed an action (like /clear)
    Action(CommandAction),
    /// Action that also sends a message to AI (e.g., /ask some question)
    ActionWithMessage(CommandAction, String),
    /// Show help information
    Help(Vec<String>),
}

/// Actions that commands can trigger
#[derive(Debug)]
pub enum CommandAction {
    /// Clear conversation history
    ClearHistory,
    /// Switch to a different model
    SwitchModel(String),
    /// Switch to a different provider (reconstructs the Provider instance)
    SwitchProvider(String),
    /// Show the current provider + list of available providers
    ListProviders,
    /// Show status
    ShowStatus,
    /// Rewind conversation
    Rewind(usize),
    /// Create a checkpoint
    CreateCheckpoint(Option<String>),
    /// Restore from a checkpoint
    RestoreCheckpoint(String),
    /// List all checkpoints
    ListCheckpoints,
    /// Resume conversation after clear or load from history
    ResumeHistory(Option<String>),
    /// Exit the application
    Exit,
    /// Set approval mode
    SetApprovalMode(String),
    /// Execute shell command (requires confirmation)
    ExecCommand(String),
    /// Drop into an interactive shell with the terminal handed over.
    OpenShell,
    /// Show shell command history
    ShowShellHistory,
    /// Open hotkey configuration dialog
    OpenHotkeyDialog,
    /// List plans (optionally filtered by conversation)
    ListPlans(Option<String>),
    /// Show a specific plan by ID
    ShowPlan(String),
    /// Delete a plan by ID
    DeletePlan(String),
    /// Activate a plan (set as current working plan)
    ActivatePlan(String),
    /// Deactivate the current plan
    DeactivatePlan,
    /// Show current plan status
    PlanStatus,
    /// Pause the current plan (preserves task state)
    PausePlan,
    /// Resume a paused plan
    ResumePlan(String),
    /// Show tasks for current plan
    ShowTasks,
    /// Mark a task as complete (task_id is optional - defaults to current in-progress task)
    TaskComplete(Option<String>),
    /// Skip a task (task_id, reason)
    TaskSkip(Option<String>, Option<String>),
    /// Add a new task to current plan
    TaskAdd(String),
    /// Start a specific task
    TaskStart(String),
    /// Block a task with optional reason
    TaskBlock(String, Option<String>),
    /// Add a dependency between tasks
    TaskDepends(String, String),
    /// Show tasks ready to execute
    TaskReady,
    /// Show time info for a task (or all tasks if None)
    TaskTime(Option<String>),
    /// Show enhanced task list with IDs
    TaskList,
    /// Execute a plan with optional approval mode
    ExecutePlan(String, Option<String>),
    /// List all templates
    ListTemplates,
    /// Save current plan as template (name, description)
    SaveTemplate(String, Option<String>),
    /// Show a template by name/ID
    ShowTemplate(String),
    /// Use a template to create a new plan (name, var substitutions)
    UseTemplate(String, Vec<String>),
    /// Delete a template
    DeleteTemplate(String),
    /// Search plans by query
    SearchPlans(String),
    /// Create a branch from active plan (branch_name, task_description)
    BranchPlan(String, String),
    /// Merge a branch plan (optional plan_id)
    MergePlan(Option<String>),
    /// Show plan hierarchy tree
    PlanTree(Option<String>),
    /// Show working set (files in context)
    ContextShow,
    /// Add file to working set (path, pinned)
    ContextAdd(String, bool),
    /// Remove file from working set
    ContextRemove(String),
    /// Pin a file in working set
    ContextPin(String),
    /// Unpin a file in working set
    ContextUnpin(String),
    /// Clear working set (keep_pinned)
    ContextClear(bool),
    /// Show current tool mode and usage
    ShowToolMode,
    /// Set tool selection mode
    SetToolMode(crate::types::tool::ToolMode),
    /// Open tool picker UI (for explicit mode)
    OpenToolPicker,
    /// Show MDAP status
    MdapStatus,
    /// Enable MDAP mode
    MdapEnable,
    /// Disable MDAP mode
    MdapDisable,
    /// Set MDAP vote margin (k)
    MdapSetK(u32),
    /// Set MDAP target success rate
    MdapSetTarget(f64),
    /// Learn a behavioral truth (rule, optional rationale)
    LearnTruth(String, Option<String>),
    /// Show knowledge system status
    KnowledgeStatus,
    /// List truths (optional category filter)
    KnowledgeList(Option<String>),
    /// Search truths
    KnowledgeSearch(String),
    /// Force sync with server
    KnowledgeSync,
    /// Contradict a truth (id, optional reason)
    KnowledgeContradict(String, Option<String>),
    /// Delete a truth from local cache
    KnowledgeDelete(String),

    // Personal Knowledge System Actions
    /// Show personal profile summary
    ProfileShow,
    /// Set a personal fact (key, value, local_only)
    ProfileSet(String, String, bool),
    /// List personal facts (optional category filter)
    ProfileList(Option<String>),
    /// Search personal facts
    ProfileSearch(String),
    /// Delete a personal fact (id or key)
    ProfileDelete(String),
    /// Force sync personal facts with server
    ProfileSync,
    /// Export profile to JSON file (path)
    ProfileExport(Option<String>),
    /// Import profile from JSON file (path)
    ProfileImport(String),
    /// Show profile statistics
    ProfileStats,

    // Multi-Agent System Actions
    /// List all active agents with tree view
    ListAgents,
    /// Switch to a different agent session
    SwitchAgent(String),
    /// Spawn a new child agent (model, reason)
    SpawnChildAgent(Option<String>, Option<String>),
    /// Show agent tree hierarchy
    AgentTree,
    /// Hibernate all agents (save state for later)
    HibernateAgents,
    /// Resume hibernated agents
    ResumeAgents,

    // Skill Actions
    /// Invoke a skill by name with optional arguments
    InvokeSkill(String, Vec<String>),
    /// List all available skills
    ListSkills,
    /// Show detailed information about a skill
    ShowSkill(String),
    /// Reload skills from disk
    ReloadSkills,
    /// Create a new skill (name, location: personal|project)
    CreateSkill(String, Option<String>),

    // Prompt Mode Actions
    /// Switch to Ask mode (read-only)
    SetPromptModeAsk,
    /// Switch to Edit mode (full tools)
    SetPromptModeEdit,

    // Plan Mode Actions
    /// Enter plan mode with optional focus
    EnterPlanMode(Option<String>),
    /// Exit plan mode and return to main context
    ExitPlanMode,
    /// Show plan mode status
    PlanModeStatus,
    /// Clear plan mode history
    ClearPlanMode,
    /// Export plan mode session to file (optional path)
    ExportPlanMode(Option<String>),
}

/// Command executor
pub struct CommandExecutor {
    registry: CommandRegistry,
}

impl CommandExecutor {
    /// Create a new command executor
    pub fn new() -> Result<Self> {
        let mut registry = CommandRegistry::new();
        registry.load_builtin()?;

        // Try to load custom commands from current directory
        if let Ok(cwd) = std::env::current_dir() {
            let commands_dir = cwd.join(".brainwires/commands");
            if let Err(e) = registry.load_custom(&commands_dir) {
                tracing::warn!("Failed to load custom commands: {}", e);
            }
        }

        Ok(Self { registry })
    }

    /// Parse a slash command from user input
    /// Returns (command_name, arguments) if input is a command, None otherwise
    pub fn parse_input(&self, input: &str) -> Option<(String, Vec<String>)> {
        let input = input.trim();

        if !input.starts_with('/') {
            return None;
        }

        // Split into command and args
        let parts: Vec<&str> = input[1..].split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let command_name = parts[0].to_string();
        let args = parts[1..].iter().map(|s| s.to_string()).collect();

        Some((command_name, args))
    }

    /// Execute a slash command
    pub fn execute(&self, command_name: &str, args: &[String]) -> Result<CommandResult> {
        let command = self
            .registry
            .get(command_name)
            .with_context(|| format!("Unknown command: /{}", command_name))?;

        // Handle built-in commands with special logic
        if command.builtin {
            return self.execute_builtin(command_name, args);
        }

        // Execute custom command
        self.execute_custom(command, args)
    }

    /// Execute a custom command
    fn execute_custom(&self, command: &Command, args: &[String]) -> Result<CommandResult> {
        // Build argument map
        let mut arg_map = HashMap::new();

        for (i, arg_def) in command.args.iter().enumerate() {
            if i < args.len() {
                arg_map.insert(arg_def.name.clone(), args[i].clone());
            } else if arg_def.required {
                anyhow::bail!("Missing required argument: {}", arg_def.name);
            }
        }

        // Render template
        let rendered = parser::render_template(&command.content, &arg_map);

        Ok(CommandResult::Message(rendered))
    }

    /// Get the command registry
    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            let mut registry = CommandRegistry::new();
            let _ = registry.load_builtin();
            Self { registry }
        })
    }
}
