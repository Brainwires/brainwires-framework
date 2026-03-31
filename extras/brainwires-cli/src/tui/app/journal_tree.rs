//! Journal Tree State
//!
//! Tree data model for the collapsible Journal view.
//! Organises conversation messages and tool executions into a hierarchy:
//!
//!   Turn (top-level, one per user message)
//!   ├── UserMessage
//!   ├── AssistantMessage
//!   │   ├── ToolCall
//!   │   ├── ToolCall
//!   │   └── SubAgentSpawn
//!   │       ├── AssistantMessage (sub-agent)
//!   │       └── ToolCall (sub-agent)
//!   └── SystemEvent

use std::collections::{HashMap, HashSet};

use crate::tui::app::state::{ToolExecutionEntry, TuiMessage};

// ── IDs ──────────────────────────────────────────────────────────────────────

pub type JournalNodeId = u64;

// ── Node Kinds ────────────────────────────────────────────────────────────────

/// What kind of node this is
#[derive(Debug, Clone, PartialEq)]
pub enum JournalNodeKind {
    /// Top-level grouping for a user request and everything it triggered
    Turn,
    /// A user message (leaf)
    UserMessage,
    /// An assistant message (may have ToolCall / SubAgentSpawn children)
    AssistantMessage,
    /// A tool execution (leaf, or parent of SubAgentSpawn)
    ToolCall,
    /// A sub-agent spawn (collapsible; children are the sub-agent's activity)
    SubAgentSpawn,
    /// A system message or status event (leaf)
    SystemEvent,
}

// ── Node Payload ──────────────────────────────────────────────────────────────

/// The data held by each node variant
#[derive(Debug, Clone)]
pub enum JournalNodePayload {
    Message {
        role: String,
        content: String,
    },
    Tool {
        tool_name: String,
        params_summary: String,
        result_summary: String,
        success: bool,
        duration_ms: Option<u64>,
    },
    SubAgentSpawn {
        agent_id: String,
        task_desc: String,
    },
    SystemEvent {
        description: String,
    },
    /// Empty payload for Turn nodes (they are containers only)
    Turn {
        turn_number: usize,
    },
}

// ── Node ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct JournalNode {
    pub id: JournalNodeId,
    pub kind: JournalNodeKind,
    pub timestamp: i64,
    pub payload: JournalNodePayload,
    pub children: Vec<JournalNodeId>,
    pub parent: Option<JournalNodeId>,
}

// ── Render Item ───────────────────────────────────────────────────────────────

/// One entry in the flat DFS render list
#[derive(Debug, Clone)]
pub struct RenderItem {
    pub node_id: JournalNodeId,
    pub depth: usize,
    pub is_last_child: bool,
    /// For each ancestor depth level, whether that ancestor has more siblings below
    /// (used to decide whether to draw `│   ` or `    ` connector lines)
    pub ancestor_has_more: Vec<bool>,
    pub is_collapsed: bool,
    pub has_children: bool,
}

// ── Tree State ────────────────────────────────────────────────────────────────

/// Manages the journal tree, collapsed state, and cached render list
pub struct JournalTreeState {
    /// All nodes indexed by id
    pub nodes: HashMap<JournalNodeId, JournalNode>,
    /// Ordered list of root Turn node IDs
    pub roots: Vec<JournalNodeId>,
    /// Which node IDs are currently collapsed
    pub collapsed: HashSet<JournalNodeId>,
    /// Currently focused/selected node (for keyboard navigation)
    pub cursor: Option<JournalNodeId>,
    /// Cached render list (depth-first traversal, respects collapsed state)
    render_list: Vec<RenderItem>,
    render_list_dirty: bool,
    next_id: JournalNodeId,
    /// Message count at the last full rebuild (for lazy stale detection)
    last_msg_count: usize,
    /// Tool execution count at the last full rebuild
    last_tool_count: usize,
}

impl Default for JournalTreeState {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            roots: Vec::new(),
            collapsed: HashSet::new(),
            cursor: None,
            render_list: Vec::new(),
            render_list_dirty: true,
            next_id: 1,
            last_msg_count: 0,
            last_tool_count: 0,
        }
    }
}

impl JournalTreeState {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Node allocation ───────────────────────────────────────────────────────

    fn alloc_id(&mut self) -> JournalNodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn insert_node(&mut self, node: JournalNode) {
        self.nodes.insert(node.id, node);
        self.render_list_dirty = true;
    }

