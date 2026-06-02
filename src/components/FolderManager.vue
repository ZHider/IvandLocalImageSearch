<script setup lang="ts">
defineProps<{
  folders: string[]
  addFolderDisabled: boolean
  indexing: boolean
}>()

const emit = defineEmits<{
  addFolder: []
  removeFolder: [index: number]
}>()
</script>

<template>
  <n-card size="small" title="已配置的文件夹">
    <n-space vertical :size="8">
      <n-button
        @click="emit('addFolder')"
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
              @click="emit('removeFolder', index)"
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
</template>

<style scoped>
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
</style>
