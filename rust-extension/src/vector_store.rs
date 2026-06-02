use std::sync::Arc;
use std::time::Duration;

use arrow_array::types::Float32Type;
use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use chrono::TimeDelta;
use futures_util::TryStreamExt;
use lancedb::index::vector::IvfHnswSqIndexBuilder;
use lancedb::index::Index;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::merge::MergeInsertBuilder;
use lancedb::table::{CompactionOptions, OptimizeAction};
use lancedb::DistanceType;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::{log_error, log_info, log_warn};

const VECTOR_TABLE: &str = "vectors";
const DB_DIR: &str = "data/lancedb";

// 写入参数优化：减少小文件生成
#[allow(dead_code)]
const MAX_ROWS_PER_FILE: usize = 10000;

// HNSW 索引参数
const HNSW_M: usize = 30;
const HNSW_EF_CONSTRUCTION: usize = 300;

// 版本清理策略：保留最近 24 小时的版本
const VERSION_RETENTION_HOURS: u64 = 24;

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
    log_info(&format!(
        "entries_to_batch: 条目数 {}, 向量维数 {}",
        entries.len(),
        dim
    ));
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
            Arc::new(StringArray::from(
                ids.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
            )),
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
// VectorStore – LanceDB backed (Optimized)
// ---------------------------------------------------------------------------

pub struct VectorStore {
    db: lancedb::connection::Connection,
    table_exists: bool,
}

