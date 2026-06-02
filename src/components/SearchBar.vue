<script setup lang="ts">
import { os } from '@neutralinojs/lib'
import { useMessage } from 'naive-ui'
const message = useMessage()

defineProps<{
  searchText: string
  searchImagePath: string
  searching: boolean
}>()

const emit = defineEmits<{
  'update:searchText': [value: string]
  'update:searchImagePath': [value: string]
  'search': []
}>()

async function selectImage() {
  try {
    const selected = await os.showOpenDialog('选择图片', {
      filters: [{ name: '图片', extensions: ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp'] }],
    })
    if (selected && selected.length > 0) {
      emit('update:searchImagePath', selected[0])
      emit('update:searchText', '')
    }
  } catch (e) {
    message.error(`选择图片失败: ${e}`)
  }
}

function clearImage() {
  emit('update:searchImagePath', '')
}

function doSearchOnEnter(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    emit('search')
  }
}

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

  const filePath = (file as any).path || file.name
  if (filePath) {
    emit('update:searchImagePath', filePath)
    emit('update:searchText', '')
  }
}
</script>

<template>
  <n-card class="search-bar-card" @dragover="onDragOver" @drop="onDrop">
    <n-space vertical :size="16">
      <n-space :size="12" align="center">
        <n-input
          :value="searchText"
          @update:value="(v: string) => emit('update:searchText', v)"
          placeholder="输入搜索关键词..."
          :disabled="searching || !!searchImagePath"
          clearable
          round
          size="large"
          style="flex: 1"
          @keydown="doSearchOnEnter"
        />

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
          @click="emit('search')"
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
</template>

<style scoped>
.search-bar-card {
  flex-shrink: 0;
}
</style>
