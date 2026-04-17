//! `ask_user_question` tool — lets the agent pause for a user answer.
//!
//! Routing:
//! - TUI mode: the executor's `user_question_tx` channel is set; the
//!   request flows to the TUI which renders an interactive picker and
//!   sends the answer back over the `oneshot` response channel.
//! - Plain CLI mode: no channel is set. The tool falls back to
//!   `dialoguer::Select` (for multi-choice) or `dialoguer::Input`
//!   (for free-text), via `spawn_blocking` so we don't starve the async
//!   runtime.
//! - Non-TTY (piped stdin / CI): returns a cancelled result, matching
//!   the behaviour of the approval and sudo paths in this codebase.

use std::collections::HashMap;
use std::io::IsTerminal;

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::ask::{UserQuestionRequest, UserQuestionResponse};
use crate::types::tool::{Tool, ToolInputSchema, ToolResult};

pub struct AskUserQuestionTool {
    tx: Option<mpsc::Sender<UserQuestionRequest>>,
}

impl AskUserQuestionTool {
    pub fn new(tx: Option<mpsc::Sender<UserQuestionRequest>>) -> Self {
        Self { tx }
    }

    pub fn get_tools() -> Vec<Tool> {
        vec![Self::tool_def()]
    }

    fn tool_def() -> Tool {
        let mut props = HashMap::new();
        props.insert(
            "question".to_string(),
            json!({
                "type": "string",
                "description": "The question to ask the user. Keep it short — one sentence is usually enough."
            }),
        );
        props.insert(
            "options".to_string(),
            json!({
                "type": "array",
                "items": {"type": "string"},
                "description": "Optional choices. When provided, the user picks from this list instead of typing free text."
            }),
        );
        props.insert(
            "multi_select".to_string(),
            json!({
                "type": "boolean",
                "description": "When true and `options` is non-empty, the user may choose multiple answers. Defaults to false."
            }),
        );
        Tool {
            name: "ask_user_question".to_string(),
            description: "Pause and ask the user a question. Returns `{answer}` for free-text or single-select, `{selected}` for multi-select, or `{cancelled}` if the user declined. Prefer this over rephrasing as text — a structured prompt is more reliable."
                .to_string(),
            input_schema: ToolInputSchema::object(props, vec!["question".to_string()]),
            requires_approval: false,
            defer_loading: false,
            ..Default::default()
        }
    }

    pub async fn execute(&self, tool_use_id: &str, input: &Value) -> ToolResult {
        #[derive(Deserialize)]
        struct Args {
            question: String,
            #[serde(default)]
            options: Vec<String>,
            #[serde(default)]
            multi_select: bool,
        }
        let args: Args = match serde_json::from_value(input.clone()) {
            Ok(a) => a,
            Err(e) => {
                return ToolResult::error(
                    tool_use_id.to_string(),
                    format!("invalid ask_user_question input: {}", e),
                );
            }
        };

        let response = if let Some(tx) = &self.tx {
            ask_via_channel(tx, &args.question, &args.options, args.multi_select).await
        } else {
            ask_via_dialoguer(&args.question, &args.options, args.multi_select).await
        };

        let payload = match response {
            UserQuestionResponse::Answer(s) => json!({ "answer": s }),
            UserQuestionResponse::Selected(v) => json!({ "selected": v }),
            UserQuestionResponse::Cancelled => json!({ "cancelled": true }),
        };
        ToolResult::success(
            tool_use_id.to_string(),
            serde_json::to_string_pretty(&payload).unwrap_or_default(),
        )
    }
}

async fn ask_via_channel(
    tx: &mpsc::Sender<UserQuestionRequest>,
    question: &str,
    options: &[String],
    multi_select: bool,
) -> UserQuestionResponse {
    let (resp_tx, resp_rx) = oneshot::channel();
    let req = UserQuestionRequest {
        id: uuid::Uuid::new_v4().to_string(),
        question: question.to_string(),
        options: options.to_vec(),
        multi_select: multi_select && !options.is_empty(),
        response_tx: resp_tx,
    };
    if tx.send(req).await.is_err() {
        return UserQuestionResponse::Cancelled;
    }
    resp_rx
        .await
        .unwrap_or(UserQuestionResponse::Cancelled)
}

