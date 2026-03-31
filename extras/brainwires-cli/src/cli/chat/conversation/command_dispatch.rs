//! Command Dispatch
//!
//! Handles routing slash commands to appropriate handlers.

use anyhow::Result;

use super::checkpoint_commands::handle_checkpoint_action;
use super::context_commands::handle_context_action;
use super::history_commands::handle_history_action;
use super::misc_commands::handle_misc_action;
use super::plan_commands::handle_plan_action;
use crate::commands::CommandExecutor;
use crate::commands::executor::{CommandAction, CommandResult};
use crate::types::agent::AgentContext;
use crate::types::message::{Message, MessageContent, Role};
use crate::utils::checkpoint::CheckpointManager;
use crate::utils::conversation::ConversationManager;
use crate::utils::logger::Logger;

/// Handle slash command execution
#[allow(clippy::too_many_arguments)]
pub async fn handle_command(
    command_executor: &CommandExecutor,
    cmd_name: &str,
    cmd_args: &[String],
    model_id: &str,
    context: &mut AgentContext,
    conversation_manager: &mut ConversationManager,
    cleared_conversation_manager: &mut Option<ConversationManager>,
    checkpoint_manager: &CheckpointManager,
    message_store: &crate::storage::MessageStore,
) -> Result<bool> {
    match command_executor.execute(cmd_name, cmd_args) {
        Ok(CommandResult::Help(lines)) => {
            for line in lines {
                println!("{}", line);
            }
            println!();
            Ok(true)
        }
        Ok(CommandResult::Action(action)) => {
            handle_command_action(
                action,
                model_id,
                context,
                conversation_manager,
                cleared_conversation_manager,
                checkpoint_manager,
                message_store,
            )
            .await
        }
        Ok(CommandResult::ActionWithMessage(action, msg)) => {
            // Execute action first (e.g., mode switch), then add message
            handle_command_action(
                action,
                model_id,
                context,
                conversation_manager,
                cleared_conversation_manager,
                checkpoint_manager,
                message_store,
            )
            .await?;

            // Then add the message to conversation for AI processing
            let expanded_message = Message {
                role: Role::User,
                content: MessageContent::Text(msg.clone()),
                name: None,
                metadata: None,
            };
            conversation_manager.add_message(expanded_message);
            context.conversation_history.push(Message {
                role: Role::User,
                content: MessageContent::Text(msg),
                name: None,
                metadata: None,
            });
            Ok(true)
        }
        Ok(CommandResult::Message(msg)) => {
            // Command produced a message to send to AI
            let expanded_message = Message {
                role: Role::User,
                content: MessageContent::Text(msg.clone()),
                name: None,
                metadata: None,
            };
            conversation_manager.add_message(expanded_message);
            context.conversation_history.push(Message {
                role: Role::User,
                content: MessageContent::Text(msg),
                name: None,
                metadata: None,
            });
            Ok(true)
        }
        Err(e) => {
            Logger::error(format!("Command error: {}", e));
            println!("{}: {}\n", console::style("Error").red().bold(), e);
            Ok(true)
        }
    }
}

/// Route command actions to appropriate handlers
async fn handle_command_action(
    action: CommandAction,
    model_id: &str,
    context: &mut AgentContext,
    conversation_manager: &mut ConversationManager,
    cleared_conversation_manager: &mut Option<ConversationManager>,
    checkpoint_manager: &CheckpointManager,
    message_store: &crate::storage::MessageStore,
) -> Result<bool> {
    match &action {
        // History commands
        CommandAction::ClearHistory
        | CommandAction::ResumeHistory(_)
        | CommandAction::Rewind(_)
        | CommandAction::ShowStatus => {
            handle_history_action(
                action,
                model_id,
                context,
                conversation_manager,
                cleared_conversation_manager,
                message_store,
            )
            .await
        }

        // Checkpoint commands
        CommandAction::CreateCheckpoint(_)
        | CommandAction::RestoreCheckpoint(_)
        | CommandAction::ListCheckpoints => {
            handle_checkpoint_action(
                action,
                model_id,
                context,
                conversation_manager,
                checkpoint_manager,
            )
            .await
        }

        // Plan commands
        CommandAction::ListPlans(_)
        | CommandAction::ShowPlan(_)
        | CommandAction::DeletePlan(_)
        | CommandAction::ActivatePlan(_)
        | CommandAction::DeactivatePlan
        | CommandAction::PlanStatus
        | CommandAction::PausePlan
        | CommandAction::ResumePlan(_)
        | CommandAction::SearchPlans(_)
        | CommandAction::BranchPlan(_, _)
        | CommandAction::MergePlan(_)
        | CommandAction::PlanTree(_) => handle_plan_action(action).await,

        // Context/Working Set commands
        CommandAction::ContextShow
        | CommandAction::ContextAdd(_, _)
        | CommandAction::ContextRemove(_)
        | CommandAction::ContextPin(_)
        | CommandAction::ContextUnpin(_)
        | CommandAction::ContextClear(_) => handle_context_action(action, context).await,

        // All other commands
        _ => handle_misc_action(action, context, conversation_manager).await,
    }
}
