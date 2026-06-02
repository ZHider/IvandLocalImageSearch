<script setup lang="ts">
import type { SearchResult } from '../types'

defineProps<{
  item: SearchResult
  thumbnailSrc: string
  itemHeight: number
}>()

const emit = defineEmits<{
  preview: []
}>()

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
  return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB'
}

function formatSimilarity(sim: number): string {
  return sim.toFixed(1) + '%'
}

function getSimilarityColor(sim: number): string {
  if (sim >= 0.8) return '#18a058'
  if (sim >= 0.6) return '#f0a020'
  return '#666'
}
</script>

<template>
  <div class="result-card" :style="{ height: itemHeight + 'px' }">
    <div class="card-thumb" @click="emit('preview')">
      <img
        v-if="thumbnailSrc"
        :src="thumbnailSrc"
        :data-file-path="item.file_path"
        class="thumb-img"
        loading="lazy"
      />
      <div v-else class="thumb-placeholder" :data-file-path="item.file_path">
        <template v-if="item.file_type === 'text'">
          <span class="placeholder-icon">📄</span>
        </template>
        <template v-else>
          <span class="placeholder-icon">🖼️</span>
        </template>
      </div>
      <div class="similarity-badge" :style="{ background: getSimilarityColor(item.similarity) }">
        {{ formatSimilarity(item.similarity) }}
      </div>
    </div>
    <div class="card-info">
      <n-ellipsis class="card-name" :tooltip="false">
        {{ item.file_name }}
      </n-ellipsis>
      <div class="card-meta">
        <n-tag :type="item.file_type === 'image' ? 'success' : 'info'" size="small" :bordered="false">
          {{ item.file_type === 'image' ? '📷' : '📝' }}
        </n-tag>
        <span class="card-size">{{ formatSize(item.file_size) }}</span>
      </div>
      <n-ellipsis v-if="item.text_preview" class="card-preview" :line-clamp="2" :tooltip="false">
        {{ item.text_preview }}
      </n-ellipsis>
    </div>
  </div>
</template>

<style scoped>
.result-card {
  background: #fff;
  border-radius: 8px;
  overflow: hidden;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.08);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.result-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.13);
}

.card-thumb {
  position: relative;
  width: 100%;
  height: 160px;
  overflow: hidden;
  background: #f0f0f0;
  flex-shrink: 0;
}

.thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.thumb-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #e8e8e8, #f5f5f5);
}

.placeholder-icon {
  font-size: 48px;
  opacity: 0.5;
}

.similarity-badge {
  position: absolute;
  top: 8px;
  right: 8px;
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 10px;
}

.card-info {
  padding: 8px 10px;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow: hidden;
}

.card-name {
  font-size: 13px;
  font-weight: 500;
  line-height: 1.3;
}

.card-meta {
  display: flex;
  align-items: center;
  gap: 6px;
}

.card-size {
  font-size: 12px;
  color: #999;
}

.card-preview {
  font-size: 12px;
  color: #888;
  line-height: 1.4;
  margin-top: 2px;
}
</style>
