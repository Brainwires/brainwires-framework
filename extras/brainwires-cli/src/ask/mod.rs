//! Out-of-band "ask the user" channel.
//!
//! The executor exposes an `ask_user_question` tool that pauses the agent
//! and prompts the user through whichever UI is active. This module defines
//! the request / response types carried over the
//! `mpsc<UserQuestionRequest> + oneshot<UserQuestionResponse>` pattern
//! already used by approval (`crate::approval`) and sudo (`crate::sudo`).

use tokio::sync::oneshot;

#[derive(Debug)]
pub struct UserQuestionRequest {
    pub id: String,
    pub question: String,
    /// If set, the user picks from these. If empty, they type free-text.
    pub options: Vec<String>,
    /// Only honored when `options` is non-empty. When `true`, the TUI may
    /// let the user tick multiple boxes.
    pub multi_select: bool,
    pub response_tx: oneshot::Sender<UserQuestionResponse>,
}

#[derive(Debug, Clone)]
pub enum UserQuestionResponse {
    /// Free-text or single-choice answer.
    Answer(String),
    /// Multi-select answer (only emitted when `multi_select` was set).
    Selected(Vec<String>),
    /// User cancelled (Esc, Ctrl+C, or non-TTY with no way to prompt).
    Cancelled,
}

/// Build a `(QuestionBlock, QuestionAnswerState)` pair so the existing TUI
/// `question_panel` renderer can display a one-shot `ask_user_question`
/// prompt with zero new rendering code. The synthetic block has one
/// `ClarifyingQuestion` built from the tool arguments.
pub fn to_question_block(
    req: &UserQuestionRequest,
) -> (
    crate::types::question::QuestionBlock,
    crate::types::question::QuestionAnswerState,
) {
    use crate::types::question::{
        ClarifyingQuestion, QuestionAnswerState, QuestionBlock, QuestionOption,
    };

    let has_options = !req.options.is_empty();
    let options = req
        .options
        .iter()
        .enumerate()
        .map(|(i, label)| QuestionOption {
            id: format!("opt-{}", i),
            label: label.clone(),
            description: None,
        })
        .collect::<Vec<_>>();

    let question = ClarifyingQuestion {
        id: req.id.clone(),
        question: req.question.clone(),
        header: "Ask".to_string(),
        options,
        // Multi-select only meaningful when options are present; for
        // free-text we rely on the "Other" field as the single text input.
        multi_select: has_options && req.multi_select,
        ambiguity_type: None,
    };

    let block = QuestionBlock {
        ambiguity_analysis: None,
        questions: vec![question],
    };
    let state = QuestionAnswerState::new(&block);
    (block, state)
}

/// Drain a completed `QuestionAnswerState` into a `UserQuestionResponse`.
/// The `block` must be the one originally produced by [`to_question_block`]
/// (single question).
pub fn collect_response(
    req: &UserQuestionRequest,
    block: &crate::types::question::QuestionBlock,
    state: &crate::types::question::QuestionAnswerState,
) -> UserQuestionResponse {
    let Some(q) = block.questions.first() else {
        return UserQuestionResponse::Cancelled;
    };

    // Multi-select path: gather every ticked label + any text in "Other".
    if req.multi_select && !req.options.is_empty() {
        let mut picked: Vec<String> = q
            .options
            .iter()
            .enumerate()
            .filter_map(|(i, opt)| {
                let selected = state
                    .selected_options
                    .first()
                    .and_then(|row| row.get(i).copied())
                    .unwrap_or(false);
                if selected {
                    Some(opt.label.clone())
                } else {
                    None
                }
            })
            .collect();
        let other_selected = state.other_selected.first().copied().unwrap_or(false);
        if other_selected {
            if let Some(text) = state.other_text.first() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    picked.push(trimmed.to_string());
                }
            }
        }
        if picked.is_empty() {
            UserQuestionResponse::Cancelled
        } else {
            UserQuestionResponse::Selected(picked)
        }
    } else if !req.options.is_empty() {
        // Single-select from options.
        let picked_label = q
            .options
            .iter()
            .enumerate()
            .find_map(|(i, opt)| {
                let selected = state
                    .selected_options
                    .first()
                    .and_then(|row| row.get(i).copied())
                    .unwrap_or(false);
                if selected { Some(opt.label.clone()) } else { None }
            });

        if let Some(label) = picked_label {
            UserQuestionResponse::Answer(label)
        } else if state.other_selected.first().copied().unwrap_or(false) {
            let text = state
                .other_text
                .first()
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            if text.is_empty() {
                UserQuestionResponse::Cancelled
            } else {
                UserQuestionResponse::Answer(text)
            }
        } else {
            UserQuestionResponse::Cancelled
        }
    } else {
        // Free-text: only the "Other" field is meaningful.
        let text = state
            .other_text
            .first()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if text.is_empty() {
            UserQuestionResponse::Cancelled
        } else {
            UserQuestionResponse::Answer(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    fn make_request(options: Vec<String>, multi: bool) -> UserQuestionRequest {
        let (tx, _rx) = oneshot::channel();
        UserQuestionRequest {
            id: "t".into(),
            question: "pick one".into(),
            options,
            multi_select: multi,
            response_tx: tx,
        }
    }

    #[test]
    fn free_text_answer_round_trips() {
        let req = make_request(vec![], false);
        let (block, mut state) = to_question_block(&req);
        // Simulate user typing "hello" in the "Other" field.
        state.other_selected[0] = true;
        state.other_text[0] = "hello".into();
        match collect_response(&req, &block, &state) {
            UserQuestionResponse::Answer(s) => assert_eq!(s, "hello"),
            other => panic!("expected Answer, got {:?}", other),
        }
    }

    #[test]
    fn single_select_picks_label() {
        let req = make_request(vec!["red".into(), "blue".into()], false);
        let (block, mut state) = to_question_block(&req);
        state.selected_options[0][1] = true;
        match collect_response(&req, &block, &state) {
            UserQuestionResponse::Answer(s) => assert_eq!(s, "blue"),
            other => panic!("expected Answer, got {:?}", other),
        }
    }

    #[test]
    fn multi_select_picks_labels() {
        let req = make_request(vec!["a".into(), "b".into(), "c".into()], true);
        let (block, mut state) = to_question_block(&req);
        state.selected_options[0][0] = true;
        state.selected_options[0][2] = true;
        match collect_response(&req, &block, &state) {
            UserQuestionResponse::Selected(v) => assert_eq!(v, vec!["a", "c"]),
            other => panic!("expected Selected, got {:?}", other),
        }
    }

    #[test]
    fn nothing_selected_is_cancelled() {
        let req = make_request(vec!["red".into()], false);
        let (block, state) = to_question_block(&req);
        assert!(matches!(
            collect_response(&req, &block, &state),
            UserQuestionResponse::Cancelled
        ));
    }
}
