use serde_json::json;

use crate::error::{DatasetError, DatasetResult};
use crate::types::{TrainingExample, TrainingMessage, TrainingRole};
use super::FormatConverter;

/// OpenAI chat fine-tuning JSONL format.
///
/// Format: `{"messages": [{"role": "...", "content": "..."}]}`
pub struct OpenAiFormat;

impl FormatConverter for OpenAiFormat {
    fn name(&self) -> &str {
        "openai"
    }

    fn to_json(&self, example: &TrainingExample) -> DatasetResult<serde_json::Value> {
        let messages: Vec<serde_json::Value> = example
            .messages
            .iter()
            .map(|msg| {
                let mut obj = json!({
                    "role": msg.role.to_string(),
                    "content": msg.content,
                });
                if let Some(ref tool_calls) = msg.tool_calls {
                    obj["tool_calls"] = json!(tool_calls);
                }
                if let Some(ref tool_call_id) = msg.tool_call_id {
                    obj["tool_call_id"] = json!(tool_call_id);
                }
                if let Some(ref name) = msg.name {
                    obj["name"] = json!(name);
                }
                obj
            })
            .collect();

        Ok(json!({ "messages": messages }))
    }

    fn from_json(&self, value: &serde_json::Value) -> DatasetResult<TrainingExample> {
        let messages_value = value.get("messages").ok_or_else(|| DatasetError::FormatConversion {
            message: "Missing 'messages' field".to_string(),
        })?;

        let messages_arr = messages_value.as_array().ok_or_else(|| DatasetError::FormatConversion {
            message: "'messages' must be an array".to_string(),
        })?;

        let mut messages = Vec::with_capacity(messages_arr.len());
        for msg_value in messages_arr {
            let role_str = msg_value.get("role")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DatasetError::FormatConversion {
                    message: "Message missing 'role'".to_string(),
                })?;

            let role = match role_str {
                "system" => TrainingRole::System,
                "user" => TrainingRole::User,
                "assistant" => TrainingRole::Assistant,
                "tool" => TrainingRole::Tool,
                other => return Err(DatasetError::FormatConversion {
                    message: format!("Unknown role: {}", other),
                }),
            };

            let content = msg_value.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let tool_calls = msg_value.get("tool_calls")
                .and_then(|v| v.as_array())
                .map(|arr| arr.clone());

            let tool_call_id = msg_value.get("tool_call_id")
                .and_then(|v| v.as_str())
                .map(String::from);

            let name = msg_value.get("name")
                .and_then(|v| v.as_str())
                .map(String::from);

            messages.push(TrainingMessage {
                role,
                content,
                tool_calls,
                tool_call_id,
                name,
            });
        }

        Ok(TrainingExample::new(messages))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_roundtrip() {
        let format = OpenAiFormat;
        let example = TrainingExample::new(vec![
            TrainingMessage::system("You are helpful"),
            TrainingMessage::user("Hello"),
            TrainingMessage::assistant("Hi there!"),
        ]);

        let json = format.to_json(&example).unwrap();
        let parsed = format.from_json(&json).unwrap();

        assert_eq!(parsed.messages.len(), 3);
        assert_eq!(parsed.messages[0].role, TrainingRole::System);
        assert_eq!(parsed.messages[1].content, "Hello");
        assert_eq!(parsed.messages[2].content, "Hi there!");
    }

    #[test]
    fn test_openai_format_structure() {
        let format = OpenAiFormat;
        let example = TrainingExample::new(vec![
            TrainingMessage::user("Q"),
            TrainingMessage::assistant("A"),
        ]);

        let json = format.to_json(&example).unwrap();
        assert!(json.get("messages").is_some());
        let messages = json["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "user");
    }
}
