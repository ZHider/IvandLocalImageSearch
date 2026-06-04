<script setup lang="ts">
import type { SearchResult } from '../types'
import { formatSize, getSimilarityColor } from '../utils/format'

defineProps<{
  show: boolean
  previewImage: string | null
  previewLoading: boolean
  previewItem: SearchResult | null
}>()

const emit = defineEmits<{
  close: []
}>()
</script>

<template>
  <n-modal
    :show="show"
    preset="card"
    title="图片预览"
    style="max-width: 90vw; max-height: 90vh; overflow-y: auto;"
    :on-close="() => emit('close')"
    :mask-closable="true"
    @update:show="(val: boolean) => { if (!val) emit('close') }"
  >
    <div v-if="previewImage" class="preview-image-wrap">
      <div class="preview-img-container">
        <img
          :src="previewImage"
          :class="previewLoading ? 'preview-image preview-loading' : 'preview-image'"
        />
        <div v-if="previewLoading" class="preview-loader">加载中…</div>
      </div>
    </div>
    <div v-if="previewItem" class="preview-meta">
      <div class="meta-row">
        <span class="meta-label">文件名</span>
        <span class="meta-value">{{ previewItem.file_name }}</span>
      </div>
      <div class="meta-row">
        <span class="meta-label">路径</span>
        <span class="meta-value meta-path">{{ previewItem.file_path }}</span>
      </div>
      <div class="meta-row">
        <span class="meta-label">文件大小</span>
        <span class="meta-value">{{ formatSize(previewItem.file_size) }}</span>
      </div>
      <div class="meta-row">
        <span class="meta-label">相似度</span>
        <span class="meta-value" :style="{ color: getSimilarityColor(previewItem.similarity) }">
          {{ previewItem.similarity.toFixed(1) + '%' }}
        </span>
      </div>
      <div v-if="previewItem.exif" class="meta-divider">EXIF 信息</div>
      <div v-if="previewItem.exif && previewItem.exif.date_taken" class="meta-row">
        <span class="meta-label">拍摄时间</span>
        <span class="meta-value">{{ previewItem.exif.date_taken }}</span>
      </div>
      <div v-if="(previewItem.exif && previewItem.exif.camera_make) || (previewItem.exif && previewItem.exif.camera_model)" class="meta-row">
        <span class="meta-label">相机</span>
        <span class="meta-value">{{ [previewItem.exif.camera_make, previewItem.exif.camera_model].filter(Boolean).join(' ') }}</span>
      </div>
      <div v-if="previewItem.exif && previewItem.exif.iso" class="meta-row">
        <span class="meta-label">ISO</span>
        <span class="meta-value">{{ previewItem.exif.iso }}</span>
      </div>
      <div v-if="previewItem.exif && previewItem.exif.aperture" class="meta-row">
        <span class="meta-label">光圈</span>
        <span class="meta-value">{{ previewItem.exif.aperture }}</span>
      </div>
      <div v-if="previewItem.exif && previewItem.exif.shutter_speed" class="meta-row">
        <span class="meta-label">快门速度</span>
        <span class="meta-value">{{ previewItem.exif.shutter_speed }}</span>
      </div>
      <div v-if="previewItem.exif && previewItem.exif.focal_length" class="meta-row">
        <span class="meta-label">焦距</span>
        <span class="meta-value">{{ previewItem.exif.focal_length }}</span>
      </div>
    </div>
  </n-modal>
</template>

<style scoped>
.preview-image {
  width: 100%;
  max-height: 70vh;
  object-fit: contain;
  border-radius: 4px;
  transition: opacity 0.3s;
}

.preview-loading {
  opacity: 0.6;
  filter: blur(2px);
}

.preview-image-wrap {
  text-align: center;
}

.preview-img-container {
  position: relative;
  display: inline-block;
  width: 100%;
}

.preview-loader {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.5);
  z-index: 1;
}

.preview-meta {
  border-top: 1px solid var(--n-border-color);
  padding-top: 12px;
  font-size: 13px;
}

.meta-row {
  display: flex;
  padding: 4px 0;
  gap: 12px;
  align-items: baseline;
}

.meta-label {
  min-width: 72px;
  color: var(--n-text-color-3);
  flex-shrink: 0;
}

.meta-value {
  color: var(--n-text-color);
  word-break: break-all;
}

.meta-path {
  font-size: 12px;
  color: var(--n-text-color-2);
}

.meta-divider {
  font-size: 12px;
  color: var(--n-text-color-3);
  margin: 8px 0 4px;
  font-weight: 500;
  border-bottom: 1px solid var(--n-border-color);
  padding-bottom: 4px;
}
</style>
