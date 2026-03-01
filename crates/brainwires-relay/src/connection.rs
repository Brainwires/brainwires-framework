use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub client_info: Option<ClientInfo>,
    pub request_id: Value,
    pub initialized: bool,
    pub metadata: HashMap<String, Value>,
}

impl RequestContext {
    pub fn new(request_id: Value) -> Self {
        Self {
            client_info: None,
            request_id,
            initialized: false,
            metadata: HashMap::new(),
        }
    }

    pub fn with_client_info(mut self, info: ClientInfo) -> Self {
        self.client_info = Some(info);
        self
    }

    pub fn set_initialized(&mut self) {
        self.initialized = true;
    }

    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    pub fn set_metadata(&mut self, key: String, value: Value) {
        self.metadata.insert(key, value);
    }
}
