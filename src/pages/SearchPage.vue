<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useMessage } from 'naive-ui'
import { useExtension } from '../composables/useExtension'
import type { SearchResult } from '../types'
import SearchBar from '../components/SearchBar.vue'
import ResultCard from '../components/ResultCard.vue'
import PreviewModal from '../components/PreviewModal.vue'

const message = useMessage()
const { send, on } = useExtension()

const searchText = ref('')
const searchImagePath = ref('')
const searching = ref(false)
const results = ref<SearchResult[]>([])
const previewImage = ref<string | null>(null)
const previewLoading = ref(false)
const previewItem = ref<SearchResult | null>(null)

const containerRef = ref<HTMLElement | null>(null)
const scrollTop = ref(0)
const containerHeight = ref(600)

const ITEM_HEIGHT = 260
const COLUMNS = computed(() => {
  const width = containerRef.value?.clientWidth ?? 800
  if (width > 1200) return 4
  if (width > 900) return 3
  if (width > 600) return 2
  return 1
})
const GAP = 12

function onContainerScroll() {
  if (containerRef.value) {
    scrollTop.value = containerRef.value.scrollTop
    containerHeight.value = containerRef.value.clientHeight
  }
}

const rowCount = computed(() => Math.ceil(results.value.length / COLUMNS.value))
const totalHeight = computed(() => rowCount.value * (ITEM_HEIGHT + GAP) + GAP)
const startRow = computed(() => Math.max(0, Math.floor(scrollTop.value / (ITEM_HEIGHT + GAP)) - 2))
const endRow = computed(() =>
  Math.min(rowCount.value, Math.ceil((scrollTop.value + containerHeight.value) / (ITEM_HEIGHT + GAP)) + 2)
)

const visibleResults = computed(() => {
  const start = startRow.value * COLUMNS.value
  const end = Math.min(results.value.length, endRow.value * COLUMNS.value)
  return results.value.slice(start, end).map((item, i) => ({
    ...item,
    _index: start + i,
  }))
})

const paddingTop = computed(() => startRow.value * (ITEM_HEIGHT + GAP))

const thumbnailCache = ref<Record<string, string>>({})
const nlPort = (window as any).NL_PORT as string | undefined

function buildThumbUrl(filename: string): string {
  if (!filename) return ''
  const name = filename.split(/[\\/]/).pop() || filename
  if (nlPort) {
    return `http://localhost:${nlPort}/data/thumbnails/${name}`
  }
  // fallback（开发模式 NL_PORT 可能不可用）
  return `/data/thumbnails/${name}`
}

function getThumbSrc(filePath: string, thumbnailPath: string): string {
  if (thumbnailCache.value[filePath]) {
    return thumbnailCache.value[filePath]
  }
  if (thumbnailPath) {
    thumbnailCache.value[filePath] = buildThumbUrl(thumbnailPath)
    return thumbnailCache.value[filePath]
  }
  return ''
}

onMounted(() => {
  if (containerRef.value) {
    containerHeight.value = containerRef.value.clientHeight
  }

  const cleanups = [
    on('searchResult', (data: unknown) => {
      searching.value = false
      const r = data as { results: SearchResult[]; total: number }
      results.value = r.results || []
      thumbnailCache.value = {}
      if (r.total === 0) {
        message.info('未找到匹配结果')
      } else {
        message.success(`找到 ${r.total} 个结果`)
      }
      nextTick(() => {
        if (containerRef.value) {
          containerRef.value.scrollTop = 0
        }
      })
    }),

    on('searchError', (data: unknown) => {
      searching.value = false
      const err = data as { error?: string }
      message.error(`搜索失败: ${err?.error || '未知错误'}`)
    }),
    on('thumbnailReady', (data: unknown) => {
      const d = data as { imagePath: string; thumbnailPath: string }
      if (d.thumbnailPath) {
        thumbnailCache.value[d.imagePath] = buildThumbUrl(d.thumbnailPath)
      }
    }),

    on('thumbnailError', (data: unknown) => {
      const d = data as { imagePath: string; error: string }
      const filename = d.imagePath.split(/[\\/]/).pop() || d.imagePath
      message.warning(`缩略图加载失败: ${filename} — ${d.error}`)
    }),

    on('previewReady', (data: unknown) => {
      const d = data as { imagePath: string; previewData: string }
      if (d.previewData) {
        previewImage.value = d.previewData
        previewLoading.value = false
      }
    }),

    on('previewError', (data: unknown) => {
      const d = data as { imagePath: string; error: string }
      previewLoading.value = false
      message.error(`加载预览失败: ${d.error}`)
    }),
  ]

  onUnmounted(() => {
    cleanups.forEach(stop => stop())
  })
})