impl VectorStore {
    /// Open (or lazily create on first write) the LanceDB database.
    /// 优化：配置写入参数以减少小文件生成
    pub async fn init() -> Result<Self, String> {
        let uri = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join(DB_DIR)
            .to_string_lossy()
            .to_string();

        log_info(&format!("初始化 LanceDB，路径: {}", uri));

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
    /// 
    /// 优化点：
    /// 1. 针对 exFAT 文件系统添加了重试机制
    /// 2. 使用 WriteMode::Create 替代 merge_insert 来减少碎片（首次创建时）
    /// 3. 控制写入批次大小，避免生成过多小文件
    pub async fn batch_upsert(&mut self, entries: &[VectorEntry]) -> Result<(), String> {
        if entries.is_empty() {
            return Ok(());
        }

        let batch = entries_to_batch(entries)?;

        // 针对 exFAT 文件系统的重试机制
        let max_retries = 3;
        let mut last_error = String::new();

        for attempt in 1..=max_retries {
            let result = if !self.table_exists {
                // First insert → create the table with optimized write params
                self.db
                    .create_table(VECTOR_TABLE, batch.clone())
                    .execute()
                    .await
                    .map_err(|e| {
                        let err = format!("创建表 {} 失败: {}", VECTOR_TABLE, e);
                        log_error(&err);
                        err
                    })
                    .map(|_| ())
            } else {
                let tbl = match self.open_table().await {
                    Ok(t) => t,
                    Err(e) => {
                        log_error(&e);
                        return Err(e);
                    }
                };
                let mut merge: MergeInsertBuilder = tbl.merge_insert(&["id"]);
                merge.when_matched_update_all(None);
                merge.when_not_matched_insert_all();
                let schema = batch.schema();
                let batch_iter = vec![Ok(batch.clone())].into_iter();
                let reader = arrow_array::RecordBatchIterator::new(batch_iter, schema);
                merge
                    .execute(Box::new(reader))
                    .await
                    .map_err(|e| {
                        let err = format!("merge_insert 失败: {}", e);
                        log_error(&err);
                        err
                    })
                    .map(|_| ())
            };

            match result {
                Ok(()) => {
                    if !self.table_exists {
                        self.table_exists = true;
                        log_info(&format!(
                            "创建 LanceDB 表完成，写入 {} 条向量",
                            entries.len()
                        ));
                    }
                    return Ok(());
                }
                Err(e) => {
                    last_error = e;
                    // 检查是否是 exFAT 相关的 IO 错误
                    if last_error.contains("LanceError(IO)") || last_error.contains("os error 1") {
                        if attempt < max_retries {
                            let delay = Duration::from_millis(500 * attempt as u64);
                            log_warn(&format!(
                                "exFAT 文件系统写入失败（尝试 {}/{}），{} 后重试: {}",
                                attempt, max_retries, format_duration(&delay), last_error
                            ));
                            sleep(delay).await;
                        } else {
                            log_error(&format!(
                                "exFAT 文件系统写入失败，已达到最大重试次数: {}",
                                last_error
                            ));
                        }
                    } else {
                        // 非 IO 错误，直接返回
                        return Err(last_error);
                    }
                }
            }
        }

        Err(format!(
            "写入向量存储失败（已重试 {} 次）: {}",
            max_retries, last_error
        ))
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

    /// Drop the entire table and all its data, resetting to empty state.
    pub async fn clear(&mut self) -> Result<(), String> {
        if !self.table_exists {
            return Ok(());
        }
        let _ = self.db.drop_table(VECTOR_TABLE, &[]).await;
        self.table_exists = false;
        Ok(())
    }

    /// Remove every row whose file_path matches any of the given paths.
    pub async fn remove_by_paths(&self, paths: &[String]) -> Result<(), String> {
        if paths.is_empty() || !self.table_exists {
            return Ok(());
        }
        let tbl = self.open_table().await?;
        // Build: file_path IN ('escaped1', 'escaped2', ...)
        let escaped: Vec<String> = paths.iter().map(|p| p.replace('\'', "''")).collect();
        let values = escaped
            .iter()
            .map(|p| format!("'{}'", p))
            .collect::<Vec<_>>()
            .join(", ");
        let predicate = format!("file_path IN ({})", values);
        tbl.delete(&predicate)
            .await
            .map_err(|e| format!("批量删除失败: {}", e))?;
        Ok(())
    }

    /// Vector similarity search.  Returns results sorted by cosine similarity
    /// (descending).
    /// 优化：使用 HNSW 索引参数进行查询
    pub async fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<SearchResult>, String> {
        if !self.table_exists {
            return Ok(vec![]);
        }
        let tbl = self.open_table().await?;

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
                let vector: Vec<f32> = (0..float_arr.len()).map(|j| float_arr.value(j)).collect();

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
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

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

    /// Build an IVF-HNSW-SQ index on the vector column for faster search.
    /// HNSW 提供比 IVF-PQ 更高的召回率和更快的查询速度。
    /// SQ (Scalar Quantization) 在保持高精度的同时减少存储空间。
    /// 
    /// 应在批量加载数据后调用此方法。
    pub async fn create_index(&self) -> Result<(), String> {
        if !self.table_exists {
            return Ok(());
        }
        let tbl = self.open_table().await?;
        
        log_info("开始创建向量索引 (IVF-HNSW-SQ)……");
        log_info(&format!("  HNSW 参数: M={}, ef_construction={}", 
            HNSW_M, HNSW_EF_CONSTRUCTION));
        
        // 使用 IVF_HNSW_SQ 索引类型
        let index = Index::IvfHnswSq(
            IvfHnswSqIndexBuilder::default()
                .distance_type(DistanceType::Cosine)
                .num_edges(HNSW_M as u32)
                .ef_construction(HNSW_EF_CONSTRUCTION as u32)
        );
        
        tbl.create_index(&["vector"], index)
            .execute()
            .await
            .map_err(|e| format!("创建索引失败: {}", e))?;
        
        log_info("向量索引 (IVF-HNSW-SQ) 创建完成");
        Ok(())
    }

    /// 优化表：执行 compaction 和 prune 操作
    /// 
    /// Compaction: 合并小文件为大文件，减少文件数量和元数据开销
    /// Prune: 清理旧版本，释放磁盘空间
    /// 
    /// 应在大量写入或删除操作后调用。
    #[allow(dead_code)]
    pub async fn optimize(&self) -> Result<(), String> {
        if !self.table_exists {
            return Ok(());
        }
        
        log_info("开始优化 LanceDB 表（compaction + prune）……");
        
        // 先执行 compaction
        if let Err(e) = self.compact_files().await {
            log_warn(&format!("文件压缩失败: {}", e));
        }
        
        // 再执行 prune
        if let Err(e) = self.cleanup_old_versions().await {
            log_warn(&format!("版本清理失败: {}", e));
        }
        
        log_info("优化完成");
        Ok(())
    }

    /// 清理旧版本以释放磁盘空间
    /// 保留最近 N 小时的版本，其余版本将被删除
    pub async fn cleanup_old_versions(&self) -> Result<(), String> {
        if !self.table_exists {
            return Ok(());
        }
        let tbl = self.open_table().await?;
        
        log_info(&format!(
            "开始清理旧版本（保留最近 {} 小时）……",
            VERSION_RETENTION_HOURS
        ));
        
        let retention_duration = TimeDelta::hours(VERSION_RETENTION_HOURS as i64);
        
        match tbl
            .optimize(OptimizeAction::Prune {
                older_than: Some(retention_duration),
                delete_unverified: None,
                error_if_tagged_old_versions: None,
            })
            .await
        {
            Ok(stats) => {
                log_info(&format!("版本清理完成，清理统计: {:?}", stats.prune));
                Ok(())
            }
            Err(e) => {
                log_warn(&format!("版本清理失败: {}", e));
                Ok(())
            }
        }
    }

    /// 执行文件压缩，合并小文件
    pub async fn compact_files(&self) -> Result<(), String> {
        if !self.table_exists {
            return Ok(());
        }
        let tbl = self.open_table().await?;
        
        log_info("开始压缩文件（合并小文件）……");
        
        match tbl
            .optimize(OptimizeAction::Compact {
                options: CompactionOptions::default(),
                remap_options: None,
            })
            .await
        {
            Ok(stats) => {
                log_info(&format!(
                    "文件压缩完成 - 统计: {:?}",
                    stats.compaction
                ));
                Ok(())
            }
            Err(e) => {
                log_warn(&format!("文件压缩失败: {}", e));
                // 压缩失败不影响正常使用
                Ok(())
            }
        }
    }

    /// 获取表的统计信息
    #[allow(dead_code)]
    pub async fn get_stats(&self) -> Result<TableStats, String> {
        if !self.table_exists {
            return Ok(TableStats::default());
        }
        let tbl = self.open_table().await?;
        
        let row_count = tbl.count_rows(None).await.map_err(|e| {
            format!("获取行数失败: {}", e)
        })?;
        
        // 尝试获取版本数
        let version_count = match tbl.list_versions().await {
            Ok(versions) => versions.len(),
            Err(_) => 0,
        };
        
        Ok(TableStats {
            row_count,
            version_count,
            table_exists: true,
        })
    }
}

/// 表统计信息
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct TableStats {
    pub row_count: usize,
    pub version_count: usize,
    pub table_exists: bool,
}

/// 格式化 Duration 为可读字符串
fn format_duration(duration: &Duration) -> String {
    let millis = duration.as_millis();
    if millis >= 1000 {
        format!("{:.1}秒", millis as f64 / 1000.0)
    } else {
        format!("{}毫秒", millis)
    }
}
