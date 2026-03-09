//! Conversions between hand-written serde types and proto-generated types.
//!
//! These are gated behind the `grpc` feature and enable the gRPC service layer
//! to use the same `A2aHandler` trait as JSON-RPC and REST.

#[cfg(feature = "grpc")]
mod grpc_convert {
    use std::collections::HashMap;

    use crate::agent_card::*;
    use crate::proto::lf_a2a_v1 as pb;
    use crate::push_notification::{AuthenticationInfo, TaskPushNotificationConfig};
    use crate::task::{Task, TaskState, TaskStatus};
    use crate::types::{Artifact, FileContent, Message, Part, Role};

    // ===================================================================
    // Helpers: HashMap<String, serde_json::Value> ↔ prost_types::Struct
    // ===================================================================

    pub(crate) fn hashmap_to_struct(m: HashMap<String, serde_json::Value>) -> prost_types::Struct {
        prost_types::Struct {
            fields: m
                .into_iter()
                .map(|(k, v)| (k, json_to_prost_value(v)))
                .collect(),
        }
    }

    pub(crate) fn struct_to_hashmap(s: prost_types::Struct) -> HashMap<String, serde_json::Value> {
        s.fields
            .into_iter()
            .map(|(k, v)| (k, prost_value_to_json(v)))
            .collect()
    }

