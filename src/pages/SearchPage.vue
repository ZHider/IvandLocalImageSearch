<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { os } from '@neutralinojs/lib'
import { useMessage } from 'naive-ui'
import { useExtension } from '../composables/useExtension'

interface SearchResult {
  file_path: string
  file_name: string
  file_type: string
  file_size: number
  similarity: number
  thumbnail_path: string
  text_preview?: string
  exif?: {
    camera_make?: string
    camera_model?: string
    iso?: string
    aperture?: string
    shutter_speed?: string
    focal_length?: string
    date_taken?: string
  }
}

const message = useMessage()
const { send, on } = useExtension()

const searchText = ref('')
const searchImagePath = ref('')
const searching = ref(false)
const results = ref<SearchResult[]>([])
const previewImage = ref<string | null>(null)

const showPreview = computed({
  get: () => previewImage.value !== null,
  set: (val: boolean) => {
    if (!val) previewImage.value = null
  },
})

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

function getThumbSrc(filePath: string, thumbnailPath: string): string {
  if (thumbnailCache.value[filePath]) {
    return thumbnailCache.value[filePath]
  }
  if (thumbnailPath) {
    thumbnailCache.value[filePath] = `file://${thumbnailPath.replace(/\\/g, '/')}`
    return thumbnailCache.value[filePath]
  }
  return ''
}

onMounted(() => {
  if (containerRef.value) {
    containerHeight.value = containerRef.value.clientHeight
  }

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
  })

  on('searchError', (data: unknown) => {
    searching.value = false
    const err = data as { error?: string }
    message.error(`搜索失败: ${err?.error || '未知错误'}`)
  })

  on('thumbnailReady', (data: unknown) => {
    const d = data as { imagePath: string; thumbnailPath: string }
    if (d.thumbnailPath) {
      thumbnailCache.value[d.imagePath] = `file://${d.thumbnailPath.replace(/\\/g, '/')}`
    }
  })

  on('thumbnailError', (data: unknown) => {
    const d = data as { imagePath: string; error: string }
    console.warn(`缩略图生成失败: ${d.imagePath}`, d.error)
  })
})

