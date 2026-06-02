import { ref, onMounted, onUnmounted } from 'vue'
import { useMessage } from 'naive-ui'
import { useExtension } from './useExtension'

export function useThumbnailManager() {
  const message = useMessage()
  const { send, on } = useExtension()

  const clearingAll = ref(false)
  const clearingExpired = ref(false)
  const progress = ref<{ current: number; total: number; deleted: number; currentFile: string } | null>(null)

  const cleanups: (() => void)[] = []

  onMounted(() => {
    cleanups.push(
      on('clearAllThumbnailsComplete', () => {
        clearingAll.value = false
        message.success('缩略图已全部清空')
      }),
      on('clearExpiredThumbnailsProgress', (data: unknown) => {
        progress.value = data as { current: number; total: number; deleted: number; currentFile: string }
      }),
      on('clearExpiredThumbnailsComplete', (data: unknown) => {
        clearingExpired.value = false
        progress.value = null
        const r = data as { deleted: number }
        message.success(`已清除 ${r.deleted} 个过期缩略图`)
      }),
      on('clearExpiredThumbnailsError', (data: unknown) => {
        clearingExpired.value = false
        progress.value = null
        const err = data as { error: string }
        message.error(`清除过期缩略图失败: ${err.error}`)
      }),
    )
  })

  onUnmounted(() => {
    cleanups.forEach(stop => stop())
    cleanups.length = 0
  })

  function clearAll() {
    if (!window.confirm('确定要清空全部缩略图吗？此操作不可撤销。')) return
    clearingAll.value = true
    send('clearAllThumbnails')
  }

  function clearExpired() {
    clearingExpired.value = true
    progress.value = null
    send('clearExpiredThumbnails')
  }

  return {
    clearingAll,
    clearingExpired,
    progress,
    clearAll,
    clearExpired,
  }
}
