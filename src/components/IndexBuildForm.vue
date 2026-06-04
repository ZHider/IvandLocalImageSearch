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

    <div class="btn-wrapper">
      <!-- 停止按钮（底层） -->
      <n-button
        v-if="stopping"
        type="warning"
        size="large"
        :loading="true"
        block
        class="btn-stop"
      >
        正在停止...
      </n-button>
      <n-button
        v-else
        type="warning"
        size="large"
        @click="emit('stopIndex')"
        block
        class="btn-stop"
      >
        ⏹ 停止索引
      </n-button>

      <!-- 开始按钮（顶层，opacity 过渡） -->
      <n-button
        type="primary"
        size="large"
        @click="emit('startIndex')"
        :disabled="folders.length === 0"
        block
        class="btn-start"
        :class="{ 'btn-start--hidden': indexing }"
        :style="{ opacity: indexing ? 0 : 1 }"
      >
        🚀 开始索引
      </n-button>
    </div>

    <IndexProgressPanel :progress="progress" />
    <IndexResultPanel :result="result" />
  </n-space>
</template>

<style scoped>
.btn-wrapper {
  display: grid;
}

.btn-start,
.btn-stop {
  grid-area: 1 / 1;
  width: 100%;
}

.btn-start {
  z-index: 2;
  transition: opacity 0.15s ease;
}

.btn-start--hidden {
  pointer-events: none;
}
</style>