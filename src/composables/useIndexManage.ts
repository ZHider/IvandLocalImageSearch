import { ref } from 'vue'
import { useMessage } from 'naive-ui'
import { useExtension } from './useExtension'
import type { IndexFileMeta, QueryIndexParams, QueryIndexResult } from '../types'

const managing = ref(false)
const files = ref<IndexFileMeta[]>([])
const selectedFilePaths = ref<Set<string>>(new Set())
const queryError = ref<string | null>(null)
const queryTotal = ref(0)
const page = ref(1)
const pageSize = ref(50)
const optimizing = ref(false)
const optimizeMessage = ref('')

let initialized = false

export function useIndexManage() {
  const message = useMessage()
  const { send, on } = useExtension()

  if (!initialized) {
    initialized = true

    on('clearIndexComplete', () => {
      managing.value = false
      files.value = []
      selectedFilePaths.value = new Set()
      queryTotal.value = 0
      page.value = 1
      message.success('索引已全部清空')
    })

    on('clearIndexError', (data: unknown) => {
      managing.value = false
      const d = data as { error: string }
      message.error(`清空索引失败: ${d.error}`)
    })

    on('queryIndexResult', (data: unknown) => {
      const r = data as QueryIndexResult
      files.value = r.files || []
      queryTotal.value = r.total || 0
      selectedFilePaths.value = new Set()
      managing.value = false
    })

    on('queryIndexError', (data: unknown) => {
      const d = data as { error: string }
      queryError.value = d.error
      files.value = []
      queryTotal.value = 0
      managing.value = false
      message.error(`查询索引失败: ${d.error}`)
    })

    on('deleteIndexResult', (data: unknown) => {
      const d = data as { success: boolean; deleted: number; warning?: string }
      managing.value = false
      if (d.warning) {
        message.warning(`已删除 ${d.deleted} 条，但有警告: ${d.warning}`)
      } else {
        message.success(`已删除 ${d.deleted} 条索引`)
      }
      // 重新查询刷新列表（保持当前页）
      send('queryIndex', {
        page: page.value,
        pageSize: pageSize.value,
      } as Record<string, unknown>)
    })

    on('deleteIndexError', (data: unknown) => {
      const d = data as { error: string }
      managing.value = false
      message.error(`删除索引失败: ${d.error}`)
    })

    on('optimizeIndexProgress', (data: unknown) => {
      const d = data as { message: string }
      optimizing.value = true
      optimizeMessage.value = d.message || '正在优化...'
    })

    on('optimizeIndexComplete', () => {
      optimizing.value = false
      optimizeMessage.value = ''
      message.success('索引优化完成')
    })

    on('optimizeIndexError', (data: unknown) => {
      const d = data as { error: string }
      optimizing.value = false
      optimizeMessage.value = ''
      message.error(`索引优化失败: ${d.error}`)
    })
  }

  function clearIndex() {
    managing.value = true
    page.value = 1
    send('clearIndex')
  }

  function queryIndex(params: QueryIndexParams = {}) {
    managing.value = true
    queryError.value = null
    if (params.page !== undefined) page.value = params.page
    if (params.pageSize !== undefined) pageSize.value = params.pageSize
    send('queryIndex', {
      ...params,
      page: page.value,
      pageSize: pageSize.value,
    } as Record<string, unknown>)
  }

  function setPage(p: number) {
    page.value = p
    queryIndex()
  }

  function setPageSize(ps: number) {
    pageSize.value = ps
    page.value = 1
    queryIndex()
  }

  function deleteSelected() {
    const paths = Array.from(selectedFilePaths.value)
    if (paths.length === 0) {
      message.warning('请先选择要删除的文件')
      return
    }
    managing.value = true
    send('deleteIndexEntries', { filePaths: paths })
  }

  function toggleFile(filePath: string, selected: boolean) {
    const set = new Set(selectedFilePaths.value)
    if (selected) {
      set.add(filePath)
    } else {
      set.delete(filePath)
    }
    selectedFilePaths.value = set
  }

  function toggleAll(selected: boolean) {
    if (selected) {
      selectedFilePaths.value = new Set(files.value.map(f => f.filePath))
    } else {
      selectedFilePaths.value = new Set()
    }
  }

  function isSelected(filePath: string): boolean {
    return selectedFilePaths.value.has(filePath)
  }

  function optimizeIndex() {
    optimizing.value = true
    optimizeMessage.value = '正在启动优化...'
    send('optimizeIndex')
  }

  return {
    managing,
    files,
    selectedFilePaths,
    queryTotal,
    queryError,
    page,
    pageSize,
    optimizing,
    optimizeMessage,
    clearIndex,
    queryIndex,
    setPage,
    setPageSize,
    deleteSelected,
    toggleFile,
    toggleAll,
    isSelected,
    optimizeIndex,
  }
}
