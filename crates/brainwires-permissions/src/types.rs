//! Core permission system types
//!
//! This module defines the capability-based permission model for agents,
//! including filesystem, tool, network, spawning, git, and quota capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ── Capability Types ─────────────────────────────────────────────────

/// Agent capabilities - explicit permissions granted to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Unique capability set ID for auditing
    #[serde(default = "default_capability_id")]
    pub capability_id: String,

    /// File system capabilities
    #[serde(default)]
    pub filesystem: FilesystemCapabilities,

    /// Tool execution capabilities
    #[serde(default)]
    pub tools: ToolCapabilities,

    /// Network capabilities
    #[serde(default)]
    pub network: NetworkCapabilities,

    /// Agent spawning capabilities
    #[serde(default)]
    pub spawning: SpawningCapabilities,

    /// Git operation capabilities
    #[serde(default)]
    pub git: GitCapabilities,

    /// Resource quota limits
    #[serde(default)]
    pub quotas: ResourceQuotas,
}

fn default_capability_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            capability_id: default_capability_id(),
            filesystem: FilesystemCapabilities::default(),
            tools: ToolCapabilities::default(),
            network: NetworkCapabilities::default(),
            spawning: SpawningCapabilities::default(),
            git: GitCapabilities::default(),
            quotas: ResourceQuotas::default(),
        }
    }
}

impl AgentCapabilities {
    /// Check if a tool is allowed by the current capabilities
    pub fn allows_tool(&self, tool_name: &str) -> bool {
        // Check explicit deny list first
        if self.tools.denied_tools.contains(tool_name) {
            return false;
        }

        // Check explicit allow list if specified
        if let Some(ref allowed) = self.tools.allowed_tools {
            return allowed.contains(tool_name);
        }

        // Fall back to category-based check
        let category = Self::categorize_tool(tool_name);
        self.tools.allowed_categories.contains(&category)
    }

    /// Check if a tool requires explicit approval
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        self.tools.always_approve.contains(tool_name)
    }

    /// Categorize a tool by name into a ToolCategory
    pub fn categorize_tool(tool_name: &str) -> ToolCategory {
        match tool_name {
            // File read operations
            "read_file" | "list_directory" | "search_files" => ToolCategory::FileRead,

            // File write operations
            "write_file" | "edit_file" | "patch_file" | "delete_file" | "create_directory" => {
                ToolCategory::FileWrite
            }

            // Search operations
            "search_code" | "index_codebase" | "query_codebase" | "search_with_filters"
            | "get_rag_statistics" | "clear_rag_index" | "search_git_history" => {
                ToolCategory::Search
            }

            // Git operations - check for destructive operations first
            name if name.starts_with("git_") => {
                if name.contains("force")
                    || name.contains("reset")
                    || name.contains("rebase")
                    || name.contains("delete_branch")
                {
                    ToolCategory::GitDestructive
                } else {
                    ToolCategory::Git
                }
            }

            // Bash/shell operations
            "execute_command" => ToolCategory::Bash,

            // Web operations
            "fetch_url" | "web_search" | "web_browse" | "web_scrape" => ToolCategory::Web,

            // Code execution
            "execute_code" | "execute_script" => ToolCategory::CodeExecution,

            // Agent operations
            "agent_spawn" | "agent_stop" | "agent_status" | "agent_list" | "agent_pool_stats"
            | "agent_file_locks" => ToolCategory::AgentSpawn,

            // Planning/task operations
            "plan_task" | "task_create" | "task_add_subtask" | "task_start" | "task_complete"
            | "task_fail" | "task_list" | "task_get" => ToolCategory::Planning,

            // MCP tools
            name if name.starts_with("mcp_") => ToolCategory::System,

            // Context operations
            "recall_context" | "search_tools" => ToolCategory::Search,

            // Default to System for unknown tools
            _ => ToolCategory::System,
        }
    }

    /// Check if a file path is allowed for reading
    pub fn allows_read(&self, path: &str) -> bool {
        // Check denied paths first
        for denied in &self.filesystem.denied_paths {
            if denied.matches(path) {
                return false;
            }
        }

        // Check if any read path matches
        for allowed in &self.filesystem.read_paths {
            if allowed.matches(path) {
                return true;
            }
        }

        false
    }

    /// Check if a file path is allowed for writing
    pub fn allows_write(&self, path: &str) -> bool {
        // Check denied paths first
        for denied in &self.filesystem.denied_paths {
            if denied.matches(path) {
                return false;
            }
        }

        // Check if any write path matches
        for allowed in &self.filesystem.write_paths {
            if allowed.matches(path) {
                return true;
            }
        }

        false
    }

    /// Check if a domain is allowed for network access
    pub fn allows_domain(&self, domain: &str) -> bool {
        // Check denied domains first
        for denied in &self.network.denied_domains {
            if Self::domain_matches(denied, domain) {
                return false;
            }
        }

        // If allow_all is set, allow everything not denied
        if self.network.allow_all {
            return true;
        }

        // Check allowed domains
        for allowed in &self.network.allowed_domains {
            if Self::domain_matches(allowed, domain) {
                return true;
            }
        }

        false
    }

    /// Check if a git operation is allowed
    pub fn allows_git_op(&self, op: GitOperation) -> bool {
        // Check for destructive operations
        if op.is_destructive() && !self.git.can_destructive {
            return false;
        }

        // Check force push
        if op == GitOperation::ForcePush && !self.git.can_force_push {
            return false;
        }

        self.git.allowed_ops.contains(&op)
    }

    /// Check if spawning agents is allowed
    pub fn can_spawn_agent(&self, current_children: u32, current_depth: u32) -> bool {
        if !self.spawning.can_spawn {
            return false;
        }

        if current_children >= self.spawning.max_children {
            return false;
        }

        if current_depth >= self.spawning.max_depth {
            return false;
        }

        true
    }

    /// Simple domain matching with wildcard support
    fn domain_matches(pattern: &str, domain: &str) -> bool {
        if pattern.starts_with("*.") {
            let suffix = &pattern[1..]; // Keep the dot
            domain.ends_with(suffix) || domain == &pattern[2..]
        } else {
            pattern == domain
        }
    }
}