    // ── Public helpers ────────────────────────────────────────────────────────

    /// Mark the render list as dirty so it will be recomputed on next access.
    pub fn mark_dirty(&mut self) {
        self.render_list_dirty = true;
    }

    // ── Lazy rebuild ─────────────────────────────────────────────────────────

    /// Rebuild the tree only if the source lists have grown since the last rebuild.
    /// Call this once per render frame before accessing the render list.
    pub fn rebuild_if_stale(&mut self, messages: &[TuiMessage], tools: &[ToolExecutionEntry]) {
        if messages.len() != self.last_msg_count || tools.len() != self.last_tool_count {
            self.rebuild_from_flat(messages, tools);
            self.last_msg_count = messages.len();
            self.last_tool_count = tools.len();
        }
    }

    // ── Rebuild from flat lists ───────────────────────────────────────────────

    /// Rebuild the entire tree from the flat message + tool execution lists.
    ///
    /// Called whenever a message or tool execution is appended. This is an
    /// O(n) operation and is fast for typical conversation lengths.
    pub fn rebuild_from_flat(&mut self, messages: &[TuiMessage], tools: &[ToolExecutionEntry]) {
        // Preserve collapsed state and cursor so UX isn't disrupted
        let old_collapsed = self.collapsed.clone();
        let old_cursor = self.cursor;

        // Merge and sort all events by timestamp
        let mut events: Vec<FlatEvent> = Vec::with_capacity(messages.len() + tools.len());
        for msg in messages {
            events.push(FlatEvent::Message(msg.clone()));
        }
        for tool in tools {
            events.push(FlatEvent::Tool(tool.clone()));
        }
        events.sort_by_key(e_timestamp);

        // Clear existing tree
        self.nodes.clear();
        self.roots.clear();
        self.next_id = 1;

        // Build tree
        let mut current_turn_id: Option<JournalNodeId> = None;
        let mut current_assistant_id: Option<JournalNodeId> = None;
        let mut turn_number = 0usize;

        for event in &events {
            match event {
                FlatEvent::Message(msg) => {
                    match msg.role.as_str() {
                        "user" => {
                            // New Turn
                            turn_number += 1;
                            let turn_id = self.alloc_id();
                            let turn_node = JournalNode {
                                id: turn_id,
                                kind: JournalNodeKind::Turn,
                                timestamp: msg.created_at,
                                payload: JournalNodePayload::Turn { turn_number },
                                children: Vec::new(),
                                parent: None,
                            };
                            self.insert_node(turn_node);
                            self.roots.push(turn_id);
                            current_turn_id = Some(turn_id);
                            current_assistant_id = None;

                            // Add UserMessage as child of Turn
                            let msg_id = self.alloc_id();
                            let msg_node = JournalNode {
                                id: msg_id,
                                kind: JournalNodeKind::UserMessage,
                                timestamp: msg.created_at,
                                payload: JournalNodePayload::Message {
                                    role: msg.role.clone(),
                                    content: msg.content.clone(),
                                },
                                children: Vec::new(),
                                parent: Some(turn_id),
                            };
                            self.insert_node(msg_node);
                            if let Some(turn) = self.nodes.get_mut(&turn_id) {
                                turn.children.push(msg_id);
                            }
                        }
                        "assistant" => {
                            // Ensure we have a Turn container
                            let turn_id = current_turn_id.unwrap_or_else(|| {
                                turn_number += 1;
                                let id = self.alloc_id();
                                let node = JournalNode {
                                    id,
                                    kind: JournalNodeKind::Turn,
                                    timestamp: msg.created_at,
                                    payload: JournalNodePayload::Turn { turn_number },
                                    children: Vec::new(),
                                    parent: None,
                                };
                                self.roots.push(id);
                                // insert_node marks dirty — we do it after
                                self.nodes.insert(id, node);
                                id
                            });
                            current_turn_id = Some(turn_id);

                            let msg_id = self.alloc_id();
                            let msg_node = JournalNode {
                                id: msg_id,
                                kind: JournalNodeKind::AssistantMessage,
                                timestamp: msg.created_at,
                                payload: JournalNodePayload::Message {
                                    role: msg.role.clone(),
                                    content: msg.content.clone(),
                                },
                                children: Vec::new(),
                                parent: Some(turn_id),
                            };
                            self.insert_node(msg_node);
                            if let Some(turn) = self.nodes.get_mut(&turn_id) {
                                turn.children.push(msg_id);
                            }
                            current_assistant_id = Some(msg_id);
                        }
                        "system" => {
                            let turn_id = current_turn_id.unwrap_or_else(|| {
                                turn_number += 1;
                                let id = self.alloc_id();
                                let node = JournalNode {
                                    id,
                                    kind: JournalNodeKind::Turn,
                                    timestamp: msg.created_at,
                                    payload: JournalNodePayload::Turn { turn_number },
                                    children: Vec::new(),
                                    parent: None,
                                };
                                self.roots.push(id);
                                self.nodes.insert(id, node);
                                id
                            });
                            current_turn_id = Some(turn_id);

                            let sys_id = self.alloc_id();
                            let sys_node = JournalNode {
                                id: sys_id,
                                kind: JournalNodeKind::SystemEvent,
                                timestamp: msg.created_at,
                                payload: JournalNodePayload::SystemEvent {
                                    description: msg.content.clone(),
                                },
                                children: Vec::new(),
                                parent: Some(turn_id),
                            };
                            self.insert_node(sys_node);
                            if let Some(turn) = self.nodes.get_mut(&turn_id) {
                                turn.children.push(sys_id);
                            }
                        }
                        _ => {}
                    }
                }
                FlatEvent::Tool(tool) => {
                    // Determine parent: prefer current AssistantMessage, fall back to Turn
                    let parent_id = current_assistant_id.or(current_turn_id);

                    if let Some(parent_id) = parent_id {
                        // Detect sub-agent spawn by tool name
                        let is_spawn = tool.tool_name == "agent_spawn";

                        let tool_id = self.alloc_id();
                        let tool_node = JournalNode {
                            id: tool_id,
                            kind: if is_spawn {
                                JournalNodeKind::SubAgentSpawn
                            } else {
                                JournalNodeKind::ToolCall
                            },
                            timestamp: tool.executed_at,
                            payload: if is_spawn {
                                JournalNodePayload::SubAgentSpawn {
                                    agent_id: extract_agent_id(&tool.result_summary),
                                    task_desc: tool.parameters_summary.clone(),
                                }
                            } else {
                                JournalNodePayload::Tool {
                                    tool_name: tool.tool_name.clone(),
                                    params_summary: tool.parameters_summary.clone(),
                                    result_summary: tool.result_summary.clone(),
                                    success: tool.success,
                                    duration_ms: tool.duration_ms,
                                }
                            },
                            children: Vec::new(),
                            parent: Some(parent_id),
                        };
                        self.insert_node(tool_node);
                        if let Some(parent) = self.nodes.get_mut(&parent_id) {
                            parent.children.push(tool_id);
                        }
                    }
                }
            }
        }

        // Restore collapsed state for IDs that still exist
        self.collapsed = old_collapsed
            .into_iter()
            .filter(|id| self.nodes.contains_key(id))
            .collect();

        // Restore or reset cursor
        self.cursor = old_cursor.filter(|id| self.nodes.contains_key(id));

        self.last_msg_count = messages.len();
        self.last_tool_count = tools.len();
        self.render_list_dirty = true;
    }

