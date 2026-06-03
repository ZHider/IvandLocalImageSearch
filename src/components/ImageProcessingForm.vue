<script setup lang="ts">
import type { AppConfig } from '../types'

defineProps<{
  config: AppConfig
  saving: boolean
  loading: boolean
}>()

const emit = defineEmits<{
  'save': []
}>()
</script>

<template>
  <n-card :bordered="false" content-style="padding-bottom: 0">
    <n-form
      :model="config"
      label-placement="left"
      label-width="140"
      require-mark-placement="right-hanging"
    >
      <n-divider>嵌入图片</n-divider>
      <n-form-item label="最长边尺寸 (px)">
        <n-input-number
          v-model:value="config.imageProcessing.embedImageSize"
          :min="64"
          :max="4096"
          :step="64"
          style="width: 200px"
        />
        <n-text depth="3" style="margin-left: 12px; font-size: 13px;">
          HEIC 转 WebP 时缩放的最长边像素，默认 1280
        </n-text>
      </n-form-item>

      <n-divider>缩略图</n-divider>
      <n-form-item label="最长边尺寸 (px)">
        <n-input-number
          v-model:value="config.imageProcessing.thumbnailSize"
          :min="64"
          :max="2048"
          :step="32"
          style="width: 200px"
        />
        <n-text depth="3" style="margin-left: 12px; font-size: 13px;">
          缩略图最长边像素，默认 300
        </n-text>
      </n-form-item>

      <n-divider>索引性能</n-divider>
      <n-form-item label="并发线程数">
        <n-input-number
          v-model:value="config.embedThreads"
          :min="1"
          :max="16"
          :step="1"
          style="width: 200px"
        />
        <n-text depth="3" style="margin-left: 12px; font-size: 13px;">
          同时请求 embedding API 的并发数，默认 1，建议 2-4
        </n-text>
      </n-form-item>

      <n-form-item>
        <n-button
          type="primary"
          @click="emit('save')"
          :loading="saving"
          :disabled="loading"
        >
          保存配置
        </n-button>
      </n-form-item>
    </n-form>
  </n-card>
</template>