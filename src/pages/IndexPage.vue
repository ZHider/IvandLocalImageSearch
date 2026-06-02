<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { os } from '@neutralinojs/lib'
import { useMessage } from 'naive-ui'
import { useExtension } from '../composables/useExtension'
import { useAppConfig } from '../composables/useAppConfig'
import FolderManager from '../components/FolderManager.vue'
import IndexProgressPanel from '../components/IndexProgressPanel.vue'
import IndexResultPanel from '../components/IndexResultPanel.vue'

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

interface IndexComplete {
  files: Array<{
    file_path: string
    file_size: number
    modified_at: number
    file_hash: string
  }>
  total: number
  newCount: number
  modifiedCount: number
  deletedCount: number
  errorCount?: number
}

const message = useMessage()
const { send, on } = useExtension()
const { config, saveConfig: doSave } = useAppConfig()

const indexing = ref(false)
const addFolderDisabled = ref(false)

const progress = ref<IndexProgress | null>(null)
const result = ref<IndexComplete | null>(null)

onMounted(() => {
  const cleanups = [
    on('indexProgress', (data: unknown) => {
      progress.value = data as IndexProgress
    }),

    on('indexComplete', (data: unknown) => {
      indexing.value = false
      progress.value = null
      result.value = data as IndexComplete
      const r = data as IndexComplete
      const msgParts = [`新增 ${r.newCount}`, `修改 ${r.modifiedCount}`, `删除 ${r.deletedCount}`]
      if (r.errorCount && r.errorCount > 0) {
        msgParts.push(`失败 ${r.errorCount}`)
        message.warning(`索引完成 ⚠️ ${msgParts.join(' ')}`)
      } else {
        message.success(`索引完成 ✅ ${msgParts.join(' ')}`)
      }
    }),

    on('indexError', (data: unknown) => {
      indexing.value = false
      progress.value = null
      const err = data as { error?: string }
      message.error(`索引失败: ${err?.error || '未知错误'}`)
    }),
  ]

  onUnmounted(() => {
    cleanups.forEach(stop => stop())
  })
})

onUnmounted(() => {
  indexing.value = false
  progress.value = null
  result.value = null
})

async function addFolder() {
  try {
    addFolderDisabled.value = true
    const selected = await os.showOpenDialog('选择文件夹内的图片（将自动添加该文件夹到索引）', {
      multiSelections: true,
      filters: [{ name: '所有文件', extensions: ['*'] }],
    })
    if (selected && selected.length > 0) {
      const dirs = new Set(
        selected.map((p: string) => {
          const i = Math.max(p.lastIndexOf('\\'), p.lastIndexOf('/'))
          return i >= 0 ? p.substring(0, i) : p
        })
      )
      for (const dir of dirs) {
        if (!config.value.folders.includes(dir)) {
          config.value.folders.push(dir)
        }
      }
      doSave()
    }
  } catch (e) {
    message.error(`选择文件失败: ${e}`)
  } finally {
    addFolderDisabled.value = false
  }
}

function removeFolder(index: number) {
  config.value.folders.splice(index, 1)
  doSave()
}

function startIndex() {
  if (config.value.folders.length === 0) {
    message.warning('请先添加要索引的文件夹')
    return
  }

  indexing.value = true
  result.value = null
  progress.value = null

  send('startIndex', {
    folders: config.value.folders,
  })
}
</script>

<template>
  <div class="index-container">
    <n-card title="📂 索引管理" class="index-card">
      <n-space vertical :size="16">
        <FolderManager
          :folders="config.folders"
          :add-folder-disabled="addFolderDisabled"
          :indexing="indexing"
          @addFolder="addFolder"
          @removeFolder="removeFolder"
        />

        <n-button
          type="primary"
          size="large"
          @click="startIndex"
          :loading="indexing"
          :disabled="config.folders.length === 0"
          block
        >
          {{ indexing ? '索引进行中...' : '🚀 开始索引' }}
        </n-button>

        <IndexProgressPanel :progress="progress" />

        <IndexResultPanel :result="result" />
      </n-space>
    </n-card>
  </div>
</template>

<style scoped>
.index-container {
  padding: 20px;
}

.index-card {
  max-width: 800px;
  margin: 0 auto;
}
</style>