    // ── Sub-agent activity injection ──────────────────────────────────────────

    /// Inject activity from a sub-agent under the appropriate SubAgentSpawn node.
    ///
    /// Called from the IPC event handler when sub-agent messages arrive.
    pub fn inject_subagent_activity(
        &mut self,
        agent_id: &str,
        messages: &[TuiMessage],
        tools: &[ToolExecutionEntry],
    ) {
        // Find the SubAgentSpawn node for this agent_id
        let spawn_id = self
            .nodes
            .values()
            .find(|n| {
                matches!(
                    &n.payload,
                    JournalNodePayload::SubAgentSpawn { agent_id: aid, .. } if aid == agent_id
                )
            })
            .map(|n| n.id);

        let spawn_id = match spawn_id {
            Some(id) => id,
            None => return, // No spawn node for this agent yet
        };

        // Merge and sort activity
        let mut events: Vec<FlatEvent> = Vec::with_capacity(messages.len() + tools.len());
        for msg in messages {
            events.push(FlatEvent::Message(msg.clone()));
        }
        for tool in tools {
            events.push(FlatEvent::Tool(tool.clone()));
        }
        events.sort_by_key(e_timestamp);

        for event in &events {
            let (kind, payload, ts) = match event {
                FlatEvent::Message(msg) => {
                    let kind = match msg.role.as_str() {
                        "assistant" => JournalNodeKind::AssistantMessage,
                        "user" => JournalNodeKind::UserMessage,
                        _ => JournalNodeKind::SystemEvent,
                    };
                    let payload = if kind == JournalNodeKind::SystemEvent {
                        JournalNodePayload::SystemEvent {
                            description: msg.content.clone(),
                        }
                    } else {
                        JournalNodePayload::Message {
                            role: msg.role.clone(),
                            content: msg.content.clone(),
                        }
                    };
                    (kind, payload, msg.created_at)
                }
                FlatEvent::Tool(tool) => (
                    JournalNodeKind::ToolCall,
                    JournalNodePayload::Tool {
                        tool_name: tool.tool_name.clone(),
                        params_summary: tool.parameters_summary.clone(),
                        result_summary: tool.result_summary.clone(),
                        success: tool.success,
                        duration_ms: tool.duration_ms,
                    },
                    tool.executed_at,
                ),
            };

            let child_id = self.alloc_id();
            let child_node = JournalNode {
                id: child_id,
                kind,
                timestamp: ts,
                payload,
                children: Vec::new(),
                parent: Some(spawn_id),
            };
            self.insert_node(child_node);
            if let Some(spawn) = self.nodes.get_mut(&spawn_id) {
                spawn.children.push(child_id);
            }
        }

        self.render_list_dirty = true;
    }

