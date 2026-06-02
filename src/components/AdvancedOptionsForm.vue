<script setup lang="ts">
import { ref, watch } from 'vue'
import type { AppConfig } from '../types'

const props = defineProps<{
  config: AppConfig
  saving: boolean
  loading: boolean
}>()

const emit = defineEmits<{
  'save': []
}>()

// JSON 合法性校验状态
const jsonError = ref<string | null>(null)

// 所有可选扩展名
const allExtensions = [
  { label: 'JPEG (.jpg)', value: 'jpg' },
  { label: 'JPEG (.jpeg)', value: 'jpeg' },
  { label: 'PNG', value: 'png' },
  { label: 'WebP', value: 'webp' },
  { label: 'BMP', value: 'bmp' },
  { label: 'TIFF (.tiff)', value: 'tiff' },
  { label: 'ICO', value: 'ico' },
  { label: 'GIF', value: 'gif' },
  { label: 'HEIC (.heic)', value: 'heic' },
  { label: 'HEIF (.heif)', value: 'heif' },
]

// 监听 extraEmbeddingParams 的变化，实时校验 JSON
watch(
  () => props.config.advancedOptions.extraEmbeddingParams,
  (val) => {
    if (!val || val.trim() === '') {
      jsonError.value = null
      return
    }
    try {
      const parsed = JSON.parse(val)
      if (typeof parsed !== 'object' || Array.isArray(parsed) || parsed === null) {
        jsonError.value = '必须是一个 JSON 对象 (Object)，如 {"key": "value"}'
      } else {
        jsonError.value = null
      }
    } catch {
      jsonError.value = 'JSON 格式不合法，请检查语法'
    }
  },
)

function validateOnSave() {
  // 保存前再次校验 JSON
  const val = props.config.advancedOptions.extraEmbeddingParams
  if (val && val.trim() !== '') {
    try {
      const parsed = JSON.parse(val)
      if (typeof parsed !== 'object' || Array.isArray(parsed) || parsed === null) {
        jsonError.value = '必须是一个 JSON 对象 (Object)'
        return
      }
    } catch {
      jsonError.value = 'JSON 格式不合法'
      return
    }
  }
  emit('save')
}
</script>

<template>
  <n-card :bordered="false" content-style="padding-bottom: 0">
    <n-form
      :model="config.advancedOptions"
      label-placement="left"
      label-width="160"
      require-mark-placement="right-hanging"
    >
      <n-divider>WebP 压缩策略</n-divider>

      <n-form-item label="单张图片阈值 (MB)">
        <n-input-number
          v-model:value="config.advancedOptions.webpThresholdMB"
          :min="0"
          :max="100"
          :step="0.5"
          :precision="1"
          style="width: 160px"
        />
        <n-text depth="3" style="margin-left: 12px; font-size: 13px;">
          超过此大小的图片将被缩放并转为 WebP 再发送，设为 0 表示全部转换
        </n-text>
      </n-form-item>

      <n-form-item label="原生支持格式">
        <n-select
          v-model:value="config.advancedOptions.nativeExtensions"
          :options="allExtensions"
          multiple
          filterable
          tag
          style="width: 400px"
          placeholder="选择或输入扩展名（不带点）"
        />
        <n-text depth="3" style="margin-left: 12px; font-size: 13px;">
          勾选的格式不会被转换，不勾选的格式将转为 WebP 再发送
        </n-text>
      </n-form-item>

      <n-divider>额外参数</n-divider>

      <n-form-item
        label="Embedding 参数"
        :feedback="jsonError ?? undefined"
        :validation-status="jsonError ? 'error' : undefined"
      >
        <n-input
          v-model:value="config.advancedOptions.extraEmbeddingParams"
          type="textarea"
          :rows="4"
          placeholder='可选，如：{"pooling_type": "cls", "normalize": true}'
          style="width: 400px; font-family: monospace;"
          :input-props="{ spellcheck: false }"
        />
        <n-text depth="3" style="margin-left: 12px; font-size: 13px;">
          JSON 对象，将被注入到 embedding 请求体的 parameters 字段中
        </n-text>
      </n-form-item>

      <n-form-item>
        <n-button
          type="primary"
          @click="validateOnSave"
          :loading="saving"
          :disabled="loading || !!jsonError"
        >
          保存配置
        </n-button>
      </n-form-item>
    </n-form>
  </n-card>
</template>
