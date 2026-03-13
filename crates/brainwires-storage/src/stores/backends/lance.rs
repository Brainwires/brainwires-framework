//! LanceDB implementation of [`StorageBackend`].
//!
//! This is the default backend — an embedded vector database that requires no
//! external server. It translates the generic [`Record`] / [`Filter`] types
//! into Arrow `RecordBatch` and LanceDB filter strings.

use anyhow::{Context, Result};
use arrow_array::{
    Array, BooleanArray, FixedSizeListArray, Float32Array, Float64Array, Int32Array, Int64Array,
    RecordBatch, RecordBatchIterator, StringArray, UInt32Array, UInt64Array,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

use crate::stores::backend::{
    FieldDef, FieldType, FieldValue, Filter, Record, ScoredRecord, StorageBackend,
};

/// LanceDB-backed [`StorageBackend`] implementation.
///
/// Wraps a LanceDB [`Connection`] and performs all Arrow serialization
/// centrally so that domain stores remain backend-agnostic.
pub struct LanceBackend {
    connection: Connection,
    db_path: String,
}

impl LanceBackend {
    /// Create a new LanceDB backend at the given path.
    pub async fn new(db_path: impl Into<String>) -> Result<Self> {
        let db_path = db_path.into();

        if let Some(parent) = std::path::Path::new(&db_path).parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        let connection = lancedb::connect(&db_path)
            .execute()
            .await
            .context("Failed to connect to LanceDB")?;

        Ok(Self {
            connection,
            db_path,
        })
    }

    /// Get the underlying LanceDB connection (for legacy code that still needs it).
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Get the database path.
    pub fn db_path(&self) -> &str {
        &self.db_path
    }
}

// ── StorageBackend impl ─────────────────────────────────────────────────

#[async_trait::async_trait]
impl StorageBackend for LanceBackend {
    async fn ensure_table(&self, table_name: &str, schema: &[FieldDef]) -> Result<()> {
        let table_names = self.connection.table_names().execute().await?;
        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        let arrow_schema = Arc::new(field_defs_to_schema(schema));
        let batches = RecordBatchIterator::new(vec![], arrow_schema);
        self.connection
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .with_context(|| format!("Failed to create table '{table_name}'"))?;
        Ok(())
    }

    async fn insert(&self, table_name: &str, records: Vec<Record>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let table = self
            .connection
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open table '{table_name}'"))?;

        let batch = records_to_batch(&records)?;
        let schema = batch.schema();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        table
            .add(Box::new(batches))
            .execute()
            .await
            .with_context(|| format!("Failed to insert into '{table_name}'"))?;
        Ok(())
    }

    async fn query(
        &self,
        table_name: &str,
        filter: Option<&Filter>,
        limit: Option<usize>,
    ) -> Result<Vec<Record>> {
        let table = self
            .connection
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open table '{table_name}'"))?;

        let mut q = table.query();
        if let Some(f) = filter {
            q = q.only_if(filter_to_sql(f));
        }
        if let Some(n) = limit {
            q = q.limit(n);
        }

        let batches: Vec<RecordBatch> = q
            .execute()
            .await
            .with_context(|| format!("Failed to query '{table_name}'"))?
            .try_collect()
            .await?;

        let mut results = Vec::new();
        for batch in &batches {
            batch_to_records(batch, &mut results)?;
        }
        Ok(results)
    }

    async fn delete(&self, table_name: &str, filter: &Filter) -> Result<()> {
        let table = self
            .connection
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open table '{table_name}'"))?;

        table
            .delete(&filter_to_sql(filter))
            .await
            .with_context(|| format!("Failed to delete from '{table_name}'"))?;
        Ok(())
    }

    async fn count(&self, table_name: &str, filter: Option<&Filter>) -> Result<usize> {
        let table = self
            .connection
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open table '{table_name}'"))?;

        let mut q = table.query();
        if let Some(f) = filter {
            q = q.only_if(filter_to_sql(f));
        }
        let batches: Vec<RecordBatch> = q.execute().await?.try_collect().await?;
        Ok(batches.iter().map(|b| b.num_rows()).sum())
    }

    async fn vector_search(
        &self,
        table_name: &str,
        _vector_column: &str,
        vector: Vec<f32>,
        limit: usize,
        filter: Option<&Filter>,
    ) -> Result<Vec<ScoredRecord>> {
        let table = self
            .connection
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open table '{table_name}'"))?;

        let mut q = table.vector_search(vector)?;
        q = q.limit(limit);
        if let Some(f) = filter {
            q = q.only_if(filter_to_sql(f));
        }

        let batches: Vec<RecordBatch> = q.execute().await?.try_collect().await?;

        let mut results = Vec::new();
        for batch in &batches {
            let distance_col = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

            for row in 0..batch.num_rows() {
                let mut record = Vec::new();
                for (col_idx, field) in batch.schema().fields().iter().enumerate() {
                    if field.name() == "_distance" {
                        continue;
                    }
                    let val = extract_field_value(batch, col_idx, row, field)?;
                    record.push((field.name().clone(), val));
                }

                let distance = distance_col.map_or(0.0, |c| c.value(row));
                let score = 1.0 / (1.0 + distance);

                results.push(ScoredRecord { record, score });
            }
        }
        Ok(results)
    }
}

// ── Conversion helpers ──────────────────────────────────────────────────

/// Convert [`FieldDef`] slice to an Arrow [`Schema`].
fn field_defs_to_schema(defs: &[FieldDef]) -> Schema {
    let fields: Vec<Field> = defs
        .iter()
        .map(|d| {
            let dt = match &d.field_type {
                FieldType::Utf8 => DataType::Utf8,
                FieldType::Int32 => DataType::Int32,
                FieldType::Int64 => DataType::Int64,
                FieldType::UInt32 => DataType::UInt32,
                FieldType::UInt64 => DataType::UInt64,
                FieldType::Float32 => DataType::Float32,
                FieldType::Float64 => DataType::Float64,
                FieldType::Boolean => DataType::Boolean,
                FieldType::Vector(dim) => DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    *dim as i32,
                ),
            };
            Field::new(&d.name, dt, d.nullable)
        })
        .collect();
    Schema::new(fields)
}