async function selectImage() {
  try {
    const selected = await os.showOpenDialog('选择图片', {
      filters: [{ name: '图片', extensions: ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp'] }],
    })
    if (selected && selected.length > 0) {
      searchImagePath.value = selected[0]
      searchText.value = ''
    }
  } catch {
    message.error('选择图片失败')
  }
}

function clearImage() {
  searchImagePath.value = ''
}

// 拖拽支持
function onDragOver(e: DragEvent) {
  e.preventDefault()
  if (e.dataTransfer) {
    e.dataTransfer.dropEffect = 'copy'
  }
}

function onDrop(e: DragEvent) {
  e.preventDefault()
  const files = e.dataTransfer?.files
  if (!files || files.length === 0) return

  const file = files[0]
  const ext = file.name.split('.').pop()?.toLowerCase() || ''
  const validExts = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp']
  if (!validExts.includes(ext)) {
    message.warning('请拖入图片文件 (jpg/png/gif/bmp/webp)')
    return
  }

  // In Neutralino, we need the full path. The browser `file.name` only gives
  // the basename + the browser stores a temporary path.  For real desktop
  // drag we use the webview drag event.  Since Neutralino wraps a webview,
  // `file.path` (Electron/Chromium extension) or the full path from the OS
  // drag is available via `file.path` in Chromium.
  // Fallback: use the OS dialog if the path isn't available.
  const filePath = (file as any).path || file.name
  if (filePath) {
    searchImagePath.value = filePath
    searchText.value = ''
  }
}

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

function doSearchOnEnter(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    doSearch()
  }
}

function openPreview(imagePath: string) {
  previewImage.value = imagePath
}

function closePreview() {
  previewImage.value = null
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
  return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB'
}

function formatSimilarity(sim: number): string {
  return sim.toFixed(1) + '%'
}

function getSimilarityColor(sim: number): string {
  if (sim >= 0.8) return '#18a058'
  if (sim >= 0.6) return '#f0a020'
  return '#666'
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
  <div class="search-container" @dragover="onDragOver" @drop="onDrop">
    <n-card class="search-card">
      <n-space vertical :size="16">
        <n-space :size="12" align="center">
          <n-input
            v-model:value="searchText"
            placeholder="输入搜索关键词..."
            :disabled="searching || !!searchImagePath"
            clearable
            round
            size="large"
            style="flex: 1"
            @keydown="doSearchOnEnter"
          >
          </n-input>

          <n-button
            @click="selectImage"
            :disabled="searching"
            size="large"
            secondary
          >
            {{ searchImagePath ? '🖼️ 已选图片' : '🖼️ 以图搜图' }}
          </n-button>

          <n-button
            type="primary"
            size="large"
            @click="doSearch"
            :loading="searching"
          >
            搜索
          </n-button>
        </n-space>

        <n-space v-if="searchImagePath" align="center">
          <n-tag type="info" closable @close="clearImage">
            📷 {{ searchImagePath.split(/[\\/]/).pop() }}
          </n-tag>
        </n-space>
      </n-space>
    </n-card>

    <div
      ref="containerRef"
      class="results-scroll"
      @scroll="onContainerScroll"
      :style="{ height: 'calc(100vh - 220px)' }"
    >
      <div v-if="results.length > 0" class="results-virtual" :style="{ height: totalHeight + 'px' }">
          <div class="results-grid" :style="{ paddingTop: paddingTop + 'px', gridTemplateColumns: `repeat(${COLUMNS}, 1fr)` }">
          <div
            v-for="item in visibleResults"
            :key="item._index"
            class="result-card"
            :style="{ height: ITEM_HEIGHT + 'px' }"
          >
            <div class="card-thumb" @click="openPreview(item.file_path)">
              <img
                v-if="getThumbSrc(item.file_path, item.thumbnail_path)"
                :src="getThumbSrc(item.file_path, item.thumbnail_path)"
                :data-file-path="item.file_path"
                class="thumb-img"
                loading="lazy"
              />
              <div v-else class="thumb-placeholder" :data-file-path="item.file_path">
                <template v-if="item.file_type === 'text'">
                  <span class="placeholder-icon">📄</span>
                </template>
                <template v-else>
                  <span class="placeholder-icon">🖼️</span>
                </template>
              </div>
              <div class="similarity-badge" :style="{ background: getSimilarityColor(item.similarity) }">
                {{ formatSimilarity(item.similarity) }}
              </div>
            </div>
            <div class="card-info">
              <n-ellipsis class="card-name" :tooltip="false">
                {{ item.file_name }}
              </n-ellipsis>
              <div class="card-meta">
                <n-tag :type="item.file_type === 'image' ? 'success' : 'info'" size="small" :bordered="false">
                  {{ item.file_type === 'image' ? '📷' : '📝' }}
                </n-tag>
                <span class="card-size">{{ formatSize(item.file_size) }}</span>
              </div>
              <n-ellipsis v-if="item.text_preview" class="card-preview" :line-clamp="2" :tooltip="false">
                {{ item.text_preview }}
              </n-ellipsis>
            </div>
          </div>
        </div>
      </div>

      <n-empty
        v-else-if="!searching"
        description="请输入搜索词或选择图片开始搜索"
        class="search-empty"
      />
    </div>

    <n-modal
      v-model:show="showPreview"
      preset="card"
      title="图片预览"
      style="max-width: 90vw"
      :on-close="closePreview"
      :mask-closable="true"
    >
      <img
        v-if="previewImage"
        :src="`file://${(previewImage || '').replace(/\\/g, '/')}`"
        class="preview-image"
      />
    </n-modal>
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

.search-card {
  flex-shrink: 0;
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

.result-card {
  background: #fff;
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.08);
  cursor: pointer;
  transition: box-shadow 0.2s;
  display: flex;
  flex-direction: column;
}

.result-card:hover {
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
}

.card-thumb {
  position: relative;
  width: 100%;
  height: 160px;
  overflow: hidden;
  background: #f0f0f0;
  flex-shrink: 0;
}

.thumb-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.thumb-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #e8e8e8, #f5f5f5);
}

.placeholder-icon {
  font-size: 48px;
  opacity: 0.5;
}

.similarity-badge {
  position: absolute;
  top: 8px;
  right: 8px;
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 10px;
}

.card-info {
  padding: 8px 10px;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow: hidden;
}

.card-name {
  font-size: 13px;
  font-weight: 500;
  line-height: 1.3;
}

.card-meta {
  display: flex;
  align-items: center;
  gap: 6px;
}

.card-size {
  font-size: 12px;
  color: #999;
}

.card-preview {
  font-size: 12px;
  color: #888;
  line-height: 1.4;
  margin-top: 2px;
}

.search-empty {
  margin-top: 80px;
}

.preview-image {
  width: 100%;
  max-height: 70vh;
  object-fit: contain;
  border-radius: 4px;
}
</style>