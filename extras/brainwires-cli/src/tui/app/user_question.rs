//! In-flight state for a one-shot `ask_user_question` tool call.
//!
//! Reuses the existing question panel renderer — see `src/tui/ui/mod.rs`'s
//! `QuestionAnswer` branch — by adapting the incoming request into a
//! synthetic single-question `QuestionBlock`. The tool's response travels
//! back over `response_tx` when the user submits or cancels.

use crate::ask::{UserQuestionRequest, UserQuestionResponse};
use crate::types::question::{QuestionAnswerState, QuestionBlock};

pub struct PendingUserQuestion {
    /// Source request. Retained so `collect_response` knows whether the
    /// caller asked for free-text, single-select, or multi-select.
    pub request: UserQuestionRequest,
    pub block: QuestionBlock,
    pub state: QuestionAnswerState,
}

impl PendingUserQuestion {
    pub fn from_request(request: UserQuestionRequest) -> Self {
        let (block, state) = crate::ask::to_question_block(&request);
        Self {
            request,
            block,
            state,
        }
    }

    /// Drain the current `state` into a response. Does not consume the
    /// struct — caller drops it after sending.
    pub fn collect_response(&self) -> UserQuestionResponse {
        crate::ask::collect_response(&self.request, &self.block, &self.state)
    }

    /// Send a response and consume the request. Returns `false` if the
    /// receiver has been dropped (e.g., the tool call already timed out),
    /// in which case the caller should just discard the pending question.
    pub fn respond(self, response: UserQuestionResponse) -> bool {
        self.request.response_tx.send(response).is_ok()
    }
}