/// Convert a batch of [`Record`]s to an Arrow [`RecordBatch`].
///
/// All records must have the same columns in the same order.
fn records_to_batch(records: &[Record]) -> Result<RecordBatch> {
    if records.is_empty() {
        anyhow::bail!("Cannot create RecordBatch from zero records");
    }

    let first = &records[0];
    let num_rows = records.len();

    // Build schema and column arrays from the first record's structure.
    let mut fields = Vec::with_capacity(first.len());
    let mut columns: Vec<Arc<dyn Array>> = Vec::with_capacity(first.len());

    for (col_idx, (name, sample)) in first.iter().enumerate() {
        match sample {
            FieldValue::Utf8(_) => {
                let values: Vec<Option<&str>> = records
                    .iter()
                    .map(|r| match &r[col_idx].1 {
                        FieldValue::Utf8(v) => v.as_deref(),
                        _ => None,
                    })
                    .collect();
                let nullable = values.iter().any(|v| v.is_none());
                fields.push(Field::new(name, DataType::Utf8, nullable));
                columns.push(Arc::new(StringArray::from(values)));
            }
            FieldValue::Int32(_) => {
                let values: Vec<Option<i32>> = records
                    .iter()
                    .map(|r| match &r[col_idx].1 {
                        FieldValue::Int32(v) => *v,
                        _ => None,
                    })
                    .collect();
                let nullable = values.iter().any(|v| v.is_none());
                fields.push(Field::new(name, DataType::Int32, nullable));
                columns.push(Arc::new(Int32Array::from(values)));
            }
            FieldValue::Int64(_) => {
                let values: Vec<Option<i64>> = records
                    .iter()
                    .map(|r| match &r[col_idx].1 {
                        FieldValue::Int64(v) => *v,
                        _ => None,
                    })
                    .collect();
                let nullable = values.iter().any(|v| v.is_none());
                fields.push(Field::new(name, DataType::Int64, nullable));
                columns.push(Arc::new(Int64Array::from(values)));
            }
            FieldValue::UInt32(_) => {
                let values: Vec<Option<u32>> = records
                    .iter()
                    .map(|r| match &r[col_idx].1 {
                        FieldValue::UInt32(v) => *v,
                        _ => None,
                    })
                    .collect();
                let nullable = values.iter().any(|v| v.is_none());
                fields.push(Field::new(name, DataType::UInt32, nullable));
                columns.push(Arc::new(UInt32Array::from(values)));
            }
            FieldValue::UInt64(_) => {
                let values: Vec<Option<u64>> = records
                    .iter()
                    .map(|r| match &r[col_idx].1 {
                        FieldValue::UInt64(v) => *v,
                        _ => None,
                    })
                    .collect();
                let nullable = values.iter().any(|v| v.is_none());
                fields.push(Field::new(name, DataType::UInt64, nullable));
                columns.push(Arc::new(UInt64Array::from(values)));
            }
            FieldValue::Float32(_) => {
                let values: Vec<Option<f32>> = records
                    .iter()
                    .map(|r| match &r[col_idx].1 {
                        FieldValue::Float32(v) => *v,
                        _ => None,
                    })
                    .collect();
                let nullable = values.iter().any(|v| v.is_none());
                fields.push(Field::new(name, DataType::Float32, nullable));
                columns.push(Arc::new(Float32Array::from(values)));
            }
            FieldValue::Float64(_) => {
                let values: Vec<Option<f64>> = records
                    .iter()
                    .map(|r| match &r[col_idx].1 {
                        FieldValue::Float64(v) => *v,
                        _ => None,
                    })
                    .collect();
                let nullable = values.iter().any(|v| v.is_none());
                fields.push(Field::new(name, DataType::Float64, nullable));
                columns.push(Arc::new(Float64Array::from(values)));
            }
            FieldValue::Boolean(_) => {
                let values: Vec<Option<bool>> = records
                    .iter()
                    .map(|r| match &r[col_idx].1 {
                        FieldValue::Boolean(v) => *v,
                        _ => None,
                    })
                    .collect();
                let nullable = values.iter().any(|v| v.is_none());
                fields.push(Field::new(name, DataType::Boolean, nullable));
                columns.push(Arc::new(BooleanArray::from(values)));
            }
            FieldValue::Vector(sample_vec) => {
                let dim = sample_vec.len() as i32;
                // Collect all float values into a flat buffer.
                let mut flat_values: Vec<f32> = Vec::with_capacity(num_rows * dim as usize);
                for r in records {
                    match &r[col_idx].1 {
                        FieldValue::Vector(v) => flat_values.extend_from_slice(v),
                        _ => flat_values.extend(std::iter::repeat_n(0.0f32, dim as usize)),
                    }
                }
                let values_array = Float32Array::from(flat_values);
                let list_field = Arc::new(Field::new("item", DataType::Float32, true));
                let list = FixedSizeListArray::try_new(
                    list_field.clone(),
                    dim,
                    Arc::new(values_array),
                    None,
                )?;
                fields.push(Field::new(
                    name,
                    DataType::FixedSizeList(list_field, dim),
                    false,
                ));
                columns.push(Arc::new(list));
            }
        }
    }

    let schema = Arc::new(Schema::new(fields));
    Ok(RecordBatch::try_new(schema, columns)?)
}