    fn json_to_prost_value(v: serde_json::Value) -> prost_types::Value {
        use prost_types::value::Kind;
        let kind = match v {
            serde_json::Value::Null => Kind::NullValue(0),
            serde_json::Value::Bool(b) => Kind::BoolValue(b),
            serde_json::Value::Number(n) => Kind::NumberValue(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(s) => Kind::StringValue(s),
            serde_json::Value::Array(arr) => Kind::ListValue(prost_types::ListValue {
                values: arr.into_iter().map(json_to_prost_value).collect(),
            }),
            serde_json::Value::Object(obj) => Kind::StructValue(prost_types::Struct {
                fields: obj
                    .into_iter()
                    .map(|(k, v)| (k, json_to_prost_value(v)))
                    .collect(),
            }),
        };
        prost_types::Value { kind: Some(kind) }
    }

    fn prost_value_to_json(v: prost_types::Value) -> serde_json::Value {
        use prost_types::value::Kind;
        match v.kind {
            Some(Kind::NullValue(_)) | None => serde_json::Value::Null,
            Some(Kind::BoolValue(b)) => serde_json::Value::Bool(b),
            Some(Kind::NumberValue(n)) => serde_json::Number::from_f64(n)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Some(Kind::StringValue(s)) => serde_json::Value::String(s),
            Some(Kind::ListValue(l)) => {
                serde_json::Value::Array(l.values.into_iter().map(prost_value_to_json).collect())
            }
            Some(Kind::StructValue(s)) => {
                let map: serde_json::Map<String, serde_json::Value> = s
                    .fields
                    .into_iter()
                    .map(|(k, v)| (k, prost_value_to_json(v)))
                    .collect();
                serde_json::Value::Object(map)
            }
        }
    }

    fn opt_hashmap_to_struct(
        m: Option<HashMap<String, serde_json::Value>>,
    ) -> Option<prost_types::Struct> {
        m.map(hashmap_to_struct)
    }

    fn opt_struct_to_hashmap(
        s: Option<prost_types::Struct>,
    ) -> Option<HashMap<String, serde_json::Value>> {
        s.map(struct_to_hashmap)
    }

    fn opt_empty(s: &str) -> Option<String> {
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    }

    // ===================================================================
    // Role
    // ===================================================================

    impl From<Role> for i32 {
        fn from(r: Role) -> i32 {
            match r {
                Role::User => pb::Role::User as i32,
                Role::Agent => pb::Role::Agent as i32,
            }
        }
    }

    impl From<i32> for Role {
        fn from(v: i32) -> Self {
            match v {
                1 => Role::User,
                2 => Role::Agent,
                _ => Role::User,
            }
        }
    }

    // ===================================================================
    // TaskState
    // ===================================================================

    impl From<TaskState> for i32 {
        fn from(s: TaskState) -> i32 {
            match s {
                TaskState::Unknown => 0,
                TaskState::Submitted => 1,
                TaskState::Working => 2,
                TaskState::Completed => 3,
                TaskState::Failed => 4,
                TaskState::Canceled => 5,
                TaskState::InputRequired => 6,
                TaskState::Rejected => 7,
                TaskState::AuthRequired => 8,
            }
        }
    }

    impl From<i32> for TaskState {
        fn from(v: i32) -> Self {
            match v {
                1 => TaskState::Submitted,
                2 => TaskState::Working,
                3 => TaskState::Completed,
                4 => TaskState::Failed,
                5 => TaskState::Canceled,
                6 => TaskState::InputRequired,
                7 => TaskState::Rejected,
                8 => TaskState::AuthRequired,
                _ => TaskState::Unknown,
            }
        }
    }

    // ===================================================================
    // Part
    // ===================================================================

    impl From<Part> for pb::Part {
        fn from(p: Part) -> pb::Part {
            let (content, metadata_map, filename, media_type) = match p {
                Part::Text { text, metadata } => (
                    Some(pb::part::Content::Text(text)),
                    metadata,
                    String::new(),
                    String::new(),
                ),
                Part::File { file, metadata } => {
                    let (content, fname, mtype) = match file {
                        FileContent::Bytes {
                            bytes,
                            mime_type,
                            name,
                        } => (
                            pb::part::Content::Raw(bytes.into_bytes().into()),
                            name.unwrap_or_default(),
                            mime_type.unwrap_or_default(),
                        ),
                        FileContent::Uri {
                            uri,
                            mime_type,
                            name,
                        } => (
                            pb::part::Content::Url(uri),
                            name.unwrap_or_default(),
                            mime_type.unwrap_or_default(),
                        ),
                    };
                    (Some(content), metadata, fname, mtype)
                }
                Part::Data { data, metadata } => (
                    Some(pb::part::Content::Data(json_to_prost_value(data))),
                    metadata,
                    String::new(),
                    String::new(),
                ),
            };

            pb::Part {
                content,
                metadata: opt_hashmap_to_struct(metadata_map),
                filename,
                media_type,
            }
        }
    }

    impl From<pb::Part> for Part {
        fn from(p: pb::Part) -> Part {
            let metadata = opt_struct_to_hashmap(p.metadata);
            match p.content {
                Some(pb::part::Content::Text(t)) => Part::Text { text: t, metadata },
                Some(pb::part::Content::Url(u)) => Part::File {
                    file: FileContent::Uri {
                        uri: u,
                        mime_type: opt_empty(&p.media_type),
                        name: opt_empty(&p.filename),
                    },
                    metadata,
                },
                Some(pb::part::Content::Raw(b)) => Part::File {
                    file: FileContent::Bytes {
                        bytes: String::from_utf8_lossy(&b).to_string(),
                        mime_type: opt_empty(&p.media_type),
                        name: opt_empty(&p.filename),
                    },
                    metadata,
                },
                Some(pb::part::Content::Data(v)) => Part::Data {
                    data: prost_value_to_json(v),
                    metadata,
                },
                None => Part::Text {
                    text: String::new(),
                    metadata,
                },
            }
        }
    }

    // ===================================================================
    // Message
    // ===================================================================

    impl From<Message> for pb::Message {
        fn from(m: Message) -> pb::Message {
            pb::Message {
                message_id: m.message_id,
                context_id: m.context_id.unwrap_or_default(),
                task_id: m.task_id.unwrap_or_default(),
                role: i32::from(m.role),
                parts: m.parts.into_iter().map(Into::into).collect(),
                metadata: opt_hashmap_to_struct(m.metadata),
                extensions: m.extensions.unwrap_or_default(),
                reference_task_ids: m.reference_task_ids.unwrap_or_default(),
            }
        }
    }

    impl From<pb::Message> for Message {
        fn from(m: pb::Message) -> Message {
            Message {
                message_id: m.message_id,
                role: Role::from(m.role),
                parts: m.parts.into_iter().map(Into::into).collect(),
                context_id: opt_empty(&m.context_id),
                task_id: opt_empty(&m.task_id),
                reference_task_ids: if m.reference_task_ids.is_empty() {
                    None
                } else {
                    Some(m.reference_task_ids)
                },
                metadata: opt_struct_to_hashmap(m.metadata),
                extensions: if m.extensions.is_empty() {
                    None
                } else {
                    Some(m.extensions)
                },
                kind: "message".to_string(),
            }
        }
    }

    // ===================================================================
    // Artifact
    // ===================================================================

    impl From<Artifact> for pb::Artifact {
        fn from(a: Artifact) -> pb::Artifact {
            pb::Artifact {
                artifact_id: a.artifact_id,
                name: a.name.unwrap_or_default(),
                description: a.description.unwrap_or_default(),
                parts: a.parts.into_iter().map(Into::into).collect(),
                metadata: opt_hashmap_to_struct(a.metadata),
                extensions: a.extensions.unwrap_or_default(),
            }
        }
    }

    impl From<pb::Artifact> for Artifact {
        fn from(a: pb::Artifact) -> Artifact {
            Artifact {
                artifact_id: a.artifact_id,
                name: opt_empty(&a.name),
                description: opt_empty(&a.description),
                parts: a.parts.into_iter().map(Into::into).collect(),
                metadata: opt_struct_to_hashmap(a.metadata),
                extensions: if a.extensions.is_empty() {
                    None
                } else {
                    Some(a.extensions)
                },
            }
        }
    }

    // ===================================================================
    // TaskStatus
    // ===================================================================

    impl From<TaskStatus> for pb::TaskStatus {
        fn from(s: TaskStatus) -> pb::TaskStatus {
            pb::TaskStatus {
                state: i32::from(s.state),
                message: s.message.map(Into::into),
                timestamp: s.timestamp.and_then(|t| {
                    chrono::DateTime::parse_from_rfc3339(&t)
                        .ok()
                        .map(|dt| prost_types::Timestamp {
                            seconds: dt.timestamp(),
                            nanos: dt.timestamp_subsec_nanos() as i32,
                        })
                }),
            }
        }
    }

    impl From<pb::TaskStatus> for TaskStatus {
        fn from(s: pb::TaskStatus) -> TaskStatus {
            TaskStatus {
                state: TaskState::from(s.state),
                message: s.message.map(Into::into),
                timestamp: s.timestamp.and_then(|t| {
                    chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32)
                        .map(|dt| dt.to_rfc3339())
                }),
            }
        }
    }

