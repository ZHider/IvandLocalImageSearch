<script setup lang="ts">
import FolderManager from './FolderManager.vue'
import IndexProgressPanel from './IndexProgressPanel.vue'
import IndexResultPanel from './IndexResultPanel.vue'
import type { IndexProgress, IndexResult } from '../types'

defineProps<{
  folders: string[]
  indexing: boolean
  stopping: boolean
  addFolderDisabled: boolean
  progress: IndexProgress | null
  result: IndexResult | null
}>()

const emit = defineEmits<{
  addFolder: []
  removeFolder: [index: number]
  startIndex: []
  stopIndex: []
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
      v-if="!indexing"
      type="primary"
      size="large"
      @click="emit('startIndex')"
      :disabled="folders.length === 0"
      block
    >
      🚀 开始索引
    </n-button>

    <n-button
      v-else-if="stopping"
      type="warning"
      size="large"
      :loading="true"
      block
    >
      正在停止...
    </n-button>

    <n-button
      v-else
      type="warning"
      size="large"
      @click="emit('stopIndex')"
      block
    >
      ⏹ 停止索引
    </n-button>

    <IndexProgressPanel :progress="progress" />
    <IndexResultPanel :result="result" />
  </n-space>
</template>