// ── Capability Profiles ──────────────────────────────────────────────

/// Capability profile names
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityProfile {
    /// Read-only exploration - safe for untrusted agents
    ReadOnly,
    /// Standard development - balanced safety and utility
    StandardDev,
    /// Full access - for trusted orchestrators
    FullAccess,
    /// Custom profile loaded from config
    Custom,
}

impl CapabilityProfile {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "read_only" | "readonly" | "read-only" => Some(Self::ReadOnly),
            "standard_dev" | "standarddev" | "standard-dev" | "standard" => Some(Self::StandardDev),
            "full_access" | "fullaccess" | "full-access" | "full" => Some(Self::FullAccess),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::StandardDev => "standard_dev",
            Self::FullAccess => "full_access",
            Self::Custom => "custom",
        }
    }
}

impl AgentCapabilities {
    /// Read-only exploration - safe for untrusted agents
    ///
    /// This profile allows:
    /// - Reading all files (except secrets)
    /// - Search operations
    /// - Read-only git operations
    /// - No network access
    /// - No spawning
    /// - Conservative quotas
    pub fn read_only() -> Self {
        Self {
            capability_id: uuid::Uuid::new_v4().to_string(),
            filesystem: FilesystemCapabilities {
                read_paths: vec![PathPattern::new("**/*")],
                write_paths: vec![],
                denied_paths: vec![
                    PathPattern::new("**/.env*"),
                    PathPattern::new("**/*credentials*"),
                    PathPattern::new("**/*secret*"),
                    PathPattern::new("**/*.pem"),
                    PathPattern::new("**/*.key"),
                ],
                follow_symlinks: false,
                access_hidden: false,
                can_delete: false,
                can_create_dirs: false,
                max_write_size: None,
            },
            tools: ToolCapabilities {
                allowed_categories: {
                    let mut cats = HashSet::new();
                    cats.insert(ToolCategory::FileRead);
                    cats.insert(ToolCategory::Search);
                    cats
                },
                denied_tools: HashSet::new(),
                allowed_tools: None,
                always_approve: HashSet::new(),
            },
            network: NetworkCapabilities::disabled(),
            spawning: SpawningCapabilities::disabled(),
            git: GitCapabilities::read_only(),
            quotas: ResourceQuotas::conservative(),
        }
    }

