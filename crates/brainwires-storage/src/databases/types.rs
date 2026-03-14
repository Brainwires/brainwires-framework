//! Schema, record, and filter types for the unified database layer.
//!
//! These types define a backend-agnostic data model that both
//! [`StorageBackend`](super::traits::StorageBackend) and
//! [`VectorDatabase`](super::traits::VectorDatabase) implementations use.

// ── Schema types ────────────────────────────────────────────────────────

/// Definition of a single field within a table schema.
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// Column name.
    pub name: String,
    /// Data type.
    pub field_type: FieldType,
    /// Whether `NULL` / `None` values are permitted.
    pub nullable: bool,
}

impl FieldDef {
    /// Shorthand constructor for a non-nullable field.
    pub fn required(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            nullable: false,
        }
    }

    /// Shorthand constructor for a nullable field.
    pub fn optional(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            nullable: true,
        }
    }
}

/// Supported column data types.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    /// UTF-8 string.
    Utf8,
    /// 32-bit signed integer.
    Int32,
    /// 64-bit signed integer.
    Int64,
    /// 32-bit unsigned integer.
    UInt32,
    /// 64-bit unsigned integer.
    UInt64,
    /// 32-bit floating point.
    Float32,
    /// 64-bit floating point.
    Float64,
    /// Boolean.
    Boolean,
    /// Fixed-size float vector with the given dimension (for embeddings).
    Vector(usize),
}

// ── Record types ────────────────────────────────────────────────────────

/// A single typed column value.
#[derive(Debug, Clone)]
pub enum FieldValue {
    /// UTF-8 string (nullable).
    Utf8(Option<String>),
    /// 32-bit signed integer (nullable).
    Int32(Option<i32>),
    /// 64-bit signed integer (nullable).
    Int64(Option<i64>),
    /// 32-bit unsigned integer (nullable).
    UInt32(Option<u32>),
    /// 64-bit unsigned integer (nullable).
    UInt64(Option<u64>),
    /// 32-bit floating point (nullable).
    Float32(Option<f32>),
    /// 64-bit floating point (nullable).
    Float64(Option<f64>),
    /// Boolean (nullable).
    Boolean(Option<bool>),
    /// Dense float vector (for embeddings). Empty vec means NULL.
    Vector(Vec<f32>),
}

impl FieldValue {
    /// Return the value as a string reference, if it is `Utf8(Some(_))`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            FieldValue::Utf8(Some(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Return the value as an `i64`, if it is `Int64(Some(_))`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            FieldValue::Int64(Some(v)) => Some(*v),
            _ => None,
        }
    }

    /// Return the value as an `i32`, if it is `Int32(Some(_))`.
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            FieldValue::Int32(Some(v)) => Some(*v),
            _ => None,
        }
    }

    /// Return the value as an `f32`, if it is `Float32(Some(_))`.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            FieldValue::Float32(Some(v)) => Some(*v),
            _ => None,
        }
    }

    /// Return the value as an `f64`, if it is `Float64(Some(_))`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            FieldValue::Float64(Some(v)) => Some(*v),
            _ => None,
        }
    }

    /// Return the value as a `bool`, if it is `Boolean(Some(_))`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            FieldValue::Boolean(Some(v)) => Some(*v),
            _ => None,
        }
    }

    /// Return the value as a float vector reference.
    pub fn as_vector(&self) -> Option<&[f32]> {
        match self {
            FieldValue::Vector(v) if !v.is_empty() => Some(v.as_slice()),
            _ => None,
        }
    }
}

/// A generic row: ordered list of `(column_name, value)` pairs.
pub type Record = Vec<(String, FieldValue)>;

/// Helper to look up a field in a [`Record`] by name.
pub fn record_get<'a>(record: &'a Record, name: &str) -> Option<&'a FieldValue> {
    record.iter().find(|(n, _)| n == name).map(|(_, v)| v)
}

/// A record returned from a vector similarity search, with a relevance score.
#[derive(Debug, Clone)]
pub struct ScoredRecord {
    /// The matched row.
    pub record: Record,
    /// Similarity score (higher is better, typically 0.0–1.0).
    pub score: f32,
}

// ── Filter types ────────────────────────────────────────────────────────

/// Structured query filter that backends translate into their native syntax.
///
/// Use [`Filter::Raw`] as an escape hatch for backend-specific expressions.
#[derive(Debug, Clone)]
pub enum Filter {
    /// Column equals value.
    Eq(String, FieldValue),
    /// Column does not equal value.
    Ne(String, FieldValue),
    /// Column is less than value.
    Lt(String, FieldValue),
    /// Column is less than or equal to value.
    Lte(String, FieldValue),
    /// Column is greater than value.
    Gt(String, FieldValue),
    /// Column is greater than or equal to value.
    Gte(String, FieldValue),
    /// Column is NOT NULL.
    NotNull(String),
    /// Column IS NULL.
    IsNull(String),
    /// Column value is in the given list.
    In(String, Vec<FieldValue>),
    /// All sub-filters must match.
    And(Vec<Filter>),
    /// At least one sub-filter must match.
    Or(Vec<Filter>),
    /// Raw backend-specific filter string (escape hatch).
    Raw(String),
}