    // ===================================================================
    // Task
    // ===================================================================

    impl From<Task> for pb::Task {
        fn from(t: Task) -> pb::Task {
            pb::Task {
                id: t.id,
                context_id: t.context_id.unwrap_or_default(),
                status: Some(t.status.into()),
                artifacts: t
                    .artifacts
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                history: t
                    .history
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                metadata: opt_hashmap_to_struct(t.metadata),
            }
        }
    }

    impl From<pb::Task> for Task {
        fn from(t: pb::Task) -> Task {
            Task {
                id: t.id,
                context_id: opt_empty(&t.context_id),
                status: t.status.map(Into::into).unwrap_or(TaskStatus {
                    state: TaskState::Unknown,
                    message: None,
                    timestamp: None,
                }),
                artifacts: if t.artifacts.is_empty() {
                    None
                } else {
                    Some(t.artifacts.into_iter().map(Into::into).collect())
                },
                history: if t.history.is_empty() {
                    None
                } else {
                    Some(t.history.into_iter().map(Into::into).collect())
                },
                metadata: opt_struct_to_hashmap(t.metadata),
                kind: "task".to_string(),
            }
        }
    }

    // ===================================================================
    // AuthenticationInfo
    // ===================================================================

    impl From<AuthenticationInfo> for pb::AuthenticationInfo {
        fn from(a: AuthenticationInfo) -> pb::AuthenticationInfo {
            pb::AuthenticationInfo {
                scheme: a.scheme,
                credentials: a.credentials.unwrap_or_default(),
            }
        }
    }

    impl From<pb::AuthenticationInfo> for AuthenticationInfo {
        fn from(a: pb::AuthenticationInfo) -> AuthenticationInfo {
            AuthenticationInfo {
                scheme: a.scheme,
                credentials: opt_empty(&a.credentials),
            }
        }
    }