async fn ask_via_dialoguer(
    question: &str,
    options: &[String],
    multi_select: bool,
) -> UserQuestionResponse {
    // Refuse to block on a non-TTY — in CI that would hang the job.
    if !std::io::stdin().is_terminal() {
        return UserQuestionResponse::Cancelled;
    }

    let q = question.to_string();
    let opts = options.to_vec();
    let handle = tokio::task::spawn_blocking(move || {
        use dialoguer::theme::ColorfulTheme;
        use dialoguer::{Input, MultiSelect, Select};
        let theme = ColorfulTheme::default();

        if opts.is_empty() {
            let answer: std::result::Result<String, _> = Input::with_theme(&theme)
                .with_prompt(&q)
                .allow_empty(false)
                .interact_text();
            match answer {
                Ok(s) => UserQuestionResponse::Answer(s),
                Err(_) => UserQuestionResponse::Cancelled,
            }
        } else if multi_select {
            let sel = MultiSelect::with_theme(&theme)
                .with_prompt(&q)
                .items(&opts)
                .interact();
            match sel {
                Ok(idxs) => {
                    let picked: Vec<String> = idxs.iter().map(|i| opts[*i].clone()).collect();
                    UserQuestionResponse::Selected(picked)
                }
                Err(_) => UserQuestionResponse::Cancelled,
            }
        } else {
            let sel = Select::with_theme(&theme)
                .with_prompt(&q)
                .items(&opts)
                .interact();
            match sel {
                Ok(i) => UserQuestionResponse::Answer(opts[i].clone()),
                Err(_) => UserQuestionResponse::Cancelled,
            }
        }
    });

    handle.await.unwrap_or(UserQuestionResponse::Cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn channel_answer_round_trips() {
        let (tx, mut rx) = mpsc::channel::<UserQuestionRequest>(4);
        // Responder: reply to any incoming request with "yes".
        let responder = tokio::spawn(async move {
            if let Some(req) = rx.recv().await {
                let _ = req.response_tx.send(UserQuestionResponse::Answer("yes".into()));
            }
        });

        let tool = AskUserQuestionTool::new(Some(tx));
        let out = tool
            .execute("t1", &json!({"question": "continue?"}))
            .await;
        responder.await.unwrap();

        assert!(!out.is_error);
        let v: Value = serde_json::from_str(&out.content).unwrap();
        assert_eq!(v["answer"], "yes");
    }

    #[tokio::test]
    async fn channel_multiselect_round_trips() {
        let (tx, mut rx) = mpsc::channel::<UserQuestionRequest>(4);
        let responder = tokio::spawn(async move {
            if let Some(req) = rx.recv().await {
                assert!(req.multi_select);
                let _ = req
                    .response_tx
                    .send(UserQuestionResponse::Selected(vec!["a".into(), "c".into()]));
            }
        });

        let tool = AskUserQuestionTool::new(Some(tx));
        let out = tool
            .execute(
                "t2",
                &json!({
                    "question": "pick some",
                    "options": ["a", "b", "c"],
                    "multi_select": true,
                }),
            )
            .await;
        responder.await.unwrap();

        let v: Value = serde_json::from_str(&out.content).unwrap();
        assert_eq!(v["selected"], json!(["a", "c"]));
    }

    #[tokio::test]
    async fn channel_cancelled_round_trips() {
        let (tx, mut rx) = mpsc::channel::<UserQuestionRequest>(4);
        let responder = tokio::spawn(async move {
            if let Some(req) = rx.recv().await {
                let _ = req.response_tx.send(UserQuestionResponse::Cancelled);
            }
        });

        let tool = AskUserQuestionTool::new(Some(tx));
        let out = tool
            .execute("t3", &json!({"question": "foo"}))
            .await;
        responder.await.unwrap();

        let v: Value = serde_json::from_str(&out.content).unwrap();
        assert_eq!(v["cancelled"], true);
    }

    #[tokio::test]
    async fn invalid_input_is_tool_error() {
        let tool = AskUserQuestionTool::new(None);
        // Missing required `question` field.
        let out = tool.execute("t4", &json!({})).await;
        assert!(out.is_error);
    }
}
