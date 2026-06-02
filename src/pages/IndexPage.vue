<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { os } from '@neutralinojs/lib'
import { useMessage } from 'naive-ui'
import { useExtension } from '../composables/useExtension'
import { useConfig } from '../composables/useConfig'
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
const { config: sharedConfig } = useConfig()

const folders = ref<string[]>([])
const indexing = ref(false)
const addFolderDisabled = ref(false)

const progress = ref<IndexProgress | null>(null)
const result = ref<IndexComplete | null>(null)

const phaseLabels: Record<string, string> = {
  scanning: '扫描文件',
  compare: '对比差异',
  processing: '处理文件',
  indexing: '更新索引',
  cleanup: '清理旧数据',
}

// 从共享配置同步文件夹列表
watch(() => sharedConfig.value.folders, (val) => {
  folders.value = [...val]
}, { immediate: true })

onMounted(() => {
  on('indexProgress', (data: unknown) => {
    progress.value = data as IndexProgress
  })

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
  })

  on('indexError', (data: unknown) => {
    indexing.value = false
    progress.value = null
    const err = data as { error?: string }
    message.error(`索引失败: ${err?.error || '未知错误'}`)
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
        if (!folders.value.includes(dir)) {
          folders.value.push(dir)
        }
      }
      saveFolders()
    }
  } catch (e) {
    message.error(`选择文件失败: ${e}`)
  } finally {
    addFolderDisabled.value = false
  }
}

function removeFolder(index: number) {
  folders.value.splice(index, 1)
  saveFolders()
}

function saveFolders() {
  const config = { ...sharedConfig.value, folders: folders.value }
  send('saveConfig', config)
}

function startIndex() {
  if (folders.value.length === 0) {
    message.warning('请先添加要索引的文件夹')
    return
  }

  indexing.value = true
  result.value = null
  progress.value = null

  send('startIndex', {
    folders: folders.value,
  })
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
  return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB'
}
</script>

<template>
  <div class="index-container">
    <n-card title="📂 索引管理" class="index-card">
      <n-space vertical :size="16">
        <n-card size="small" title="已配置的文件夹">
          <n-space vertical :size="8">
            <n-button
              @click="addFolder"
              :disabled="addFolderDisabled || indexing"
              size="small"
              type="primary"
              ghost
            >
              + 添加文件夹
            </n-button>
            <n-empty v-if="folders.length === 0" description="暂无文件夹，请先添加" />
            <n-list v-else bordered>
              <n-list-item v-for="(folder, index) in folders" :key="index">
                <div class="folder-item">
                  <span class="folder-path">{{ folder }}</span>
                  <n-button
                    @click="removeFolder(index)"
                    size="tiny"
                    type="error"
                    ghost
                    :disabled="indexing"
                  >
                    删除
                  </n-button>
                </div>
              </n-list-item>
            </n-list>
          </n-space>
        </n-card>

        <n-button
          type="primary"
          size="large"
          @click="startIndex"
          :loading="indexing"
          :disabled="folders.length === 0"
          block
        >
          {{ indexing ? '索引进行中...' : '🚀 开始索引' }}
        </n-button>

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

.folder-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.folder-path {
  flex: 1;
  font-size: 14px;
  word-break: break-all;
}

.current-file {
  word-break: break-all;
  max-width: 100%;
}
</style>