    // ===================================================================
    // TaskPushNotificationConfig
    // ===================================================================

    impl From<TaskPushNotificationConfig> for pb::TaskPushNotificationConfig {
        fn from(c: TaskPushNotificationConfig) -> pb::TaskPushNotificationConfig {
            pb::TaskPushNotificationConfig {
                tenant: c.tenant.unwrap_or_default(),
                id: c.id.unwrap_or_default(),
                task_id: c.task_id,
                url: c.url,
                token: c.token.unwrap_or_default(),
                authentication: c.authentication.map(Into::into),
            }
        }
    }

    impl From<pb::TaskPushNotificationConfig> for TaskPushNotificationConfig {
        fn from(c: pb::TaskPushNotificationConfig) -> TaskPushNotificationConfig {
            TaskPushNotificationConfig {
                tenant: opt_empty(&c.tenant),
                id: opt_empty(&c.id),
                task_id: c.task_id,
                url: c.url,
                token: opt_empty(&c.token),
                authentication: c.authentication.map(Into::into),
            }
        }
    }

    // ===================================================================
    // AgentProvider
    // ===================================================================

    impl From<AgentProvider> for pb::AgentProvider {
        fn from(p: AgentProvider) -> pb::AgentProvider {
            pb::AgentProvider {
                url: p.url,
                organization: p.organization,
            }
        }
    }

    impl From<pb::AgentProvider> for AgentProvider {
        fn from(p: pb::AgentProvider) -> AgentProvider {
            AgentProvider {
                url: p.url,
                organization: p.organization,
            }
        }
    }

    // ===================================================================
    // AgentExtension
    // ===================================================================

    impl From<AgentExtension> for pb::AgentExtension {
        fn from(e: AgentExtension) -> pb::AgentExtension {
            pb::AgentExtension {
                uri: e.uri,
                description: e.description.unwrap_or_default(),
                required: e.required,
                params: e.params.map(hashmap_to_struct),
            }
        }
    }

    impl From<pb::AgentExtension> for AgentExtension {
        fn from(e: pb::AgentExtension) -> AgentExtension {
            AgentExtension {
                uri: e.uri,
                description: opt_empty(&e.description),
                required: e.required,
                params: e.params.map(struct_to_hashmap),
            }
        }
    }

    // ===================================================================
    // AgentCapabilities
    // ===================================================================

    impl From<AgentCapabilities> for pb::AgentCapabilities {
        fn from(c: AgentCapabilities) -> pb::AgentCapabilities {
            pb::AgentCapabilities {
                streaming: c.streaming,
                push_notifications: c.push_notifications,
                extended_agent_card: c.extended_agent_card,
                extensions: c
                    .extensions
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            }
        }
    }

    impl From<pb::AgentCapabilities> for AgentCapabilities {
        fn from(c: pb::AgentCapabilities) -> AgentCapabilities {
            AgentCapabilities {
                streaming: c.streaming,
                push_notifications: c.push_notifications,
                extended_agent_card: c.extended_agent_card,
                extensions: if c.extensions.is_empty() {
                    None
                } else {
                    Some(c.extensions.into_iter().map(Into::into).collect())
                },
            }
        }
    }

    // ===================================================================
    // AgentSkill
    // ===================================================================

    impl From<AgentSkill> for pb::AgentSkill {
        fn from(s: AgentSkill) -> pb::AgentSkill {
            pb::AgentSkill {
                id: s.id,
                name: s.name,
                description: s.description,
                tags: s.tags,
                examples: s.examples.unwrap_or_default(),
                input_modes: s.input_modes.unwrap_or_default(),
                output_modes: s.output_modes.unwrap_or_default(),
                security_requirements: s
                    .security_requirements
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            }
        }
    }

    impl From<pb::AgentSkill> for AgentSkill {
        fn from(s: pb::AgentSkill) -> AgentSkill {
            AgentSkill {
                id: s.id,
                name: s.name,
                description: s.description,
                tags: s.tags,
                examples: if s.examples.is_empty() {
                    None
                } else {
                    Some(s.examples)
                },
                input_modes: if s.input_modes.is_empty() {
                    None
                } else {
                    Some(s.input_modes)
                },
                output_modes: if s.output_modes.is_empty() {
                    None
                } else {
                    Some(s.output_modes)
                },
                security_requirements: if s.security_requirements.is_empty() {
                    None
                } else {
                    Some(
                        s.security_requirements
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                    )
                },
            }
        }
    }