    /// Standard development - balanced safety and utility
    ///
    /// This profile allows:
    /// - Reading all files (except secrets)
    /// - Writing to src/, tests/, docs/
    /// - File read/write, search, git, planning tools
    /// - Network access to common dev domains
    /// - Limited agent spawning
    /// - Standard quotas
    pub fn standard_dev() -> Self {
        Self {
            capability_id: uuid::Uuid::new_v4().to_string(),
            filesystem: FilesystemCapabilities {
                read_paths: vec![PathPattern::new("**/*")],
                write_paths: vec![
                    PathPattern::new("src/**"),
                    PathPattern::new("tests/**"),
                    PathPattern::new("docs/**"),
                    PathPattern::new("scripts/**"),
                    PathPattern::new("*.toml"),
                    PathPattern::new("*.json"),
                    PathPattern::new("*.yaml"),
                    PathPattern::new("*.yml"),
                    PathPattern::new("*.md"),
                    PathPattern::new("Makefile"),
                    PathPattern::new(".gitignore"),
                ],
                denied_paths: vec![
                    PathPattern::new("**/.env*"),
                    PathPattern::new("**/*credentials*"),
                    PathPattern::new("**/*secret*"),
                    PathPattern::new("**/node_modules/**"),
                    PathPattern::new("**/target/**"),
                    PathPattern::new("**/.git/**"),
                ],
                follow_symlinks: true,
                access_hidden: true,
                can_delete: true,
                can_create_dirs: true,
                max_write_size: Some(1024 * 1024), // 1MB
            },
            tools: ToolCapabilities {
                allowed_categories: {
                    let mut cats = HashSet::new();
                    cats.insert(ToolCategory::FileRead);
                    cats.insert(ToolCategory::FileWrite);
                    cats.insert(ToolCategory::Search);
                    cats.insert(ToolCategory::Git);
                    cats.insert(ToolCategory::Planning);
                    cats.insert(ToolCategory::Web);
                    cats
                },
                denied_tools: {
                    let mut denied = HashSet::new();
                    denied.insert("execute_code".to_string());
                    denied
                },
                allowed_tools: None,
                always_approve: {
                    let mut approve = HashSet::new();
                    approve.insert("delete_file".to_string());
                    approve.insert("execute_command".to_string());
                    approve
                },
            },
            network: NetworkCapabilities {
                allowed_domains: vec![
                    "github.com".to_string(),
                    "*.github.com".to_string(),
                    "docs.rs".to_string(),
                    "crates.io".to_string(),
                    "npmjs.com".to_string(),
                    "*.npmjs.com".to_string(),
                    "pypi.org".to_string(),
                    "stackoverflow.com".to_string(),
                ],
                denied_domains: vec![],
                allow_all: false,
                rate_limit: Some(60),
                allow_api_calls: true,
                max_response_size: Some(10 * 1024 * 1024), // 10MB
            },
            spawning: SpawningCapabilities {
                can_spawn: true,
                max_children: 3,
                max_depth: 2,
                can_elevate: false,
            },
            git: GitCapabilities::standard(),
            quotas: ResourceQuotas::standard(),
        }
    }

    /// Full access - for trusted orchestrators
    ///
    /// This profile allows:
    /// - Full filesystem access
    /// - All tools including bash and code execution
    /// - Full network access
    /// - Full spawning capabilities
    /// - Generous quotas
    pub fn full_access() -> Self {
        Self {
            capability_id: uuid::Uuid::new_v4().to_string(),
            filesystem: FilesystemCapabilities::full(),
            tools: ToolCapabilities::full(),
            network: NetworkCapabilities::full(),
            spawning: SpawningCapabilities::full(),
            git: GitCapabilities::full(),
            quotas: ResourceQuotas::generous(),
        }
    }

    /// Create capabilities from a profile name
    pub fn from_profile(profile: CapabilityProfile) -> Self {
        match profile {
            CapabilityProfile::ReadOnly => Self::read_only(),
            CapabilityProfile::StandardDev => Self::standard_dev(),
            CapabilityProfile::FullAccess => Self::full_access(),
            CapabilityProfile::Custom => Self::default(),
        }
    }

    /// Create a child capability set that is a subset of the parent
    ///
    /// Child capabilities can never exceed parent capabilities.
    pub fn derive_child(&self) -> Self {
        // Child inherits parent capabilities but with reduced spawning depth
        let mut child = self.clone();
        child.capability_id = uuid::Uuid::new_v4().to_string();

        // Reduce spawning depth
        if child.spawning.max_depth > 0 {
            child.spawning.max_depth -= 1;
        }

        // Disable elevation for children
        child.spawning.can_elevate = false;

        child
    }

