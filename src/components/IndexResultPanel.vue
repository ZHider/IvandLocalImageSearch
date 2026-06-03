<script setup lang="ts">
import type { IndexResult } from '../types'
import { formatSize } from '../utils/format'

defineProps<{
  result: IndexResult | null
}>()
</script>

<template>
  <n-card v-if="result" size="small" title="索引结果">
    <n-space :size="16">
      <n-statistic label="扫描文件总数" :value="result.total" />
      <n-statistic label="新增" :value="result.newCount" />
      <n-statistic label="修改" :value="result.modifiedCount" />
      <n-statistic label="删除" :value="result.deletedCount" />
      <n-statistic v-if="result.errorCount" label="失败" :value="result.errorCount" />
    </n-space>

    <n-divider />

    <n-data-table
      v-if="result.files.length > 0"
      :columns="[
        { title: '文件路径', key: 'file_path', ellipsis: { tooltip: true } },
        { title: '大小', key: 'file_size', width: 100 },
      ]"
      :data="result.files.map(f => ({ file_path: f.file_path, file_size: formatSize(f.file_size) }))"
      :max-height="300"
      :bordered="false"
      size="small"
    />
  </n-card>
</template>