    // ===================================================================
    // AgentInterface
    // ===================================================================

    impl From<AgentInterface> for pb::AgentInterface {
        fn from(i: AgentInterface) -> pb::AgentInterface {
            pb::AgentInterface {
                url: i.url,
                protocol_binding: i.protocol_binding,
                tenant: i.tenant.unwrap_or_default(),
                protocol_version: i.protocol_version,
            }
        }
    }

    impl From<pb::AgentInterface> for AgentInterface {
        fn from(i: pb::AgentInterface) -> AgentInterface {
            AgentInterface {
                url: i.url,
                protocol_binding: i.protocol_binding,
                tenant: opt_empty(&i.tenant),
                protocol_version: i.protocol_version,
            }
        }
    }

    // ===================================================================
    // AgentCardSignature
    // ===================================================================

    impl From<AgentCardSignature> for pb::AgentCardSignature {
        fn from(s: AgentCardSignature) -> pb::AgentCardSignature {
            pb::AgentCardSignature {
                protected: s.protected,
                signature: s.signature,
                header: s.header.map(hashmap_to_struct),
            }
        }
    }

    impl From<pb::AgentCardSignature> for AgentCardSignature {
        fn from(s: pb::AgentCardSignature) -> AgentCardSignature {
            AgentCardSignature {
                protected: s.protected,
                signature: s.signature,
                header: s.header.map(struct_to_hashmap),
            }
        }
    }

    // ===================================================================
    // SecurityRequirement
    // ===================================================================

    impl From<SecurityRequirement> for pb::SecurityRequirement {
        fn from(r: SecurityRequirement) -> pb::SecurityRequirement {
            pb::SecurityRequirement {
                schemes: r
                    .schemes
                    .into_iter()
                    .map(|(k, v)| (k, pb::StringList { list: v }))
                    .collect(),
            }
        }
    }

    impl From<pb::SecurityRequirement> for SecurityRequirement {
        fn from(r: pb::SecurityRequirement) -> SecurityRequirement {
            SecurityRequirement {
                schemes: r.schemes.into_iter().map(|(k, v)| (k, v.list)).collect(),
            }
        }
    }

    // ===================================================================
    // SecurityScheme
    // ===================================================================

    impl From<SecurityScheme> for pb::SecurityScheme {
        fn from(s: SecurityScheme) -> pb::SecurityScheme {
            let scheme = match s {
                SecurityScheme::ApiKey {
                    name,
                    location,
                    description,
                } => Some(pb::security_scheme::Scheme::ApiKeySecurityScheme(
                    pb::ApiKeySecurityScheme {
                        description: description.unwrap_or_default(),
                        location,
                        name,
                    },
                )),
                SecurityScheme::Http {
                    scheme,
                    bearer_format,
                    description,
                } => Some(pb::security_scheme::Scheme::HttpAuthSecurityScheme(
                    pb::HttpAuthSecurityScheme {
                        description: description.unwrap_or_default(),
                        scheme,
                        bearer_format: bearer_format.unwrap_or_default(),
                    },
                )),
                SecurityScheme::OAuth2 {
                    flows,
                    description,
                    oauth2_metadata_url,
                } => Some(pb::security_scheme::Scheme::Oauth2SecurityScheme(
                    pb::OAuth2SecurityScheme {
                        description: description.unwrap_or_default(),
                        flows: Some(flows.into()),
                        oauth2_metadata_url: oauth2_metadata_url.unwrap_or_default(),
                    },
                )),
                SecurityScheme::OpenIdConnect {
                    open_id_connect_url,
                    description,
                } => Some(pb::security_scheme::Scheme::OpenIdConnectSecurityScheme(
                    pb::OpenIdConnectSecurityScheme {
                        description: description.unwrap_or_default(),
                        open_id_connect_url,
                    },
                )),
                SecurityScheme::MutualTls { description } => Some(
                    pb::security_scheme::Scheme::MtlsSecurityScheme(pb::MutualTlsSecurityScheme {
                        description: description.unwrap_or_default(),
                    }),
                ),
            };
            pb::SecurityScheme { scheme }
        }
    }