    /// Merge capabilities, taking the more restrictive option for each field
    pub fn intersect(&self, other: &Self) -> Self {
        Self {
            capability_id: uuid::Uuid::new_v4().to_string(),
            filesystem: FilesystemCapabilities {
                // Intersection of allowed paths
                read_paths: self
                    .filesystem
                    .read_paths
                    .iter()
                    .filter(|p| other.filesystem.read_paths.iter().any(|op| op.pattern() == p.pattern()))
                    .cloned()
                    .collect(),
                write_paths: self
                    .filesystem
                    .write_paths
                    .iter()
                    .filter(|p| other.filesystem.write_paths.iter().any(|op| op.pattern() == p.pattern()))
                    .cloned()
                    .collect(),
                // Union of denied paths (more restrictive)
                denied_paths: {
                    let mut denied = self.filesystem.denied_paths.clone();
                    for p in &other.filesystem.denied_paths {
                        if !denied.iter().any(|dp| dp.pattern() == p.pattern()) {
                            denied.push(p.clone());
                        }
                    }
                    denied
                },
                follow_symlinks: self.filesystem.follow_symlinks && other.filesystem.follow_symlinks,
                access_hidden: self.filesystem.access_hidden && other.filesystem.access_hidden,
                can_delete: self.filesystem.can_delete && other.filesystem.can_delete,
                can_create_dirs: self.filesystem.can_create_dirs && other.filesystem.can_create_dirs,
                max_write_size: match (self.filesystem.max_write_size, other.filesystem.max_write_size) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                },
            },
            tools: ToolCapabilities {
                // Intersection of allowed categories
                allowed_categories: self
                    .tools
                    .allowed_categories
                    .intersection(&other.tools.allowed_categories)
                    .cloned()
                    .collect(),
                // Union of denied tools
                denied_tools: self
                    .tools
                    .denied_tools
                    .union(&other.tools.denied_tools)
                    .cloned()
                    .collect(),
                allowed_tools: match (&self.tools.allowed_tools, &other.tools.allowed_tools) {
                    (Some(a), Some(b)) => Some(a.intersection(b).cloned().collect()),
                    (Some(a), None) => Some(a.clone()),
                    (None, Some(b)) => Some(b.clone()),
                    (None, None) => None,
                },
                // Union of tools requiring approval
                always_approve: self
                    .tools
                    .always_approve
                    .union(&other.tools.always_approve)
                    .cloned()
                    .collect(),
            },
            network: NetworkCapabilities {
                allowed_domains: self
                    .network
                    .allowed_domains
                    .iter()
                    .filter(|d| other.network.allowed_domains.contains(d) || other.network.allow_all)
                    .cloned()
                    .collect(),
                denied_domains: {
                    let mut denied = self.network.denied_domains.clone();
                    denied.extend(other.network.denied_domains.iter().cloned());
                    denied.sort();
                    denied.dedup();
                    denied
                },
                allow_all: self.network.allow_all && other.network.allow_all,
                rate_limit: match (self.network.rate_limit, other.network.rate_limit) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                },
                allow_api_calls: self.network.allow_api_calls && other.network.allow_api_calls,
                max_response_size: match (self.network.max_response_size, other.network.max_response_size) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                },
            },
            spawning: SpawningCapabilities {
                can_spawn: self.spawning.can_spawn && other.spawning.can_spawn,
                max_children: self.spawning.max_children.min(other.spawning.max_children),
                max_depth: self.spawning.max_depth.min(other.spawning.max_depth),
                can_elevate: self.spawning.can_elevate && other.spawning.can_elevate,
            },
            git: GitCapabilities {
                allowed_ops: self
                    .git
                    .allowed_ops
                    .intersection(&other.git.allowed_ops)
                    .cloned()
                    .collect(),
                protected_branches: {
                    let mut branches = self.git.protected_branches.clone();
                    branches.extend(other.git.protected_branches.iter().cloned());
                    branches.sort();
                    branches.dedup();
                    branches
                },
                can_force_push: self.git.can_force_push && other.git.can_force_push,
                can_destructive: self.git.can_destructive && other.git.can_destructive,
                require_pr_branches: {
                    let mut branches = self.git.require_pr_branches.clone();
                    branches.extend(other.git.require_pr_branches.iter().cloned());
                    branches.sort();
                    branches.dedup();
                    branches
                },
            },
            quotas: ResourceQuotas {
                max_execution_time: match (self.quotas.max_execution_time, other.quotas.max_execution_time) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                },
                max_memory: match (self.quotas.max_memory, other.quotas.max_memory) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                },
                max_tokens: match (self.quotas.max_tokens, other.quotas.max_tokens) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                },
                max_tool_calls: match (self.quotas.max_tool_calls, other.quotas.max_tool_calls) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                },
                max_files_modified: match (self.quotas.max_files_modified, other.quotas.max_files_modified) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                },
            },
        }
    }
}

// ── Filesystem Capabilities ──────────────────────────────────────────

/// File system capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemCapabilities {
    /// Allowed read paths (glob patterns)
    #[serde(default = "default_read_paths")]
    pub read_paths: Vec<PathPattern>,

    /// Allowed write paths (glob patterns)
    #[serde(default)]
    pub write_paths: Vec<PathPattern>,

    /// Denied paths (override allows)
    #[serde(default = "default_denied_paths")]
    pub denied_paths: Vec<PathPattern>,

    /// Can follow symlinks outside allowed paths
    #[serde(default = "default_true")]
    pub follow_symlinks: bool,

    /// Can access hidden files (dotfiles)
    #[serde(default = "default_true")]
    pub access_hidden: bool,

    /// Maximum file size for write operations (bytes)
    #[serde(default)]
    pub max_write_size: Option<u64>,

    /// Can delete files
    #[serde(default)]
    pub can_delete: bool,

    /// Can create directories
    #[serde(default = "default_true")]
    pub can_create_dirs: bool,
}

fn default_read_paths() -> Vec<PathPattern> {
    vec![PathPattern::new("**/*")]
}

fn default_denied_paths() -> Vec<PathPattern> {
    vec![
        PathPattern::new("**/.env*"),
        PathPattern::new("**/*credentials*"),
        PathPattern::new("**/*secret*"),
    ]
}

fn default_true() -> bool {
    true
}