    // ── Render List ───────────────────────────────────────────────────────────

    /// Recompute the flat DFS render list if dirty
    fn recompute_render_list(&mut self) {
        if !self.render_list_dirty {
            return;
        }
        self.render_list.clear();

        let roots = self.roots.clone();
        let last_root_idx = roots.len().saturating_sub(1);
        for (i, &root_id) in roots.iter().enumerate() {
            self.dfs(root_id, 0, i == last_root_idx, &[]);
        }

        self.render_list_dirty = false;
    }

    fn dfs(&mut self, id: JournalNodeId, depth: usize, is_last: bool, ancestor_has_more: &[bool]) {
        let has_children = self
            .nodes
            .get(&id)
            .map(|n| !n.children.is_empty())
            .unwrap_or(false);
        let is_collapsed = self.collapsed.contains(&id);
        let mut anc = ancestor_has_more.to_vec();
        self.render_list.push(RenderItem {
            node_id: id,
            depth,
            is_last_child: is_last,
            ancestor_has_more: anc.clone(),
            is_collapsed,
            has_children,
        });

        if has_children && !is_collapsed {
            let children = self
                .nodes
                .get(&id)
                .map(|n| n.children.clone())
                .unwrap_or_default();
            let last_idx = children.len().saturating_sub(1);
            // Push whether THIS level has more siblings (for children to use)
            anc.push(!is_last);
            for (ci, &child_id) in children.iter().enumerate() {
                self.dfs(child_id, depth + 1, ci == last_idx, &anc);
            }
        }
    }

