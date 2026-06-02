<script setup lang="ts">
import { ref, h, onMounted } from 'vue'
import type { DataTableColumn } from 'naive-ui'
import { NButton } from 'naive-ui'
import { useIndexManage } from '../composables/useIndexManage'
import { useThumbnailManager } from '../composables/useThumbnailManager'

const manage = useIndexManage()
const thumb = useThumbnailManager()

const pathFilter = ref('')
const timeAfter = ref<number | null>(null)
const timeBefore = ref<number | null>(null)

function formatTimestamp(ts: number | null): string | undefined {
  if (ts === null) return undefined
  return new Date(ts).toISOString().replace(/\.\d{3}Z$/, 'Z')
}

onMounted(() => {
  manage.queryIndex()
})

function handleSearchFilter() {
  manage.queryIndex({
    page: 1,
    pathFilter: pathFilter.value || undefined,
    timeAfter: formatTimestamp(timeAfter.value),
    timeBefore: formatTimestamp(timeBefore.value),
  })
}

function handleDeleteOne(filePath: string) {
  manage.selectedFilePaths.value = new Set([filePath])
  manage.deleteSelected()
}

function handleClear() {
  window.confirm('确定要清空全部索引吗？此操作不可撤销。') && manage.clearIndex()
}

const columns: DataTableColumn[] = [
  { type: 'selection', width: 40 },
  { title: '路径', key: 'filePath', ellipsis: { tooltip: true } },
  { title: '索引时间', key: 'indexedAt', width: 180 },
  {
    title: '操作',
    key: 'filePath',
    width: 80,
    render(row: Record<string, unknown>) {
      return h(
        NButton,
        { size: 'tiny', text: true, type: 'error', onClick: () => handleDeleteOne(row.filePath as string) },
        { default: () => '删除' }
      )
    },
  },
]
</script>

<template>
  <n-space vertical :size="16">
    <!-- 筛选栏 -->
    <n-space :size="12" wrap>
      <n-input
        v-model:value="pathFilter"
        placeholder="路径筛选（输入关键字）"
        clearable
        style="min-width: 200px"
        @keyup.enter="handleSearchFilter"
      />
      <n-date-picker
        v-model:value="timeAfter"
        type="datetime"
        placeholder="起始时间"
        clearable
        style="min-width: 160px"
      />
      <n-date-picker
        v-model:value="timeBefore"
        type="datetime"
        placeholder="截止时间"
        clearable
        style="min-width: 160px"
      />
      <n-button type="primary" @click="handleSearchFilter" :loading="manage.managing.value">
        查询
      </n-button>
    </n-space>

    <!-- 索引操作 -->
    <n-space :size="12">
      <n-button type="warning" @click="handleClear" :loading="manage.managing.value">
        🗑️ 清空全部索引
      </n-button>
      <n-button
        type="error"
        @click="manage.deleteSelected()"
        :disabled="manage.selectedFilePaths.value.size === 0"
        :loading="manage.managing.value"
      >
        删除选中
      </n-button>
      <n-tag v-if="manage.queryTotal.value > 0">
        共 {{ manage.queryTotal.value }} 条
      </n-tag>
    </n-space>

    <!-- 数据表 -->
    <n-data-table
      v-if="manage.files.value.length > 0 || manage.managing.value"
      :columns="columns"
      :data="manage.files.value"
      :row-key="(row: any) => row.filePath"
      :checked-row-keys="Array.from(manage.selectedFilePaths.value)"
      :bordered="true"
      :single-line="true"
      :loading="manage.managing.value"
      :pagination="{
        page: manage.page.value,
        pageSize: manage.pageSize.value,
        itemCount: manage.queryTotal.value,
        showQuickJumper: true,
        showSizePicker: true,
        pageSizes: [20, 50, 100, 200],
      }"
      size="small"
      @update:page="manage.setPage($event)"
      @update:page-size="manage.setPageSize($event)"
      @update:checked-row-keys="(keys: (string | number)[]) => manage.selectedFilePaths.value = new Set(keys.map(String))"
    />

    <n-empty v-else-if="!manage.managing.value" description="无索引记录" />

    <!-- 缩略图缓存管理 -->
    <n-divider />

    <n-space vertical :size="12">
      <n-text depth="2" style="font-size: 15px; font-weight: 500;">🖼️ 缩略图缓存管理</n-text>

      <n-space :size="12">
        <n-button @click="thumb.clearAll()" :loading="thumb.clearingAll.value">
          🗑️ 清空全部缩略图
        </n-button>
        <n-button @click="thumb.clearExpired()" :loading="thumb.clearingExpired.value">
          🔍 清除过期缩略图
        </n-button>
      </n-space>

      <n-progress
        v-if="thumb.progress.value"
        type="line"
        :percentage="Math.round(thumb.progress.value.current / thumb.progress.value.total * 100)"
        :indicator-placement="'inside'"
        :height="24"
        :border-radius="4"
      >
        {{ thumb.progress.value.deleted }} 个已删除
      </n-progress>

      <n-text v-if="thumb.progress.value" depth="3" class="current-file">
        进度: {{ thumb.progress.value.current }} / {{ thumb.progress.value.total }}
        <template v-if="thumb.progress.value.currentFile">
          — {{ thumb.progress.value.currentFile }}
        </template>
      </n-text>
    </n-space>
  </n-space>
</template>
