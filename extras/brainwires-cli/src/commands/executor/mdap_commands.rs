//! MDAP Commands
//!
//! Commands for MDAP (Massively Decomposed Agentic Processes) mode configuration.

use anyhow::Result;

use super::{CommandAction, CommandExecutor, CommandResult};

impl CommandExecutor {
    /// Execute MDAP-related commands
    pub(super) fn execute_mdap_command(
        &self,
        name: &str,
        args: &[String],
    ) -> Option<Result<CommandResult>> {
        match name {
            "mdap" => Some(self.cmd_mdap(args)),
            "mdap:on" => Some(Ok(CommandResult::Action(CommandAction::MdapEnable))),
            "mdap:off" => Some(Ok(CommandResult::Action(CommandAction::MdapDisable))),
            "mdap:k" => Some(self.cmd_mdap_k(args)),
            "mdap:target" => Some(self.cmd_mdap_target(args)),
            _ => None,
        }
    }

    fn cmd_mdap(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            // No args - show status
            Ok(CommandResult::Action(CommandAction::MdapStatus))
        } else {
            // Toggle on/off
            match args[0].to_lowercase().as_str() {
                "on" | "enable" | "true" | "1" => {
                    Ok(CommandResult::Action(CommandAction::MdapEnable))
                }
                "off" | "disable" | "false" | "0" => {
                    Ok(CommandResult::Action(CommandAction::MdapDisable))
                }
                _ => {
                    anyhow::bail!(
                        "Usage: /mdap [on|off]\n\n\
                        Show MDAP status or toggle MDAP mode.\n\n\
                        Options:\n\
                        - on/enable  - Enable MDAP high-reliability mode\n\
                        - off/disable - Disable MDAP mode\n\n\
                        Related commands:\n\
                        - /mdap:k <value>     - Set vote margin (default: 3)\n\
                        - /mdap:target <rate> - Set target success rate (default: 0.95)"
                    );
                }
            }
        }
    }

    fn cmd_mdap_k(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /mdap:k <value>\n\n\
                Set the vote margin (k) for MDAP voting.\n\n\
                The vote margin determines how many more votes the winning response\n\
                needs over the runner-up. Higher values increase reliability but cost.\n\n\
                Recommended values:\n\
                - k=1: Fast, ~95% accuracy\n\
                - k=3: Balanced (default), ~99% accuracy\n\
                - k=5: High reliability, ~99.9% accuracy"
            );
        }

        let k: u32 = args[0].parse().map_err(|_| {
            anyhow::anyhow!("Invalid k value: '{}'. Must be a positive integer.", args[0])
        })?;

        if k == 0 {
            anyhow::bail!("k must be at least 1");
        }

        if k > 10 {
            anyhow::bail!("k cannot exceed 10 (would be extremely expensive)");
        }

        Ok(CommandResult::Action(CommandAction::MdapSetK(k)))
    }

    fn cmd_mdap_target(&self, args: &[String]) -> Result<CommandResult> {
        if args.is_empty() {
            anyhow::bail!(
                "Usage: /mdap:target <rate>\n\n\
                Set the target success rate for MDAP execution.\n\n\
                The rate should be between 0.5 and 0.999.\n\
                Higher rates require more samples and cost more.\n\n\
                Examples:\n\
                - 0.95 (95%) - Standard reliability\n\
                - 0.99 (99%) - High reliability\n\
                - 0.999 (99.9%) - Very high reliability"
            );
        }

        // Handle percentage format (e.g., "95%") or decimal (e.g., "0.95")
        let input = args[0].trim_end_matches('%');
        let target: f64 = input.parse().map_err(|_| {
            anyhow::anyhow!("Invalid target rate: '{}'. Use decimal (0.95) or percentage (95%).", args[0])
        })?;

        // Convert percentage to decimal if needed
        let target = if target > 1.0 { target / 100.0 } else { target };

        if target <= 0.5 {
            anyhow::bail!("Target rate must be greater than 0.5 (50%)");
        }

        if target >= 1.0 {
            anyhow::bail!("Target rate must be less than 1.0 (100%). Perfect reliability is not achievable.");
        }

        Ok(CommandResult::Action(CommandAction::MdapSetTarget(target)))
    }
}