    /// Get the current render list (recomputes if dirty)
    pub fn render_list(&mut self) -> &[RenderItem] {
        self.recompute_render_list();
        &self.render_list
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    /// Move cursor to the next item in the render list
    pub fn cursor_next(&mut self) {
        self.recompute_render_list();
        if self.render_list.is_empty() {
            return;
        }
        if let Some(cursor) = self.cursor {
            let pos = self.render_list.iter().position(|r| r.node_id == cursor);
            if let Some(idx) = pos {
                if idx + 1 < self.render_list.len() {
                    self.cursor = Some(self.render_list[idx + 1].node_id);
                }
                return;
            }
        }
        // No cursor or not found — go to first item
        self.cursor = self.render_list.first().map(|r| r.node_id);
    }

    /// Move cursor to the previous item in the render list
    pub fn cursor_prev(&mut self) {
        self.recompute_render_list();
        if self.render_list.is_empty() {
            return;
        }
        if let Some(cursor) = self.cursor {
            let pos = self.render_list.iter().position(|r| r.node_id == cursor);
            if let Some(idx) = pos {
                if idx > 0 {
                    self.cursor = Some(self.render_list[idx - 1].node_id);
                }
                return;
            }
        }
        // No cursor — go to last item
        self.cursor = self.render_list.last().map(|r| r.node_id);
    }

    /// Jump cursor to the first item
    pub fn cursor_first(&mut self) {
        self.recompute_render_list();
        self.cursor = self.render_list.first().map(|r| r.node_id);
    }

    /// Jump cursor to the last item
    pub fn cursor_last(&mut self) {
        self.recompute_render_list();
        self.cursor = self.render_list.last().map(|r| r.node_id);
    }

    /// Get render-list index of the current cursor
    pub fn cursor_render_index(&mut self) -> Option<usize> {
        self.recompute_render_list();
        let cursor = self.cursor?;
        self.render_list.iter().position(|r| r.node_id == cursor)
    }

    // ── Collapse / Expand ─────────────────────────────────────────────────────

    pub fn toggle_collapse(&mut self, id: JournalNodeId) {
        if self.collapsed.contains(&id) {
            self.collapsed.remove(&id);
        } else {
            self.collapsed.insert(id);
        }
        self.render_list_dirty = true;
    }

    pub fn expand(&mut self, id: JournalNodeId) {
        self.collapsed.remove(&id);
        self.render_list_dirty = true;
    }

    pub fn collapse(&mut self, id: JournalNodeId) {
        if let Some(node) = self.nodes.get(&id)
            && !node.children.is_empty()
        {
            self.collapsed.insert(id);
            self.render_list_dirty = true;
        }
    }

    /// Collapse the cursor node; if already collapsed (or leaf), move cursor to parent
    pub fn cursor_collapse_or_parent(&mut self) {
        let cursor = match self.cursor {
            Some(c) => c,
            None => return,
        };
        let has_children = self
            .nodes
            .get(&cursor)
            .map(|n| !n.children.is_empty())
            .unwrap_or(false);
        if has_children && !self.collapsed.contains(&cursor) {
            self.collapse(cursor);
        } else if let Some(parent_id) = self.nodes.get(&cursor).and_then(|n| n.parent) {
            self.cursor = Some(parent_id);
        }
    }

    /// Summary line for a collapsed / leaf node (first ~60 chars of main content)
    pub fn summary_text(node: &JournalNode) -> String {
        match &node.payload {
            JournalNodePayload::Turn { turn_number } => format!("Turn {}", turn_number),
            JournalNodePayload::Message { content, .. } => {
                let first_line = content.lines().next().unwrap_or("").trim().to_string();
                if first_line.len() > 60 {
                    format!("{}…", &first_line[..57])
                } else if first_line.is_empty() {
                    "(empty)".to_string()
                } else {
                    first_line
                }
            }
            JournalNodePayload::Tool {
                tool_name,
                params_summary,
                ..
            } => {
                format!("{} {}", tool_name, params_summary)
            }
            JournalNodePayload::SubAgentSpawn { task_desc, .. } => {
                format!("⚡ {}", task_desc)
            }
            JournalNodePayload::SystemEvent { description } => {
                let first_line = description.lines().next().unwrap_or("").trim().to_string();
                if first_line.len() > 60 {
                    format!("{}…", &first_line[..57])
                } else {
                    first_line
                }
            }
        }
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

enum FlatEvent {
    Message(TuiMessage),
    Tool(ToolExecutionEntry),
}

fn e_timestamp(e: &FlatEvent) -> i64 {
    match e {
        FlatEvent::Message(m) => m.created_at,
        FlatEvent::Tool(t) => t.executed_at,
    }
}

/// Extract agent ID from a tool result summary (best-effort)
fn extract_agent_id(result_summary: &str) -> String {
    // Try to find a pattern like "agent_id: <id>" or just return the first word
    if let Some(rest) = result_summary.strip_prefix("agent_id: ") {
        rest.split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_string()
    } else {
        result_summary
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_string()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(role: &str, content: &str, ts: i64) -> TuiMessage {
        TuiMessage {
            role: role.to_string(),
            content: content.to_string(),
            created_at: ts,
        }
    }

    fn make_tool(name: &str, ts: i64, success: bool) -> ToolExecutionEntry {
        ToolExecutionEntry {
            tool_name: name.to_string(),
            parameters_summary: format!("file: {}.rs", name),
            result_summary: "ok".to_string(),
            success,
            executed_at: ts,
            duration_ms: Some(100),
        }
    }

    #[test]
    fn test_empty_tree() {
        let mut tree = JournalTreeState::new();
        tree.rebuild_from_flat(&[], &[]);
        assert!(tree.roots.is_empty());
        assert!(tree.render_list().is_empty());
    }

    #[test]
    fn test_single_turn() {
        let mut tree = JournalTreeState::new();
        let msgs = vec![
            make_msg("user", "hello", 100),
            make_msg("assistant", "hi there", 200),
        ];
        tree.rebuild_from_flat(&msgs, &[]);

        assert_eq!(tree.roots.len(), 1);
        let turn_id = tree.roots[0];
        let turn = tree.nodes.get(&turn_id).unwrap();
        assert_eq!(turn.kind, JournalNodeKind::Turn);
        // Turn has 2 children: UserMessage, AssistantMessage
        assert_eq!(turn.children.len(), 2);
    }

    #[test]
    fn test_tool_attached_to_assistant() {
        let mut tree = JournalTreeState::new();
        let msgs = vec![
            make_msg("user", "write code", 100),
            make_msg("assistant", "sure", 200),
        ];
        let tools = vec![make_tool("write_file", 250, true)];
        tree.rebuild_from_flat(&msgs, &tools);

        // Find AssistantMessage
        let turn_id = tree.roots[0];
        let turn = tree.nodes.get(&turn_id).unwrap();
        // Turn children: UserMessage + AssistantMessage
        let asst_id = turn.children[1];
        let asst = tree.nodes.get(&asst_id).unwrap();
        assert_eq!(asst.kind, JournalNodeKind::AssistantMessage);
        // Tool is child of assistant
        assert_eq!(asst.children.len(), 1);
        let tool_node = tree.nodes.get(&asst.children[0]).unwrap();
        assert_eq!(tool_node.kind, JournalNodeKind::ToolCall);
    }

    #[test]
    fn test_multiple_turns() {
        let mut tree = JournalTreeState::new();
        let msgs = vec![
            make_msg("user", "first", 100),
            make_msg("assistant", "reply 1", 200),
            make_msg("user", "second", 300),
            make_msg("assistant", "reply 2", 400),
        ];
        tree.rebuild_from_flat(&msgs, &[]);
        assert_eq!(tree.roots.len(), 2);
    }

    #[test]
    fn test_collapse_expand() {
        let mut tree = JournalTreeState::new();
        let msgs = vec![
            make_msg("user", "hello", 100),
            make_msg("assistant", "hi", 200),
        ];
        tree.rebuild_from_flat(&msgs, &[]);
        let turn_id = tree.roots[0];

        // Not collapsed by default
        assert!(!tree.collapsed.contains(&turn_id));

        tree.collapse(turn_id);
        assert!(tree.collapsed.contains(&turn_id));

        tree.expand(turn_id);
        assert!(!tree.collapsed.contains(&turn_id));
    }

    #[test]
    fn test_render_list_respects_collapse() {
        let mut tree = JournalTreeState::new();
        let msgs = vec![
            make_msg("user", "hello", 100),
            make_msg("assistant", "hi", 200),
        ];
        tree.rebuild_from_flat(&msgs, &[]);
        let turn_id = tree.roots[0];

        let full_len = tree.render_list().len();
        assert!(full_len > 1, "Should have Turn + children in render list");

        tree.collapse(turn_id);
        let collapsed_len = tree.render_list().len();
        // Collapsed: only the Turn node itself is visible (no children)
        assert_eq!(collapsed_len, 1);

        tree.expand(turn_id);
        assert_eq!(tree.render_list().len(), full_len);
    }

    #[test]
    fn test_cursor_navigation() {
        let mut tree = JournalTreeState::new();
        let msgs = vec![
            make_msg("user", "hello", 100),
            make_msg("assistant", "hi", 200),
        ];
        tree.rebuild_from_flat(&msgs, &[]);

        tree.cursor_first();
        let first = tree.cursor.unwrap();
        tree.cursor_next();
        let second = tree.cursor.unwrap();
        assert_ne!(first, second);

        tree.cursor_last();
        let last_idx = tree.render_list().len() - 1;
        let last_id = tree.render_list[last_idx].node_id;
        assert_eq!(tree.cursor.unwrap(), last_id);
    }

    #[test]
    fn test_summary_text() {
        let node = JournalNode {
            id: 1,
            kind: JournalNodeKind::Turn,
            timestamp: 0,
            payload: JournalNodePayload::Turn { turn_number: 3 },
            children: vec![],
            parent: None,
        };
        assert_eq!(JournalTreeState::summary_text(&node), "Turn 3");

        let msg_node = JournalNode {
            id: 2,
            kind: JournalNodeKind::UserMessage,
            timestamp: 0,
            payload: JournalNodePayload::Message {
                role: "user".to_string(),
                content: "Hello world".to_string(),
            },
            children: vec![],
            parent: None,
        };
        assert_eq!(JournalTreeState::summary_text(&msg_node), "Hello world");
    }
}