impl Default for FilesystemCapabilities {
    fn default() -> Self {
        Self {
            read_paths: default_read_paths(),
            write_paths: Vec::new(),
            denied_paths: default_denied_paths(),
            follow_symlinks: true,
            access_hidden: true,
            max_write_size: None,
            can_delete: false,
            can_create_dirs: true,
        }
    }
}

impl FilesystemCapabilities {
    /// Create full access filesystem capabilities
    pub fn full() -> Self {
        Self {
            read_paths: vec![PathPattern::new("**/*")],
            write_paths: vec![PathPattern::new("**/*")],
            denied_paths: Vec::new(),
            follow_symlinks: true,
            access_hidden: true,
            max_write_size: None,
            can_delete: true,
            can_create_dirs: true,
        }
    }
}

// ── Tool Capabilities ────────────────────────────────────────────────

/// Tool execution capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapabilities {
    /// Tool categories allowed
    #[serde(default = "default_allowed_categories")]
    pub allowed_categories: HashSet<ToolCategory>,

    /// Specific tools denied (overrides category allows)
    #[serde(default)]
    pub denied_tools: HashSet<String>,

    /// Specific tools allowed (if not using categories)
    #[serde(default)]
    pub allowed_tools: Option<HashSet<String>>,

    /// Require approval for these tools regardless of trust
    #[serde(default)]
    pub always_approve: HashSet<String>,
}

fn default_allowed_categories() -> HashSet<ToolCategory> {
    let mut set = HashSet::new();
    set.insert(ToolCategory::FileRead);
    set.insert(ToolCategory::Search);
    set.insert(ToolCategory::Web);
    set
}

impl Default for ToolCapabilities {
    fn default() -> Self {
        Self {
            allowed_categories: default_allowed_categories(),
            denied_tools: HashSet::new(),
            allowed_tools: None,
            always_approve: HashSet::new(),
        }
    }
}

impl ToolCapabilities {
    /// Create full access tool capabilities
    pub fn full() -> Self {
        let mut categories = HashSet::new();
        categories.insert(ToolCategory::FileRead);
        categories.insert(ToolCategory::FileWrite);
        categories.insert(ToolCategory::Search);
        categories.insert(ToolCategory::Git);
        categories.insert(ToolCategory::GitDestructive);
        categories.insert(ToolCategory::Bash);
        categories.insert(ToolCategory::Web);
        categories.insert(ToolCategory::CodeExecution);
        categories.insert(ToolCategory::AgentSpawn);
        categories.insert(ToolCategory::Planning);
        categories.insert(ToolCategory::System);

        Self {
            allowed_categories: categories,
            denied_tools: HashSet::new(),
            allowed_tools: None,
            always_approve: HashSet::new(),
        }
    }
}

/// Tool categories for permission grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCategory {
    /// Read file operations: read_file, list_directory, search_files
    FileRead,
    /// Write file operations: write_file, edit_file, patch_file, delete_file
    FileWrite,
    /// Search operations: search_code, semantic search, RAG
    Search,
    /// Git operations: status, diff, log, add, commit, push, pull
    Git,
    /// Destructive git operations: force push, hard reset, rebase
    GitDestructive,
    /// Shell command execution
    Bash,
    /// Web operations: fetch_url, web_search, web_scrape
    Web,
    /// Code execution in sandboxed environment
    CodeExecution,
    /// Agent spawning and management
    AgentSpawn,
    /// Planning and task management
    Planning,
    /// System-level operations
    System,
}

// ── Network Capabilities ─────────────────────────────────────────────

/// Network capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCapabilities {
    /// Allowed domains (supports wildcards like *.github.com)
    #[serde(default)]
    pub allowed_domains: Vec<String>,

    /// Denied domains (override allows)
    #[serde(default)]
    pub denied_domains: Vec<String>,

    /// Allow all domains (use with caution)
    #[serde(default)]
    pub allow_all: bool,

    /// Rate limit (requests per minute)
    #[serde(default)]
    pub rate_limit: Option<u32>,

    /// Can make external API calls
    #[serde(default)]
    pub allow_api_calls: bool,

    /// Maximum response size to process (bytes)
    #[serde(default)]
    pub max_response_size: Option<u64>,
}

impl Default for NetworkCapabilities {
    fn default() -> Self {
        Self {
            allowed_domains: Vec::new(),
            denied_domains: Vec::new(),
            allow_all: false,
            rate_limit: Some(60),
            allow_api_calls: false,
            max_response_size: Some(10 * 1024 * 1024), // 10MB
        }
    }
}

impl NetworkCapabilities {
    /// Create disabled network capabilities
    pub fn disabled() -> Self {
        Self {
            allowed_domains: Vec::new(),
            denied_domains: Vec::new(),
            allow_all: false,
            rate_limit: Some(0),
            allow_api_calls: false,
            max_response_size: None,
        }
    }

