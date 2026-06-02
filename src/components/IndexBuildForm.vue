<script setup lang="ts">
import FolderManager from './FolderManager.vue'
import IndexProgressPanel from './IndexProgressPanel.vue'
import IndexResultPanel from './IndexResultPanel.vue'
import type { IndexProgress, IndexResult } from '../types'

defineProps<{
  folders: string[]
  indexing: boolean
  addFolderDisabled: boolean
  progress: IndexProgress | null
  result: IndexResult | null
}>()

const emit = defineEmits<{
  addFolder: []
  removeFolder: [index: number]
  startIndex: []
}>()
</script>

<template>
  <n-space vertical :size="16">
    <FolderManager
      :folders="folders"
      :add-folder-disabled="addFolderDisabled"
      :indexing="indexing"
      @addFolder="emit('addFolder')"
      @removeFolder="emit('removeFolder', $event)"
    />

    <n-button
      type="primary"
      size="large"
      @click="emit('startIndex')"
      :loading="indexing"
      :disabled="folders.length === 0"
      block
    >
      {{ indexing ? '索引进行中...' : '🚀 开始索引' }}
    </n-button>

    <IndexProgressPanel :progress="progress" />
    <IndexResultPanel :result="result" />
  </n-space>
</template>
