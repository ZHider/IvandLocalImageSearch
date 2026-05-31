use std::sync::Arc;

use arrow_array::types::Float32Type;
use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use futures_util::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::merge::MergeInsertBuilder;
use lancedb::DistanceType;
use serde::{Deserialize, Serialize};

use crate::log_info;

const VECTOR_TABLE: &str = "vectors";
const DB_DIR: &str = "data/lancedb";

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entry: VectorEntry,
    /// Cosine similarity in [0, 1], higher = more similar.
    pub score: f32,
}

// ---------------------------------------------------------------------------
// Schema helpers
// ---------------------------------------------------------------------------

fn make_schema(vector_dim: i32) -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                vector_dim,
            ),
            true,
        ),
        Field::new("file_path", DataType::Utf8, false),
        Field::new("file_type", DataType::Utf8, false),
        Field::new("metadata", DataType::Utf8, false),
    ]))
}

/// Build a RecordBatch from `VectorEntry` items.
fn entries_to_batch(entries: &[VectorEntry]) -> Result<RecordBatch, String> {
    if entries.is_empty() {
        return Err("entries_to_batch: empty slice".into());
    }
    let dim = entries[0].vector.len() as i32;
    log_info(&format!("entries_to_batch: 条目数 {}, 向量维数 {}", entries.len(), dim));
    let schema = make_schema(dim);

    let n = entries.len();
    let mut ids = Vec::with_capacity(n);
    let mut all_vectors: Vec<Option<f32>> = Vec::with_capacity(n * dim as usize);
    let mut file_paths = Vec::with_capacity(n);
    let mut file_types = Vec::with_capacity(n);
    let mut metadata_json = Vec::with_capacity(n);

    for e in entries {
        ids.push(&e.id);
        // Pad / truncate vectors to the expected dimension
        let vec_data: Vec<f32> = if e.vector.len() as i32 >= dim {
            e.vector[..dim as usize].to_vec()
        } else {
            let mut v = e.vector.clone();
            v.resize(dim as usize, 0.0);
            v
        };
        for val in &vec_data {
            all_vectors.push(Some(*val));
        }
        let fp = e.metadata["file_path"]
            .as_str()
            .unwrap_or(&e.id)
            .to_string();
        file_paths.push(fp);
        let ft = e.metadata["file_type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        file_types.push(ft);
        metadata_json.push(serde_json::to_string(&e.metadata).unwrap_or_else(|_| "{}".into()));
    }

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(ids.iter().map(|s| s.as_str()).collect::<Vec<&str>>())),
            Arc::new(
                FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                    all_vectors.chunks(dim as usize).map(|chunk| {
                        let arr: Vec<Option<f32>> = chunk.to_vec();
                        Some(arr)
                    }),
                    dim,
                ),
            ),
            Arc::new(StringArray::from(file_paths)),
            Arc::new(StringArray::from(file_types)),
            Arc::new(StringArray::from(metadata_json)),
        ],
    )
    .map_err(|e| format!("创建 RecordBatch 失败: {}", e))?;

    Ok(batch)
}

