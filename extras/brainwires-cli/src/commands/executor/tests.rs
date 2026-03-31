//! Tests for CommandExecutor

use super::*;

#[test]
fn test_parse_input() {
    let executor = CommandExecutor::default();

    assert!(executor.parse_input("hello").is_none());
    assert!(executor.parse_input("").is_none());

    let (cmd, args) = executor.parse_input("/help").unwrap();
    assert_eq!(cmd, "help");
    assert_eq!(args.len(), 0);

    let (cmd, args) = executor.parse_input("/model llama-3.3-70b").unwrap();
    assert_eq!(cmd, "model");
    assert_eq!(args, vec!["llama-3.3-70b"]);
}

#[test]
fn test_execute_help() {
    let executor = CommandExecutor::default();
    let result = executor.execute("help", &[]).unwrap();

    match result {
        CommandResult::Help(lines) => {
            assert!(!lines.is_empty());
            assert!(lines[0].contains("Available Commands"));
        }
        _ => panic!("Expected Help result"),
    }
}

#[test]
fn test_execute_clear() {
    let executor = CommandExecutor::default();
    let result = executor.execute("clear", &[]).unwrap();

    match result {
        CommandResult::Action(CommandAction::ClearHistory) => {}
        _ => panic!("Expected ClearHistory action"),
    }
}

#[test]
fn test_execute_model() {
    let executor = CommandExecutor::default();
    let result = executor
        .execute("model", &["llama-3.3-70b".to_string()])
        .unwrap();

    match result {
        CommandResult::Action(CommandAction::SwitchModel(name)) => {
            assert_eq!(name, "llama-3.3-70b");
        }
        _ => panic!("Expected SwitchModel action"),
    }
}