/// Extract all rows from a [`RecordBatch`] into [`Record`]s.
fn batch_to_records(batch: &RecordBatch, out: &mut Vec<Record>) -> Result<()> {
    for row in 0..batch.num_rows() {
        let mut record = Vec::with_capacity(batch.num_columns());
        for (col_idx, field) in batch.schema().fields().iter().enumerate() {
            let val = extract_field_value(batch, col_idx, row, field)?;
            record.push((field.name().clone(), val));
        }
        out.push(record);
    }
    Ok(())
}

/// Extract a single [`FieldValue`] from a batch cell.
fn extract_field_value(
    batch: &RecordBatch,
    col_idx: usize,
    row: usize,
    field: &Field,
) -> Result<FieldValue> {
    let col = batch.column(col_idx);

    Ok(match field.data_type() {
        DataType::Utf8 => {
            let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
            if arr.is_null(row) {
                FieldValue::Utf8(None)
            } else {
                FieldValue::Utf8(Some(arr.value(row).to_string()))
            }
        }
        DataType::Int32 => {
            let arr = col.as_any().downcast_ref::<Int32Array>().unwrap();
            if arr.is_null(row) {
                FieldValue::Int32(None)
            } else {
                FieldValue::Int32(Some(arr.value(row)))
            }
        }
        DataType::Int64 => {
            let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
            if arr.is_null(row) {
                FieldValue::Int64(None)
            } else {
                FieldValue::Int64(Some(arr.value(row)))
            }
        }
        DataType::UInt32 => {
            let arr = col.as_any().downcast_ref::<UInt32Array>().unwrap();
            if arr.is_null(row) {
                FieldValue::UInt32(None)
            } else {
                FieldValue::UInt32(Some(arr.value(row)))
            }
        }
        DataType::UInt64 => {
            let arr = col.as_any().downcast_ref::<UInt64Array>().unwrap();
            if arr.is_null(row) {
                FieldValue::UInt64(None)
            } else {
                FieldValue::UInt64(Some(arr.value(row)))
            }
        }
        DataType::Float32 => {
            let arr = col.as_any().downcast_ref::<Float32Array>().unwrap();
            if arr.is_null(row) {
                FieldValue::Float32(None)
            } else {
                FieldValue::Float32(Some(arr.value(row)))
            }
        }
        DataType::Float64 => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            if arr.is_null(row) {
                FieldValue::Float64(None)
            } else {
                FieldValue::Float64(Some(arr.value(row)))
            }
        }
        DataType::Boolean => {
            let arr = col.as_any().downcast_ref::<BooleanArray>().unwrap();
            if arr.is_null(row) {
                FieldValue::Boolean(None)
            } else {
                FieldValue::Boolean(Some(arr.value(row)))
            }
        }
        DataType::FixedSizeList(_, _dim) => {
            let arr = col.as_any().downcast_ref::<FixedSizeListArray>().unwrap();
            if arr.is_null(row) {
                FieldValue::Vector(Vec::new())
            } else {
                let inner = arr.value(row);
                let floats = inner.as_any().downcast_ref::<Float32Array>().unwrap();
                FieldValue::Vector(floats.values().to_vec())
            }
        }
        other => {
            // Fallback: try to read as string
            tracing::warn!("Unsupported Arrow data type {other:?}, reading as Utf8");
            FieldValue::Utf8(Some(format!("{:?}", col)))
        }
    })
}

