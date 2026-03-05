use serde_json::Value;
use std::collections::HashMap;

/// Information about a connected MCP client.
#[derive(Debug, Clone, Default)]
pub struct ClientInfo {
    /// Client name.
    pub name: String,
    /// Client version.
    pub version: String,
}

/// Context for an MCP request.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Connected client info, if available.
    pub client_info: Option<ClientInfo>,
    /// JSON-RPC request ID.
    pub request_id: Value,
    /// Whether the connection has been initialized.
    pub initialized: bool,
    /// Arbitrary key-value metadata.
    pub metadata: HashMap<String, Value>,
}

impl RequestContext {
    /// Create a new request context with the given ID.
    pub fn new(request_id: Value) -> Self {
        Self {
            client_info: None,
            request_id,
            initialized: false,
            metadata: HashMap::new(),
        }
    }

    /// Set the client info.
    pub fn with_client_info(mut self, info: ClientInfo) -> Self {
        self.client_info = Some(info);
        self
    }

    /// Mark this context as initialized.
    pub fn set_initialized(&mut self) {
        self.initialized = true;
    }

    /// Get a metadata value by key.
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    /// Set a metadata key-value pair.
    pub fn set_metadata(&mut self, key: String, value: Value) {
        self.metadata.insert(key, value);
    }
}
