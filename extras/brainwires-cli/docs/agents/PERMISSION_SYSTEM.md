# Agent Permission System

> **Status:** This document describes the **planned** permission system architecture.
> `PermissionMode` (`src/types/agent.rs`) is currently implemented. The `PolicyEngine`,
> `AgentCapabilities`, `TrustFactor`, and audit infrastructure described below represent
> the target design for future implementation phases.

## Comprehensive Agent Permission System

### Research Foundation

Based on analysis of recent Arxiv papers (2024-2025) on AI agent security:
- [Infrastructure for AI Agents](https://arxiv.org/html/2501.10114v2) - Identity binding, capability certification, oversight layers
- [TRiSM for Agentic AI](https://arxiv.org/html/2506.04133v1) - Trust, Risk, Security Management framework
- [Governance-as-a-Service](https://arxiv.org/html/2508.18765v2) - Runtime policy enforcement, trust factors

### Core Design Principles

1. **Capability-Based Security**: Agents receive explicit capabilities, not implicit access
2. **Least Privilege**: Agents get minimum permissions needed for their task
3. **Defense in Depth**: Multiple enforcement layers (declaration → validation → execution → audit)
4. **Trust Factor Evolution**: Agent trust scores evolve based on behavior history
5. **Human-in-the-Loop**: Critical actions require human approval regardless of trust level

---

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Permission System Architecture               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐       │
│  │   Agent      │───▶│  Capability  │───▶│   Policy     │       │
│  │   Request    │    │  Validator   │    │   Engine     │       │
│  └──────────────┘    └──────────────┘    └──────────────┘       │
│                              │                   │              │
│                              ▼                   ▼              │
│                      ┌──────────────┐    ┌──────────────┐       │
│                      │   Trust      │    │   Audit      │       │
│                      │   Factor     │    │   Logger     │       │
│                      └──────────────┘    └──────────────┘       │
│                              │                   │              │
│                              ▼                   ▼              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   AccessControlManager                   │   │
│  │   (File Locks | Resource Locks | Read-Before-Write)      │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

### 1. Capability System

Extend current `PermissionMode` to a granular capability model:

```rust
/// Agent capabilities - explicit permissions granted to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Unique capability set ID for auditing
    pub capability_id: String,

    /// File system capabilities
    pub filesystem: FilesystemCapabilities,

    /// Tool execution capabilities
    pub tools: ToolCapabilities,

    /// Network capabilities
    pub network: NetworkCapabilities,

    /// Agent spawning capabilities
    pub spawning: SpawningCapabilities,

    /// Git operation capabilities
    pub git: GitCapabilities,

    /// Resource quota limits
    pub quotas: ResourceQuotas,

    /// Trust level (affects approval requirements)
    pub trust_level: TrustLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemCapabilities {
    /// Allowed read paths (glob patterns)
    pub read_paths: Vec<PathPattern>,

    /// Allowed write paths (glob patterns)
    pub write_paths: Vec<PathPattern>,

    /// Denied paths (override allows)
    pub denied_paths: Vec<PathPattern>,

    /// Can follow symlinks outside allowed paths
    pub follow_symlinks: bool,

    /// Can access hidden files (dotfiles)
    pub access_hidden: bool,

    /// Maximum file size for write operations
    pub max_write_size: Option<u64>,

    /// Can delete files
    pub can_delete: bool,

    /// Can create directories
    pub can_create_dirs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapabilities {
    /// Tool categories allowed
    pub allowed_categories: HashSet<ToolCategory>,

    /// Specific tools denied (overrides category allows)
    pub denied_tools: HashSet<String>,

    /// Specific tools allowed (if not using categories)
    pub allowed_tools: Option<HashSet<String>>,

    /// Tool-specific argument restrictions
    pub tool_restrictions: HashMap<String, ToolRestrictions>,

    /// Require approval for these tools regardless of trust
    pub always_approve: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCategory {
    FileRead,       // read_file, list_directory, search_files
    FileWrite,      // write_file, edit_file, patch_file, delete_file
    Search,         // search_code, search_files, semantic_search
    Git,            // All git operations
    GitDestructive, // force push, hard reset, rebase
    Bash,           // Shell command execution
    BashRestricted, // Bash with command allowlist only
    Web,            // fetch_url, web_search, web_scrape
    CodeExecution,  // execute_code (sandboxed)
    AgentSpawn,     // agent_spawn, agent_stop
    Planning,       // plan_task, task management
    System,         // System-level operations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCapabilities {
    /// Allowed domains (glob patterns)
    pub allowed_domains: Vec<String>,

    /// Denied domains (override allows)
    pub denied_domains: Vec<String>,

    /// Allow all domains (use with caution)
    pub allow_all: bool,

    /// Rate limit (requests per minute)
    pub rate_limit: Option<u32>,

    /// Can make external API calls
    pub allow_api_calls: bool,

    /// Maximum response size to process
    pub max_response_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawningCapabilities {
    /// Can spawn child agents
    pub can_spawn: bool,

    /// Maximum concurrent child agents
    pub max_children: u32,

    /// Maximum depth of agent hierarchy
    pub max_depth: u32,

    /// Capabilities inherited by children (subset of parent)
    pub child_capability_template: Option<Box<AgentCapabilities>>,

    /// Can spawn agents with elevated privileges (requires approval)
    pub can_elevate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCapabilities {
    /// Allowed operations
    pub allowed_ops: HashSet<GitOperation>,

    /// Protected branches (cannot push directly)
    pub protected_branches: Vec<String>,

    /// Can force push (dangerous)
    pub can_force_push: bool,

    /// Can perform destructive operations
    pub can_destructive: bool,

    /// Require PR for these branches
    pub require_pr_branches: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GitOperation {
    Status,
    Diff,
    Log,
    Add,
    Commit,
    Push,
    Pull,
    Fetch,
    Branch,
    Checkout,
    Merge,
    Rebase,
    Reset,
    Stash,
    Tag,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuotas {
    /// Maximum execution time (seconds)
    pub max_execution_time: Option<u64>,

    /// Maximum memory usage (bytes)
    pub max_memory: Option<u64>,

    /// Maximum API tokens consumed
    pub max_tokens: Option<u64>,

    /// Maximum tool calls per session
    pub max_tool_calls: Option<u32>,

    /// Maximum files modified per session
    pub max_files_modified: Option<u32>,
}
```

---

### 2. Trust Factor System

Implement graduated trust based on agent behavior (from GaaS paper):

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustLevel {
    /// New/unknown agent - requires approval for most actions
    Untrusted = 0,

    /// Some history, basic operations allowed
    Low = 1,

    /// Good track record, most operations allowed
    Medium = 2,

    /// Excellent history, minimal oversight needed
    High = 3,

    /// System-level agent (orchestrator)
    System = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustFactor {
    /// Current trust score (0.0 - 1.0)
    pub score: f64,

    /// Derived trust level
    pub level: TrustLevel,

    /// Historical violation counts
    pub violations: ViolationCounts,

    /// Successful operation count
    pub successful_ops: u64,

    /// Total operation count
    pub total_ops: u64,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,

    /// Trust decay rate (per day without activity)
    pub decay_rate: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViolationCounts {
    /// Critical violations (attempted unauthorized access)
    pub critical: u32,

    /// Major violations (policy breaches)
    pub major: u32,

    /// Minor violations (soft policy warnings)
    pub minor: u32,

    /// Recent violations (within last 24h) - weighted higher
    pub recent_critical: u32,
    pub recent_major: u32,
    pub recent_minor: u32,
}

impl TrustFactor {
    /// Calculate trust score based on violations and success rate
    pub fn calculate_score(&self) -> f64 {
        let base_score = if self.total_ops > 0 {
            self.successful_ops as f64 / self.total_ops as f64
        } else {
            0.5 // Default for new agents
        };

        // Violation penalties (recent violations weighted 2x)
        let violation_penalty =
            (self.violations.critical as f64 * 0.15) +
            (self.violations.major as f64 * 0.08) +
            (self.violations.minor as f64 * 0.02) +
            (self.violations.recent_critical as f64 * 0.30) +
            (self.violations.recent_major as f64 * 0.16) +
            (self.violations.recent_minor as f64 * 0.04);

        (base_score - violation_penalty).clamp(0.0, 1.0)
    }

    /// Derive trust level from score
    pub fn derive_level(&self) -> TrustLevel {
        match self.score {
            s if s >= 0.9 => TrustLevel::High,
            s if s >= 0.7 => TrustLevel::Medium,
            s if s >= 0.4 => TrustLevel::Low,
            _ => TrustLevel::Untrusted,
        }
    }
}
```

---

### 3. Policy Engine

Declarative policy enforcement (inspired by GaaS):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEngine {
    /// Active policies
    pub policies: Vec<Policy>,

    /// Default action when no policy matches
    pub default_action: PolicyAction,

    /// Enable audit logging
    pub audit_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique policy ID
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Policy priority (higher = checked first)
    pub priority: u32,

    /// Conditions that must match
    pub conditions: Vec<PolicyCondition>,

    /// Action to take when matched
    pub action: PolicyAction,

    /// Enforcement mode
    pub enforcement: EnforcementMode,

    /// Is this policy active?
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyCondition {
    /// Match specific tool
    Tool(String),

    /// Match tool category
    ToolCategory(ToolCategory),

    /// Match file path pattern
    FilePath(PathPattern),

    /// Match agent trust level
    TrustLevel(TrustLevelCondition),

    /// Match network domain
    Domain(String),

    /// Match git operation
    GitOp(GitOperation),

    /// Match time of day
    TimeRange { start: u8, end: u8 },

    /// Compound conditions
    And(Vec<PolicyCondition>),
    Or(Vec<PolicyCondition>),
    Not(Box<PolicyCondition>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevelCondition {
    AtLeast(TrustLevel),
    AtMost(TrustLevel),
    Exactly(TrustLevel),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    /// Allow the action
    Allow,

    /// Deny the action
    Deny,

    /// Require human approval
    RequireApproval,

    /// Allow but log for review
    AllowWithAudit,

    /// Deny with custom message
    DenyWithMessage(String),

    /// Escalate to higher authority
    Escalate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EnforcementMode {
    /// Hard block - cannot proceed
    Coercive,

    /// Soft warning - logged but allowed
    Normative,

    /// Adaptive based on trust score
    Adaptive,
}
```

---

### 4. Audit Logging System

Comprehensive audit trail (from TRiSM framework):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub event_id: Uuid,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Agent that triggered the event
    pub agent_id: String,

    /// Parent agent (if spawned)
    pub parent_agent_id: Option<String>,

    /// Event type
    pub event_type: AuditEventType,

    /// Action requested
    pub action: String,

    /// Target resource (file, URL, tool, etc.)
    pub target: String,

    /// Policy that was evaluated
    pub policy_id: Option<String>,

    /// Decision made
    pub decision: PolicyAction,

    /// Trust level at time of action
    pub trust_level: TrustLevel,

    /// Was human approval required/obtained?
    pub human_approval: Option<HumanApproval>,

    /// Additional context
    pub metadata: HashMap<String, Value>,

    /// Outcome (if action was allowed)
    pub outcome: Option<ActionOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Tool execution attempt
    ToolExecution,

    /// File system access
    FileAccess,

    /// Network request
    NetworkRequest,

    /// Agent spawned
    AgentSpawn,

    /// Agent terminated
    AgentTerminate,

    /// Policy violation
    PolicyViolation,

    /// Trust level change
    TrustChange,

    /// Human intervention
    HumanIntervention,

    /// Capability modification
    CapabilityChange,

    /// System event
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanApproval {
    /// Was approval requested?
    pub requested: bool,

    /// Was approval granted?
    pub granted: Option<bool>,

    /// User who approved/denied
    pub user: Option<String>,

    /// Timestamp of decision
    pub decided_at: Option<DateTime<Utc>>,

    /// Reason provided
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionOutcome {
    Success,
    Failure(String),
    Partial(String),
    Timeout,
    Cancelled,
}
```

---

### 5. Capability Profiles (Presets)

Pre-defined capability sets for common use cases:

```rust
impl AgentCapabilities {
    /// Read-only exploration - safe for untrusted agents
    pub fn read_only() -> Self {
        Self {
            filesystem: FilesystemCapabilities {
                read_paths: vec![PathPattern::All],
                write_paths: vec![],
                denied_paths: vec![
                    PathPattern::glob("**/.env*"),
                    PathPattern::glob("**/*credentials*"),
                    PathPattern::glob("**/*secret*"),
                ],
                follow_symlinks: false,
                access_hidden: false,
                can_delete: false,
                can_create_dirs: false,
                max_write_size: None,
            },
            tools: ToolCapabilities {
                allowed_categories: hashset![
                    ToolCategory::FileRead,
                    ToolCategory::Search,
                ],
                denied_tools: hashset![],
                allowed_tools: None,
                tool_restrictions: HashMap::new(),
                always_approve: hashset![],
            },
            network: NetworkCapabilities::disabled(),
            spawning: SpawningCapabilities::disabled(),
            git: GitCapabilities::read_only(),
            quotas: ResourceQuotas::conservative(),
            trust_level: TrustLevel::Low,
        }
    }

    /// Standard development - balanced safety and utility
    pub fn standard_dev() -> Self {
        Self {
            filesystem: FilesystemCapabilities {
                read_paths: vec![PathPattern::All],
                write_paths: vec![
                    PathPattern::glob("src/**"),
                    PathPattern::glob("tests/**"),
                    PathPattern::glob("docs/**"),
                ],
                denied_paths: vec![
                    PathPattern::glob("**/.env*"),
                    PathPattern::glob("**/node_modules/**"),
                    PathPattern::glob("**/target/**"),
                ],
                follow_symlinks: true,
                access_hidden: true,
                can_delete: true,
                can_create_dirs: true,
                max_write_size: Some(1024 * 1024), // 1MB
            },
            tools: ToolCapabilities {
                allowed_categories: hashset![
                    ToolCategory::FileRead,
                    ToolCategory::FileWrite,
                    ToolCategory::Search,
                    ToolCategory::Git,
                    ToolCategory::BashRestricted,
                    ToolCategory::Planning,
                ],
                denied_tools: hashset!["execute_code".to_string()],
                allowed_tools: None,
                tool_restrictions: HashMap::new(),
                always_approve: hashset![
                    "delete_file".to_string(),
                    "execute_command".to_string(),
                ],
            },
            network: NetworkCapabilities {
                allowed_domains: vec![
                    "github.com".to_string(),
                    "*.github.com".to_string(),
                    "docs.rs".to_string(),
                    "crates.io".to_string(),
                ],
                denied_domains: vec![],
                allow_all: false,
                rate_limit: Some(60),
                allow_api_calls: true,
                max_response_size: Some(10 * 1024 * 1024),
            },
            spawning: SpawningCapabilities {
                can_spawn: true,
                max_children: 3,
                max_depth: 2,
                child_capability_template: None, // Inherit
                can_elevate: false,
            },
            git: GitCapabilities::standard(),
            quotas: ResourceQuotas::standard(),
            trust_level: TrustLevel::Medium,
        }
    }

    /// Full access - for trusted orchestrators
    pub fn full_access() -> Self {
        Self {
            filesystem: FilesystemCapabilities::full(),
            tools: ToolCapabilities::full(),
            network: NetworkCapabilities::full(),
            spawning: SpawningCapabilities::full(),
            git: GitCapabilities::full(),
            quotas: ResourceQuotas::generous(),
            trust_level: TrustLevel::High,
        }
    }
}
```

---

### 6. Integration Points

#### 6.1 Extend `AgentContext`

```rust
pub struct AgentContext {
    // ... existing fields ...

    /// Agent's granted capabilities
    pub capabilities: AgentCapabilities,

    /// Agent's current trust factor
    pub trust_factor: TrustFactor,

    /// Policy engine reference
    pub policy_engine: Arc<PolicyEngine>,

    /// Audit logger
    pub audit_logger: Arc<AuditLogger>,
}
```

#### 6.2 Tool Executor Integration

```rust
impl ToolExecutor {
    pub async fn execute_with_permissions(
        &self,
        tool: &Tool,
        args: &Value,
        context: &AgentContext,
    ) -> Result<ToolResult> {
        // 1. Check capabilities
        if !context.capabilities.allows_tool(tool) {
            self.audit_logger.log_denied(tool, "capability_check");
            return Err(PermissionDenied::tool(tool.name.clone()));
        }

        // 2. Evaluate policies
        let decision = context.policy_engine.evaluate(
            &PolicyRequest::tool(tool, args, context)
        ).await?;

        // 3. Handle decision
        match decision {
            PolicyAction::Allow => {},
            PolicyAction::RequireApproval => {
                let approved = self.request_approval(tool, args).await?;
                if !approved {
                    return Err(PermissionDenied::approval_denied());
                }
            },
            PolicyAction::Deny => {
                return Err(PermissionDenied::policy(decision.policy_id));
            },
            // ... other actions
        }

        // 4. Execute with quota tracking
        let result = context.quotas.track_execution(|| {
            self.inner_execute(tool, args, context).await
        }).await?;

        // 5. Update trust factor based on outcome
        context.trust_factor.record_operation(result.is_ok());

        // 6. Audit log
        self.audit_logger.log_execution(tool, args, &result);

        result
    }
}
```

#### 6.3 Configuration File Format

```toml
# ~/.brainwires/permissions.toml

[default]
profile = "standard_dev"  # read_only | standard_dev | full_access | custom

[filesystem]
read_paths = ["**/*"]
write_paths = ["src/**", "tests/**", "docs/**"]
denied_paths = ["**/.env*", "**/secrets/**"]
follow_symlinks = true
access_hidden = true
max_write_size = "1MB"

[tools]
allowed_categories = ["FileRead", "FileWrite", "Search", "Git", "Planning"]
denied_tools = ["execute_code"]
always_approve = ["delete_file", "execute_command"]

[network]
allowed_domains = ["github.com", "*.github.com", "docs.rs"]
rate_limit = 60  # requests per minute
allow_api_calls = true

[spawning]
enabled = true
max_children = 3
max_depth = 2

[git]
allowed_ops = ["Status", "Diff", "Log", "Add", "Commit", "Push", "Pull"]
protected_branches = ["main", "master"]
can_force_push = false

[quotas]
max_execution_time = "30m"
max_tool_calls = 500
max_files_modified = 50

[policies]

[[policies.rules]]
name = "protect_secrets"
priority = 100
conditions = [
    { file_path = "**/.env*" },
    { file_path = "**/*secret*" },
    { file_path = "**/*credential*" },
]
action = "Deny"
enforcement = "Coercive"

[[policies.rules]]
name = "approve_destructive_git"
priority = 90
conditions = [
    { git_op = "Reset" },
    { git_op = "Rebase" },
]
action = "RequireApproval"
enforcement = "Coercive"

[[policies.rules]]
name = "audit_network"
priority = 50
conditions = [
    { tool_category = "Web" },
]
action = "AllowWithAudit"
enforcement = "Normative"
```

---

### 7. Human-in-the-Loop Controls

Critical actions that ALWAYS require approval regardless of trust level:

```rust
const ALWAYS_APPROVE_OPERATIONS: &[&str] = &[
    // Destructive git operations
    "git_force_push",
    "git_reset_hard",
    "git_rebase",

    // File operations on sensitive paths
    "write_env_file",
    "delete_config",

    // System operations
    "execute_arbitrary_code",
    "spawn_elevated_agent",

    // Network operations
    "post_to_external_api",
    "upload_file",

    // Agent lifecycle
    "terminate_other_agent",
    "modify_agent_capabilities",
];
```

---

### 8. Implementation Phases

**Phase 1: Foundation** (Core Infrastructure)
- [ ] Implement `AgentCapabilities` struct
- [ ] Implement `TrustFactor` system
- [ ] Add capability checking to `ToolExecutor`
- [ ] Create `permissions.toml` config loading

**Phase 2: Policy Engine**
- [ ] Implement `PolicyEngine` with condition matching
- [ ] Add policy evaluation to tool execution path
- [ ] Create default policy sets
- [ ] Integrate with existing `ApprovalManager`

**Phase 3: Audit System**
- [ ] Implement `AuditLogger`
- [ ] Add audit events to all permission checks
- [ ] Create audit log viewer/query interface
- [ ] Add audit export functionality

**Phase 4: Trust Evolution**
- [ ] Implement trust score calculation
- [ ] Add violation tracking
- [ ] Create trust level promotion/demotion logic
- [ ] Add trust decay for inactive agents

**Phase 5: Advanced Features**
- [ ] Capability inheritance for child agents
- [ ] Runtime capability modification
- [ ] Policy hot-reloading
- [ ] Network boundary enforcement
- [ ] Resource quota enforcement

---

### 9. CLI Integration

New commands for permission management:

```bash
# View current permissions
brainwires permissions show

# Set permission profile
brainwires permissions set standard_dev

# View audit log
brainwires audit log [--agent <id>] [--since <time>]

# View agent trust levels
brainwires trust show [--agent <id>]

# Reset trust for an agent
brainwires trust reset <agent-id>

# Create custom policy
brainwires policy create <name> --conditions '...' --action deny
```

---

### Sources

- [Infrastructure for AI Agents](https://arxiv.org/html/2501.10114v2) - Agent identity, capability certification
- [TRiSM for Agentic AI](https://arxiv.org/html/2506.04133v1) - Trust, Risk, Security framework
- [Governance-as-a-Service](https://arxiv.org/html/2508.18765v2) - Policy enforcement, trust factors
- [Agentic AI Frameworks](https://arxiv.org/html/2508.10146v1) - Framework comparisons, MCP protocol