    /// Create full network capabilities
    pub fn full() -> Self {
        Self {
            allowed_domains: Vec::new(),
            denied_domains: Vec::new(),
            allow_all: true,
            rate_limit: None,
            allow_api_calls: true,
            max_response_size: None,
        }
    }
}

// ── Spawning Capabilities ────────────────────────────────────────────

/// Agent spawning capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawningCapabilities {
    /// Can spawn child agents
    #[serde(default)]
    pub can_spawn: bool,

    /// Maximum concurrent child agents
    #[serde(default = "default_max_children")]
    pub max_children: u32,

    /// Maximum depth of agent hierarchy
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,

    /// Can spawn agents with elevated privileges (requires approval)
    #[serde(default)]
    pub can_elevate: bool,
}

fn default_max_children() -> u32 {
    3
}

fn default_max_depth() -> u32 {
    2
}

impl Default for SpawningCapabilities {
    fn default() -> Self {
        Self {
            can_spawn: false,
            max_children: 3,
            max_depth: 2,
            can_elevate: false,
        }
    }
}

impl SpawningCapabilities {
    /// Create disabled spawning capabilities
    pub fn disabled() -> Self {
        Self {
            can_spawn: false,
            max_children: 0,
            max_depth: 0,
            can_elevate: false,
        }
    }

    /// Create full spawning capabilities
    pub fn full() -> Self {
        Self {
            can_spawn: true,
            max_children: 10,
            max_depth: 5,
            can_elevate: true,
        }
    }
}

// ── Git Capabilities ─────────────────────────────────────────────────

/// Git operation capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCapabilities {
    /// Allowed operations
    #[serde(default = "default_git_ops")]
    pub allowed_ops: HashSet<GitOperation>,

    /// Protected branches (cannot push directly)
    #[serde(default)]
    pub protected_branches: Vec<String>,

    /// Can force push (dangerous)
    #[serde(default)]
    pub can_force_push: bool,

    /// Can perform destructive operations
    #[serde(default)]
    pub can_destructive: bool,

    /// Require PR for these branches
    #[serde(default)]
    pub require_pr_branches: Vec<String>,
}

fn default_git_ops() -> HashSet<GitOperation> {
    let mut ops = HashSet::new();
    ops.insert(GitOperation::Status);
    ops.insert(GitOperation::Diff);
    ops.insert(GitOperation::Log);
    ops
}

impl Default for GitCapabilities {
    fn default() -> Self {
        Self {
            allowed_ops: default_git_ops(),
            protected_branches: vec!["main".to_string(), "master".to_string()],
            can_force_push: false,
            can_destructive: false,
            require_pr_branches: Vec::new(),
        }
    }
}

impl GitCapabilities {
    /// Create read-only git capabilities
    pub fn read_only() -> Self {
        let mut ops = HashSet::new();
        ops.insert(GitOperation::Status);
        ops.insert(GitOperation::Diff);
        ops.insert(GitOperation::Log);
        ops.insert(GitOperation::Fetch);

        Self {
            allowed_ops: ops,
            protected_branches: vec!["main".to_string(), "master".to_string()],
            can_force_push: false,
            can_destructive: false,
            require_pr_branches: Vec::new(),
        }
    }

    /// Create standard git capabilities
    pub fn standard() -> Self {
        let mut ops = HashSet::new();
        ops.insert(GitOperation::Status);
        ops.insert(GitOperation::Diff);
        ops.insert(GitOperation::Log);
        ops.insert(GitOperation::Add);
        ops.insert(GitOperation::Commit);
        ops.insert(GitOperation::Push);
        ops.insert(GitOperation::Pull);
        ops.insert(GitOperation::Fetch);
        ops.insert(GitOperation::Branch);
        ops.insert(GitOperation::Checkout);
        ops.insert(GitOperation::Stash);

        Self {
            allowed_ops: ops,
            protected_branches: vec!["main".to_string(), "master".to_string()],
            can_force_push: false,
            can_destructive: false,
            require_pr_branches: Vec::new(),
        }
    }

    /// Create full git capabilities
    pub fn full() -> Self {
        let mut ops = HashSet::new();
        ops.insert(GitOperation::Status);
        ops.insert(GitOperation::Diff);
        ops.insert(GitOperation::Log);
        ops.insert(GitOperation::Add);
        ops.insert(GitOperation::Commit);
        ops.insert(GitOperation::Push);
        ops.insert(GitOperation::Pull);
        ops.insert(GitOperation::Fetch);
        ops.insert(GitOperation::Branch);
        ops.insert(GitOperation::Checkout);
        ops.insert(GitOperation::Merge);
        ops.insert(GitOperation::Rebase);
        ops.insert(GitOperation::Reset);
        ops.insert(GitOperation::Stash);
        ops.insert(GitOperation::Tag);
        ops.insert(GitOperation::ForcePush);

        Self {
            allowed_ops: ops,
            protected_branches: Vec::new(),
            can_force_push: true,
            can_destructive: true,
            require_pr_branches: Vec::new(),
        }
    }
}