/// Convert a _distance value (Cosine distance from LanceDB) to a cosine
/// similarity score in [0, 1] — higher is better.
fn distance_to_score(dist: f32) -> f32 {
    // Cosine distance = 1 - cosine_similarity → clamped to [0, 1]
    (1.0 - dist).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// VectorStore – LanceDB backed
// ---------------------------------------------------------------------------

pub struct VectorStore {
    db: lancedb::connection::Connection,
    table_exists: bool,
}

impl VectorStore {
    /// Open (or lazily create on first write) the LanceDB database.
    pub async fn init() -> Result<Self, String> {
        let uri = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(DB_DIR)
            .to_string_lossy()
            .to_string();

        let db = lancedb::connect(&uri)
            .execute()
            .await
            .map_err(|e| format!("连接 LanceDB 失败: {}", e))?;

        let exists = db.open_table(VECTOR_TABLE).execute().await.is_ok();
        if exists {
            log_info(&format!("打开已有 LanceDB 表: {}", VECTOR_TABLE));
        } else {
            log_info("LanceDB 表尚不存在，首次写入时将自动创建");
        }

        Ok(VectorStore {
            db,
            table_exists: exists,
        })
    }

    async fn open_table(&self) -> Result<lancedb::Table, String> {
        self.db
            .open_table(VECTOR_TABLE)
            .execute()
            .await
            .map_err(|e| format!("打开表 {} 失败: {}", VECTOR_TABLE, e))
    }

    /// Batch upsert entries.  Uses `merge_insert` on the `id` column so
    /// existing rows are replaced.
    pub async fn batch_upsert(&mut self, entries: &[VectorEntry]) -> Result<(), String> {
        if entries.is_empty() {
            return Ok(());
        }

        let batch = entries_to_batch(entries)?;

        if !self.table_exists {
            // First insert → create the table
            self.db
                .create_table(VECTOR_TABLE, batch)
                .execute()
                .await
                .map_err(|e| format!("创建表 {} 失败: {}", VECTOR_TABLE, e))?;
            self.table_exists = true;
            log_info(&format!(
                "创建 LanceDB 表完成，写入 {} 条向量",
                entries.len()
            ));
        } else {
            let tbl = self.open_table().await?;
            let mut merge: MergeInsertBuilder = tbl.merge_insert(&["id"]);
            merge.when_matched_update_all(None);
            merge.when_not_matched_insert_all();
            let schema = batch.schema();
            let batch_iter = vec![Ok(batch)].into_iter();
            let reader =
                arrow_array::RecordBatchIterator::new(batch_iter, schema);
            merge
                .execute(Box::new(reader))
                .await
                .map_err(|e| format!("merge_insert 失败: {}", e))?;
        }

        Ok(())
    }

    /// Remove every row whose file_path equals the given path.
    pub async fn remove_by_file_path(&self, file_path: &str) -> Result<(), String> {
        if !self.table_exists {
            return Ok(());
        }
        let tbl = self.open_table().await?;
        // Escape single quotes in the path for the SQL predicate
        let escaped = file_path.replace('\'', "''");
        let predicate = format!("file_path = '{}'", escaped);
        tbl.delete(&predicate)
            .await
            .map_err(|e| format!("删除失败: {}", e))?;
        Ok(())
    }

    /// Vector similarity search.  Returns results sorted by cosine similarity
    /// (descending).
    pub async fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<SearchResult>, String> {
        if !self.table_exists {
            return Ok(vec![]);
        }
        let tbl = self.open_table().await?;

        // We need a cosine-distance query.  The simplest way is to query with
        // Euclidean distance on normalized vectors, or use the distance_type
        // builder.  Let's use distance_type(Cosine).
        //
        // Note: `.nearest_to()` returns a VectorQuery.  We chain the distance
        // type and limit before executing.
        let batch_stream = tbl
            .query()
            .nearest_to(query)
            .map_err(|e| format!("设置查询向量失败: {}", e))?
            .distance_type(DistanceType::Cosine)
            .limit(top_k)
            .execute()
            .await
            .map_err(|e| format!("向量搜索执行失败: {}", e))?;

        let batches: Vec<RecordBatch> = batch_stream
            .try_collect()
            .await
            .map_err(|e| format!("收集搜索结果失败: {}", e))?;

        let mut results = Vec::new();
        for rb in &batches {
            let n = rb.num_rows();
            if n == 0 {
                continue;
            }

            let ids = rb
                .column_by_name("id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .ok_or_else(|| "搜索结果缺少 id 列".to_string())?;

            let metadata_col = rb
                .column_by_name("metadata")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .ok_or_else(|| "搜索结果缺少 metadata 列".to_string())?;

            let vector_col = rb
                .column_by_name("vector")
                .and_then(|c| c.as_any().downcast_ref::<FixedSizeListArray>())
                .ok_or_else(|| "搜索结果缺少 vector 列".to_string())?;

            let dist_col = rb
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>())
                .ok_or_else(|| "搜索结果缺少 _distance 列".to_string())?;

            for i in 0..n {
                let id = ids.value(i);
                let meta_str = metadata_col.value(i);

                let metadata: serde_json::Value =
                    serde_json::from_str(meta_str).unwrap_or_else(|_| serde_json::json!({}));
                let dist = dist_col.value(i);
                let score = distance_to_score(dist);

                // Extract the vector from FixedSizeList
                let vec_arr = vector_col.value(i);
                let float_arr = vec_arr
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .ok_or_else(|| "向量列类型转换失败".to_string())?;
                let vector: Vec<f32> = (0..float_arr.len())
                    .map(|j| float_arr.value(j))
                    .collect();

                results.push(SearchResult {
                    entry: VectorEntry {
                        id: id.to_string(),
                        vector,
                        metadata,
                    },
                    score,
                });
            }
        }

        // Sort by score descending (already ordered by LanceDB distance,
        // but let's be explicit)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    /// Return the count of rows in the table (0 if table doesn't exist).
    pub async fn count(&self) -> Result<usize, String> {
        if !self.table_exists {
            return Ok(0);
        }
        let tbl = self.open_table().await?;
        tbl.count_rows(None)
            .await
            .map_err(|e| format!("count_rows 失败: {}", e))
    }

    /// Build an IVF-PQ index on the vector column for faster search.
    /// Call this once after bulk-loading data.
    pub async fn create_index(&self) -> Result<(), String> {
        if !self.table_exists {
            return Ok(());
        }
        let tbl = self.open_table().await?;
        log_info("开始创建向量索引 (IVF-PQ)……");
        tbl.create_index(&["vector"], lancedb::index::Index::Auto)
            .execute()
            .await
            .map_err(|e| format!("创建索引失败: {}", e))?;
        log_info("向量索引创建完成");
        Ok(())
    }
}