    impl From<pb::SecurityScheme> for SecurityScheme {
        fn from(s: pb::SecurityScheme) -> SecurityScheme {
            match s.scheme {
                Some(pb::security_scheme::Scheme::ApiKeySecurityScheme(a)) => {
                    SecurityScheme::ApiKey {
                        name: a.name,
                        location: a.location,
                        description: opt_empty(&a.description),
                    }
                }
                Some(pb::security_scheme::Scheme::HttpAuthSecurityScheme(h)) => {
                    SecurityScheme::Http {
                        scheme: h.scheme,
                        bearer_format: opt_empty(&h.bearer_format),
                        description: opt_empty(&h.description),
                    }
                }
                Some(pb::security_scheme::Scheme::Oauth2SecurityScheme(o)) => {
                    SecurityScheme::OAuth2 {
                        flows: o
                            .flows
                            .map(Into::into)
                            .unwrap_or(OAuthFlows::ClientCredentials {
                                token_url: String::new(),
                                refresh_url: None,
                                scopes: HashMap::new(),
                            }),
                        description: opt_empty(&o.description),
                        oauth2_metadata_url: opt_empty(&o.oauth2_metadata_url),
                    }
                }
                Some(pb::security_scheme::Scheme::OpenIdConnectSecurityScheme(o)) => {
                    SecurityScheme::OpenIdConnect {
                        open_id_connect_url: o.open_id_connect_url,
                        description: opt_empty(&o.description),
                    }
                }
                Some(pb::security_scheme::Scheme::MtlsSecurityScheme(m)) => {
                    SecurityScheme::MutualTls {
                        description: opt_empty(&m.description),
                    }
                }
                None => SecurityScheme::MutualTls { description: None },
            }
        }
    }

    // ===================================================================
    // OAuthFlows
    // ===================================================================

    impl From<OAuthFlows> for pb::OAuthFlows {
        fn from(f: OAuthFlows) -> pb::OAuthFlows {
            let flow = match f {
                OAuthFlows::AuthorizationCode {
                    authorization_url,
                    token_url,
                    refresh_url,
                    scopes,
                    pkce_required,
                } => Some(pb::o_auth_flows::Flow::AuthorizationCode(
                    pb::AuthorizationCodeOAuthFlow {
                        authorization_url,
                        token_url,
                        refresh_url: refresh_url.unwrap_or_default(),
                        scopes,
                        pkce_required: pkce_required.unwrap_or(false),
                    },
                )),
                OAuthFlows::ClientCredentials {
                    token_url,
                    refresh_url,
                    scopes,
                } => Some(pb::o_auth_flows::Flow::ClientCredentials(
                    pb::ClientCredentialsOAuthFlow {
                        token_url,
                        refresh_url: refresh_url.unwrap_or_default(),
                        scopes,
                    },
                )),
                #[allow(deprecated)]
                OAuthFlows::Implicit {
                    authorization_url,
                    refresh_url,
                    scopes,
                } => Some(pb::o_auth_flows::Flow::Implicit(pb::ImplicitOAuthFlow {
                    authorization_url: authorization_url.unwrap_or_default(),
                    refresh_url: refresh_url.unwrap_or_default(),
                    scopes,
                })),
                #[allow(deprecated)]
                OAuthFlows::Password {
                    token_url,
                    refresh_url,
                    scopes,
                } => Some(pb::o_auth_flows::Flow::Password(pb::PasswordOAuthFlow {
                    token_url: token_url.unwrap_or_default(),
                    refresh_url: refresh_url.unwrap_or_default(),
                    scopes,
                })),
                OAuthFlows::DeviceCode {
                    device_authorization_url,
                    token_url,
                    refresh_url,
                    scopes,
                } => Some(pb::o_auth_flows::Flow::DeviceCode(
                    pb::DeviceCodeOAuthFlow {
                        device_authorization_url,
                        token_url,
                        refresh_url: refresh_url.unwrap_or_default(),
                        scopes,
                    },
                )),
            };
            pb::OAuthFlows { flow }
        }
    }

