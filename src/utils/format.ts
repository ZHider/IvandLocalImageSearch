/**
 * 格式化文件大小为可读字符串
 */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
  return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB'
}

/**
 * 根据相似度值返回对应的颜色
 */
export function getSimilarityColor(sim: number): string {
  if (sim >= 0.8) return '#18a058'
  if (sim >= 0.6) return '#f0a020'
  return '#666'
}