function doSearch() {
  if (!searchText.value.trim() && !searchImagePath.value) {
    message.warning('请输入搜索文字或选择图片')
    return
  }

  searching.value = true
  results.value = []

  if (searchImagePath.value) {
    send('search', {
      type: 'image',
      imagePath: searchImagePath.value,
    })
  } else {
    send('search', {
      type: 'text',
      text: searchText.value.trim(),
    })
  }
}

function openPreview(item: SearchResult) {
  previewItem.value = item
  previewLoading.value = true
  const thumb = getThumbSrc(item.file_path, item.thumbnail_path)
  if (thumb) {
    previewImage.value = thumb
  }
  send('getPreview', { imagePath: item.file_path })
}

function closePreview() {
  previewImage.value = null
  previewItem.value = null
  previewLoading.value = false
}

let thumbnailObs: IntersectionObserver | null = null

function setupThumbObserver() {
  if (thumbnailObs) {
    thumbnailObs.disconnect()
  }

  if (!containerRef.value) return

  thumbnailObs = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          const el = entry.target as HTMLElement
          const filePath = el.dataset.filePath
          if (filePath && !thumbnailCache.value[filePath]) {
            send('getThumbnail', { imagePath: filePath })
          }
        }
      }
    },
    { root: containerRef.value, rootMargin: '100px' }
  )

  const thumbs = containerRef.value.querySelectorAll('[data-file-path]')
  thumbs.forEach((el) => thumbnailObs?.observe(el))
}

watch(results, () => {
  nextTick(() => {
    setupThumbObserver()
  })
})

onUnmounted(() => {
  thumbnailObs?.disconnect()
  previewImage.value = null
})
</script>

<template>
  <div class="search-container">
    <SearchBar
      v-model:search-text="searchText"
      v-model:search-image-path="searchImagePath"
      :searching="searching"
      @search="doSearch"
    />

    <div
      ref="containerRef"
      class="results-scroll"
      @scroll="onContainerScroll"
      :style="{ height: 'calc(100vh - 220px)' }"
    >
      <div v-if="results.length > 0" class="results-virtual" :style="{ height: totalHeight + 'px' }">
        <div class="results-grid" :style="{ paddingTop: paddingTop + 'px', gridTemplateColumns: `repeat(${COLUMNS}, 1fr)` }">
          <ResultCard
            v-for="item in visibleResults"
            :key="item._index"
            :item="item"
            :thumbnail-src="getThumbSrc(item.file_path, item.thumbnail_path)"
            :item-height="ITEM_HEIGHT"
            @preview="openPreview(item)"
          />
        </div>
      </div>

      <n-empty
        v-else-if="!searching"

        description="请输入搜索词或选择图片开始搜索"
        class="search-empty"
      />
    </div>

    <PreviewModal
      :show="previewImage !== null"
      :preview-image="previewImage"
      :preview-loading="previewLoading"
      :preview-item="previewItem"
      @close="closePreview"
    />
  </div>
</template>

<style scoped>
.search-container {
  padding: 16px;
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.results-scroll {
  flex: 1;
  overflow-y: auto;
  margin-top: 12px;
  border-radius: 8px;
  background: var(--n-color, #fafafa);
}

.results-virtual {
  position: relative;
  width: 100%;
}

.results-grid {
  display: grid;
  gap: 12px;
  padding: 12px;
}

.search-empty {
  margin-top: 80px;
}
</style>