/// Git operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GitOperation {
    /// View working tree status.
    Status,
    /// Show changes between commits.
    Diff,
    /// View commit history.
    Log,
    /// Stage changes.
    Add,
    /// Create a commit.
    Commit,
    /// Push to remote.
    Push,
    /// Pull from remote.
    Pull,
    /// Fetch from remote.
    Fetch,
    /// Branch operations.
    Branch,
    /// Switch branches.
    Checkout,
    /// Merge branches.
    Merge,
    /// Rebase commits.
    Rebase,
    /// Reset to a previous state.
    Reset,
    /// Stash changes.
    Stash,
    /// Tag a commit.
    Tag,
    /// Force push to remote.
    ForcePush,
}

impl GitOperation {
    /// Check if this operation is destructive
    pub fn is_destructive(&self) -> bool {
        matches!(
            self,
            GitOperation::Rebase
                | GitOperation::Reset
                | GitOperation::ForcePush
                | GitOperation::Merge
        )
    }
}

// ── Resource Quotas ──────────────────────────────────────────────────

/// Resource quota limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuotas {
    /// Maximum execution time (seconds)
    #[serde(default)]
    pub max_execution_time: Option<u64>,

    /// Maximum memory usage (bytes)
    #[serde(default)]
    pub max_memory: Option<u64>,

    /// Maximum API tokens consumed
    #[serde(default)]
    pub max_tokens: Option<u64>,

    /// Maximum tool calls per session
    #[serde(default)]
    pub max_tool_calls: Option<u32>,

    /// Maximum files modified per session
    #[serde(default)]
    pub max_files_modified: Option<u32>,
}

impl Default for ResourceQuotas {
    fn default() -> Self {
        Self {
            max_execution_time: Some(30 * 60), // 30 minutes
            max_memory: None,
            max_tokens: Some(100_000),
            max_tool_calls: Some(500),
            max_files_modified: Some(50),
        }
    }
}

impl ResourceQuotas {
    /// Create conservative quotas
    pub fn conservative() -> Self {
        Self {
            max_execution_time: Some(5 * 60), // 5 minutes
            max_memory: Some(512 * 1024 * 1024), // 512MB
            max_tokens: Some(10_000),
            max_tool_calls: Some(50),
            max_files_modified: Some(10),
        }
    }

    /// Create standard quotas
    pub fn standard() -> Self {
        Self::default()
    }

    /// Create generous quotas
    pub fn generous() -> Self {
        Self {
            max_execution_time: Some(2 * 60 * 60), // 2 hours
            max_memory: None,
            max_tokens: Some(500_000),
            max_tool_calls: Some(2000),
            max_files_modified: Some(200),
        }
    }
}

// ── Path Pattern ─────────────────────────────────────────────────────

/// Path pattern for glob matching
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PathPattern {
    pattern: String,
}

