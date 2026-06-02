<script setup lang="ts">
interface IndexProgress {
  phase: string
  current: number
  total: number
  percentage: number
  currentFile?: string
  deletedFile?: string
  newCount: number
  modifiedCount: number
  deletedCount: number
  errorCount?: number
}

defineProps<{
  progress: IndexProgress | null
}>()

const phaseLabels: Record<string, string> = {
  scanning: '扫描文件',
  compare: '对比差异',
  processing: '处理文件',
  indexing: '更新索引',
  cleanup: '清理旧数据',
}
</script>

<template>
  <n-card v-if="progress" size="small" title="索引进度">
    <n-space vertical :size="12">
      <n-tag type="info">
        {{ phaseLabels[progress.phase] || progress.phase }}
      </n-tag>
      <n-progress
        type="line"
        :percentage="progress.percentage"
        :indicator-placement="'inside'"
        :height="24"
        :border-radius="4"
        :fill-border-radius="0"
      />
      <n-text depth="3">
        进度: {{ progress.current }} / {{ progress.total }}
      </n-text>
      <n-text v-if="progress.currentFile" depth="3" class="current-file">
        📄 {{ progress.currentFile }}
      </n-text>
      <n-text v-if="progress.deletedFile" depth="3" class="current-file">
        🗑️ {{ progress.deletedFile }}
      </n-text>
      <n-text v-if="progress.errorCount" depth="3" type="warning">
        ⚠️ 失败: {{ progress.errorCount }} 个
      </n-text>
    </n-space>
  </n-card>
</template>

<style scoped>
.current-file {
  word-break: break-all;
  max-width: 100%;
}
</style>
