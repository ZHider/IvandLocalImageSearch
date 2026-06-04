import { ref } from 'vue'
import { useMessage } from 'naive-ui'
import { useExtension } from './useExtension'
import type { IndexProgress, IndexResult } from '../types'

const progress = ref<IndexProgress | null>(null)
const result = ref<IndexResult | null>(null)
const stopping = ref(false)

let initialized = false

export function useIndex() {
  const message = useMessage()
  const { send, on } = useExtension()

  if (!initialized) {
    initialized = true

    on('indexProgress', (data: unknown) => {
      progress.value = data as IndexProgress
    })

    on('indexComplete', (data: unknown) => {
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
    })

    on('indexError', (data: unknown) => {
      progress.value = null
      const err = data as { error?: string }
      message.error(`索引失败: ${err?.error || '未知错误'}`)
    })

    on('indexCancelled', () => {
      stopping.value = false
      progress.value = null
      message.info('索引已停止')
    })
  }

  function start(folders: string[], embedThreads: number = 1) {
    if (folders.length === 0) return
    result.value = null
    progress.value = null
    send('startIndex', { folders, embedThreads })
  }

  function stop() {
    stopping.value = true
    send('cancelIndex', {})
  }

  return {
    stopping,
    progress,
    result,
    start,
    stop,
  }
}