    impl From<pb::OAuthFlows> for OAuthFlows {
        fn from(f: pb::OAuthFlows) -> OAuthFlows {
            match f.flow {
                Some(pb::o_auth_flows::Flow::AuthorizationCode(a)) => {
                    OAuthFlows::AuthorizationCode {
                        authorization_url: a.authorization_url,
                        token_url: a.token_url,
                        refresh_url: opt_empty(&a.refresh_url),
                        scopes: a.scopes,
                        pkce_required: Some(a.pkce_required),
                    }
                }
                Some(pb::o_auth_flows::Flow::ClientCredentials(c)) => {
                    OAuthFlows::ClientCredentials {
                        token_url: c.token_url,
                        refresh_url: opt_empty(&c.refresh_url),
                        scopes: c.scopes,
                    }
                }
                #[allow(deprecated)]
                Some(pb::o_auth_flows::Flow::Implicit(i)) => OAuthFlows::Implicit {
                    authorization_url: opt_empty(&i.authorization_url),
                    refresh_url: opt_empty(&i.refresh_url),
                    scopes: i.scopes,
                },
                #[allow(deprecated)]
                Some(pb::o_auth_flows::Flow::Password(p)) => OAuthFlows::Password {
                    token_url: opt_empty(&p.token_url),
                    refresh_url: opt_empty(&p.refresh_url),
                    scopes: p.scopes,
                },
                Some(pb::o_auth_flows::Flow::DeviceCode(d)) => OAuthFlows::DeviceCode {
                    device_authorization_url: d.device_authorization_url,
                    token_url: d.token_url,
                    refresh_url: opt_empty(&d.refresh_url),
                    scopes: d.scopes,
                },
                None => OAuthFlows::ClientCredentials {
                    token_url: String::new(),
                    refresh_url: None,
                    scopes: HashMap::new(),
                },
            }
        }
    }

    // ===================================================================
    // AgentCard
    // ===================================================================

    impl From<AgentCard> for pb::AgentCard {
        fn from(c: AgentCard) -> pb::AgentCard {
            pb::AgentCard {
                name: c.name,
                description: c.description,
                version: c.version,
                supported_interfaces: c
                    .supported_interfaces
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                provider: c.provider.map(Into::into),
                capabilities: Some(c.capabilities.into()),
                security_schemes: c
                    .security_schemes
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect(),
                security_requirements: c
                    .security_requirements
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                default_input_modes: c.default_input_modes,
                default_output_modes: c.default_output_modes,
                skills: c.skills.into_iter().map(Into::into).collect(),
                signatures: c
                    .signatures
                    .unwrap_or_default()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                documentation_url: c.documentation_url,
                icon_url: c.icon_url,
            }
        }
    }

    impl From<pb::AgentCard> for AgentCard {
        fn from(c: pb::AgentCard) -> AgentCard {
            AgentCard {
                name: c.name,
                description: c.description,
                version: c.version,
                supported_interfaces: if c.supported_interfaces.is_empty() {
                    None
                } else {
                    Some(c.supported_interfaces.into_iter().map(Into::into).collect())
                },
                provider: c.provider.map(Into::into),
                capabilities: c.capabilities.map(Into::into).unwrap_or_default(),
                security_schemes: {
                    let m: HashMap<String, SecurityScheme> = c
                        .security_schemes
                        .into_iter()
                        .map(|(k, v)| (k, v.into()))
                        .collect();
                    if m.is_empty() { None } else { Some(m) }
                },
                security_requirements: if c.security_requirements.is_empty() {
                    None
                } else {
                    Some(
                        c.security_requirements
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                    )
                },
                default_input_modes: c.default_input_modes,
                default_output_modes: c.default_output_modes,
                skills: c.skills.into_iter().map(Into::into).collect(),
                signatures: if c.signatures.is_empty() {
                    None
                } else {
                    Some(c.signatures.into_iter().map(Into::into).collect())
                },
                documentation_url: c.documentation_url,
                icon_url: c.icon_url,
            }
        }
    }
}
