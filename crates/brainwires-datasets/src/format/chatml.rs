use serde_json::json;

use crate::error::{DatasetError, DatasetResult};
use crate::types::{TrainingExample, TrainingMessage, TrainingRole};
use super::FormatConverter;

/// ChatML template format.
///
/// Format: `{"text": "<|im_start|>system\n...<|im_end|>\n<|im_start|>user\n...<|im_end|>\n..."}`
pub struct ChatMlFormat;

impl ChatMlFormat {
    fn messages_to_chatml(messages: &[TrainingMessage]) -> String {
        let mut text = String::new();
        for msg in messages {
            let role = msg.role.to_string();
            text.push_str(&format!("<|im_start|>{}\n{}<|im_end|>\n", role, msg.content));
        }
        text
    }

    fn parse_chatml(text: &str) -> DatasetResult<Vec<TrainingMessage>> {
        let mut messages = Vec::new();
        let mut remaining = text;

        while let Some(start) = remaining.find("<|im_start|>") {
            remaining = &remaining[start + 12..]; // skip "<|im_start|>"

            let end = remaining.find("<|im_end|>").ok_or_else(|| DatasetError::FormatConversion {
                message: "Unclosed <|im_start|> tag".to_string(),
            })?;

            let block = &remaining[..end];
            let newline_pos = block.find('\n').unwrap_or(block.len());
            let role_str = block[..newline_pos].trim();
            let content = if newline_pos < block.len() {
                block[newline_pos + 1..].trim().to_string()
            } else {
                String::new()
            };

            let role = match role_str {
                "system" => TrainingRole::System,
                "user" => TrainingRole::User,
                "assistant" => TrainingRole::Assistant,
                "tool" => TrainingRole::Tool,
                other => {
                    return Err(DatasetError::FormatConversion {
                        message: format!("Unknown ChatML role: {}", other),
                    })
                }
            };

            messages.push(TrainingMessage::new(role, content));
            remaining = &remaining[end + 10..]; // skip "<|im_end|>"
        }

        if messages.is_empty() {
            return Err(DatasetError::FormatConversion {
                message: "No ChatML messages found".to_string(),
            });
        }

        Ok(messages)
    }
}

impl FormatConverter for ChatMlFormat {
    fn name(&self) -> &str {
        "chatml"
    }

    fn to_json(&self, example: &TrainingExample) -> DatasetResult<serde_json::Value> {
        let text = Self::messages_to_chatml(&example.messages);
        Ok(json!({ "text": text }))
    }

    fn parse_json(&self, value: &serde_json::Value) -> DatasetResult<TrainingExample> {
        let text = value
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DatasetError::FormatConversion {
                message: "Missing 'text' field for ChatML format".to_string(),
            })?;

        let messages = Self::parse_chatml(text)?;
        Ok(TrainingExample::new(messages))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chatml_roundtrip() {
        let format = ChatMlFormat;
        let example = TrainingExample::new(vec![
            TrainingMessage::system("You are helpful"),
            TrainingMessage::user("What is Rust?"),
            TrainingMessage::assistant("Rust is a systems programming language."),
        ]);

        let json = format.to_json(&example).unwrap();
        let text = json["text"].as_str().unwrap();
        assert!(text.contains("<|im_start|>system"));
        assert!(text.contains("<|im_start|>user"));
        assert!(text.contains("<|im_start|>assistant"));
        assert!(text.contains("<|im_end|>"));

        let parsed = format.parse_json(&json).unwrap();
        assert_eq!(parsed.messages.len(), 3);
        assert_eq!(parsed.messages[0].role, TrainingRole::System);
        assert_eq!(parsed.messages[2].content, "Rust is a systems programming language.");
    }

    #[test]
    fn test_chatml_format_structure() {
        let text = "<|im_start|>user\nHello<|im_end|>\n<|im_start|>assistant\nHi!<|im_end|>\n";
        let messages = ChatMlFormat::parse_chatml(text).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].content, "Hi!");
    }
}
