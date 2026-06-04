import { ref, onMounted, onUnmounted } from 'vue'
import { useMessage } from 'naive-ui'
import { useExtension } from './useExtension'
import type { IndexProgress, IndexResult } from '../types'

export function useIndex() {
  const message = useMessage()
  const { send, on } = useExtension()

  const indexing = ref(false)
  const stopping = ref(false)
  const progress = ref<IndexProgress | null>(null)
  const result = ref<IndexResult | null>(null)

  const cleanups: (() => void)[] = []

  onMounted(() => {
    cleanups.push(
      on('indexProgress', (data: unknown) => {
        progress.value = data as IndexProgress
      }),

      on('indexComplete', (data: unknown) => {
        indexing.value = false
        progress.value = null
        result.value = data as IndexResult
        const r = data as IndexResult
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

      on('indexCancelled', () => {
        indexing.value = false
        stopping.value = false
        progress.value = null
        message.info('索引已停止')
      }),
    )
  })

  onUnmounted(() => {
    cleanups.forEach(stop => stop())
    cleanups.length = 0
    indexing.value = false
    progress.value = null
    result.value = null
  })

  function start(folders: string[], embedThreads: number = 1) {
    if (folders.length === 0) return

    indexing.value = true
    result.value = null
    progress.value = null

    send('startIndex', { folders, embedThreads })
  }

  function stop() {
    console.log('[useIndex] 发送 cancelIndex 事件')
    stopping.value = true
    send('cancelIndex', {})
  }

  return {
    indexing,
    stopping,
    progress,
    result,
    start,
    stop,
  }
}