impl PathPattern {
    /// Create a new path pattern
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
        }
    }

    /// Create a glob pattern
    pub fn glob(pattern: &str) -> Self {
        Self::new(pattern)
    }

    /// Check if a path matches this pattern
    #[cfg(feature = "native")]
    pub fn matches(&self, path: &str) -> bool {
        // Use glob matching
        if let Ok(pattern) = glob::Pattern::new(&self.pattern) {
            pattern.matches(path) || pattern.matches_path(std::path::Path::new(path))
        } else {
            // Fall back to simple string matching if pattern is invalid
            path.contains(&self.pattern)
        }
    }

    /// Check if a path matches this pattern (simple string matching for WASM)
    #[cfg(not(feature = "native"))]
    pub fn matches(&self, path: &str) -> bool {
        path.contains(&self.pattern)
    }

    /// Get the pattern string
    pub fn pattern(&self) -> &str {
        &self.pattern
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_pattern_matching() {
        let pattern = PathPattern::new("**/.env*");
        assert!(pattern.matches(".env"));
        assert!(pattern.matches(".env.local"));
        assert!(pattern.matches("config/.env"));

        let pattern = PathPattern::new("src/**/*.rs");
        assert!(pattern.matches("src/main.rs"));
        assert!(pattern.matches("src/lib/mod.rs"));
    }

    #[test]
    fn test_full_access_pattern() {
        let pattern = PathPattern::new("**/*");
        assert!(pattern.matches("index.html"), "**/* should match root files");
        assert!(pattern.matches("./index.html"), "**/* should match ./file");
        assert!(pattern.matches("src/main.rs"), "**/* should match nested files");
    }

    #[test]
    fn test_tool_categorization() {
        assert_eq!(
            AgentCapabilities::categorize_tool("read_file"),
            ToolCategory::FileRead
        );
        assert_eq!(
            AgentCapabilities::categorize_tool("write_file"),
            ToolCategory::FileWrite
        );
        assert_eq!(
            AgentCapabilities::categorize_tool("git_status"),
            ToolCategory::Git
        );
        assert_eq!(
            AgentCapabilities::categorize_tool("git_force_push"),
            ToolCategory::GitDestructive
        );
        assert_eq!(
            AgentCapabilities::categorize_tool("execute_command"),
            ToolCategory::Bash
        );
    }

    #[test]
    fn test_allows_tool() {
        let caps = AgentCapabilities::default();

        // Default only allows FileRead, Search, and Web
        assert!(caps.allows_tool("read_file"));
        assert!(caps.allows_tool("search_code"));
        assert!(!caps.allows_tool("write_file"));
        assert!(!caps.allows_tool("execute_command"));
    }

    #[test]
    fn test_denied_tools() {
        let mut caps = AgentCapabilities::default();
        caps.tools.denied_tools.insert("read_file".to_string());

        // Even though FileRead is allowed, this specific tool is denied
        assert!(!caps.allows_tool("read_file"));
        assert!(caps.allows_tool("list_directory")); // Other FileRead tools still work
    }

    #[test]
    fn test_domain_matching() {
        let caps = AgentCapabilities {
            network: NetworkCapabilities {
                allowed_domains: vec![
                    "github.com".to_string(),
                    "*.github.com".to_string(),
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(caps.allows_domain("github.com"));
        assert!(caps.allows_domain("api.github.com"));
        assert!(caps.allows_domain("raw.github.com"));
        assert!(!caps.allows_domain("gitlab.com"));
    }

    #[test]
    fn test_git_operations() {
        let caps = AgentCapabilities::default();

        // Default allows read-only git ops
        assert!(caps.allows_git_op(GitOperation::Status));
        assert!(caps.allows_git_op(GitOperation::Diff));
        assert!(!caps.allows_git_op(GitOperation::Push));
        assert!(!caps.allows_git_op(GitOperation::ForcePush));
    }

    #[test]
    fn test_read_only_profile() {
        let caps = AgentCapabilities::read_only();

        assert!(caps.allows_tool("read_file"));
        assert!(caps.allows_tool("search_code"));
        assert!(!caps.allows_tool("write_file"));
        assert!(!caps.allows_tool("execute_command"));
        assert!(!caps.allows_domain("github.com"));
        assert!(!caps.can_spawn_agent(0, 0));
    }

    #[test]
    fn test_standard_dev_profile() {
        let caps = AgentCapabilities::standard_dev();

        assert!(caps.allows_tool("read_file"));
        assert!(caps.allows_tool("write_file"));
        assert!(caps.allows_tool("git_status"));
        assert!(!caps.allows_tool("execute_code"));
        assert!(caps.requires_approval("delete_file"));
        assert!(caps.requires_approval("execute_command"));
        assert!(caps.allows_domain("github.com"));
        assert!(caps.allows_domain("api.github.com"));
        assert!(!caps.allows_domain("malware.com"));
        assert!(caps.can_spawn_agent(0, 0));
        assert!(caps.can_spawn_agent(2, 1));
        assert!(!caps.can_spawn_agent(3, 0));
        assert!(!caps.can_spawn_agent(0, 2));
    }

    #[test]
    fn test_full_access_profile() {
        let caps = AgentCapabilities::full_access();

        assert!(caps.allows_tool("read_file"));
        assert!(caps.allows_tool("write_file"));
        assert!(caps.allows_tool("execute_code"));
        assert!(caps.allows_tool("execute_command"));
        assert!(caps.allows_domain("any-domain.com"));
        assert!(caps.can_spawn_agent(9, 4));
    }

    #[test]
    fn test_derive_child() {
        let parent = AgentCapabilities::standard_dev();
        let child = parent.derive_child();

        assert_eq!(child.spawning.max_depth, parent.spawning.max_depth - 1);
        assert!(!child.spawning.can_elevate);
        assert_ne!(child.capability_id, parent.capability_id);
    }

    #[test]
    fn test_capability_intersection() {
        let full = AgentCapabilities::full_access();
        let read_only = AgentCapabilities::read_only();

        let intersected = full.intersect(&read_only);

        assert!(intersected.allows_tool("read_file"));
        assert!(!intersected.allows_tool("write_file"));
        assert!(!intersected.can_spawn_agent(0, 0));
    }

    #[test]
    fn test_profile_parsing() {
        assert_eq!(
            CapabilityProfile::parse("read_only"),
            Some(CapabilityProfile::ReadOnly)
        );
        assert_eq!(
            CapabilityProfile::parse("standard_dev"),
            Some(CapabilityProfile::StandardDev)
        );
        assert_eq!(
            CapabilityProfile::parse("full_access"),
            Some(CapabilityProfile::FullAccess)
        );
        assert_eq!(CapabilityProfile::parse("invalid"), None);
    }
}
