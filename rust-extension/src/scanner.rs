use serde::Serialize;
use std::path::Path;
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

use crate::hasher;
use crate::log_info;

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub file_path: String,
    pub file_size: u64,
    pub modified_at: u64,
    pub file_hash: String,
}

use crate::file_utils;

fn is_valid_file(path: &Path) -> bool {
    file_utils::is_valid_file(path)
}
pub struct ScanResult {
    pub files: Vec<FileEntry>,
    pub total: usize,
}

pub fn scan_folders(folders: &[String]) -> Result<ScanResult, String> {
    let mut files = Vec::new();

    for folder in folders {
        let folder_path = Path::new(folder);
        if !folder_path.exists() || !folder_path.is_dir() {
            log_info(&format!("跳过不存在的目录: {}", folder));
            continue;
        }

        log_info(&format!("开始扫描目录: {}", folder));

        for entry in WalkDir::new(folder)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if !is_valid_file(path) {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    log_info(&format!("无法获取文件元数据 {}: {}", path.display(), e));
                    continue;
                }
            };

            let file_size = metadata.len();

            let modified_at = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let file_hash = match hasher::compute_blake3_hex(path) {
                Ok(h) => h,
                Err(e) => {
                    log_info(&format!("无法计算文件哈希 {}: {}", path.display(), e));
                    continue;
                }
            };

            files.push(FileEntry {
                file_path: path.to_string_lossy().to_string(),
                file_size,
                modified_at,
                file_hash,
            });
        }
    }

    let total = files.len();
    log_info(&format!("扫描完成，共找到 {} 个文件", total));

    Ok(ScanResult { files, total })
}