/// Convert a [`Filter`] to a LanceDB SQL filter string.
fn filter_to_sql(filter: &Filter) -> String {
    match filter {
        Filter::Eq(col, val) => format!("{col} = {}", value_to_sql(val)),
        Filter::Ne(col, val) => format!("{col} != {}", value_to_sql(val)),
        Filter::Lt(col, val) => format!("{col} < {}", value_to_sql(val)),
        Filter::Lte(col, val) => format!("{col} <= {}", value_to_sql(val)),
        Filter::Gt(col, val) => format!("{col} > {}", value_to_sql(val)),
        Filter::Gte(col, val) => format!("{col} >= {}", value_to_sql(val)),
        Filter::NotNull(col) => format!("{col} IS NOT NULL"),
        Filter::IsNull(col) => format!("{col} IS NULL"),
        Filter::In(col, vals) => {
            let items: Vec<String> = vals.iter().map(value_to_sql).collect();
            format!("{col} IN ({})", items.join(", "))
        }
        Filter::And(parts) => {
            let clauses: Vec<String> = parts.iter().map(filter_to_sql).collect();
            format!("({})", clauses.join(" AND "))
        }
        Filter::Or(parts) => {
            let clauses: Vec<String> = parts.iter().map(filter_to_sql).collect();
            format!("({})", clauses.join(" OR "))
        }
        Filter::Raw(s) => s.clone(),
    }
}

/// Convert a [`FieldValue`] to a SQL literal.
fn value_to_sql(val: &FieldValue) -> String {
    match val {
        FieldValue::Utf8(Some(s)) => format!("'{}'", s.replace('\'', "''")),
        FieldValue::Utf8(None) => "NULL".to_string(),
        FieldValue::Int32(Some(v)) => v.to_string(),
        FieldValue::Int32(None) => "NULL".to_string(),
        FieldValue::Int64(Some(v)) => v.to_string(),
        FieldValue::Int64(None) => "NULL".to_string(),
        FieldValue::UInt32(Some(v)) => v.to_string(),
        FieldValue::UInt32(None) => "NULL".to_string(),
        FieldValue::UInt64(Some(v)) => v.to_string(),
        FieldValue::UInt64(None) => "NULL".to_string(),
        FieldValue::Float32(Some(v)) => v.to_string(),
        FieldValue::Float32(None) => "NULL".to_string(),
        FieldValue::Float64(Some(v)) => v.to_string(),
        FieldValue::Float64(None) => "NULL".to_string(),
        FieldValue::Boolean(Some(v)) => if *v { "TRUE" } else { "FALSE" }.to_string(),
        FieldValue::Boolean(None) => "NULL".to_string(),
        FieldValue::Vector(_) => "NULL".to_string(), // vectors aren't used in filters
    }
}
