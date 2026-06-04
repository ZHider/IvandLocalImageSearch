<script setup lang="ts">
import { ref, h, inject, type Ref } from 'vue'
import type { MenuOption } from 'naive-ui'
import { useMessage } from 'naive-ui'
import { os } from '@neutralinojs/lib'
import { useAppConfig } from '../composables/useAppConfig'
import { useIndex } from '../composables/useIndex'
import SiderLayout from '../components/SiderLayout.vue'
import IndexBuildForm from '../components/IndexBuildForm.vue'
import IndexManagePanel from '../components/IndexManagePanel.vue'

const message = useMessage()
const { config, saveConfig: doSave } = useAppConfig()
const { progress, result, start: startIndex, stop: stopIndex, stopping } = useIndex()

// 使用全局索引状态（来自 App.vue），确保导航栏访问时状态同步
const indexing = inject<Ref<boolean>>('indexing', ref(false))

const activeTab = ref<string>('build')

const menuOptions: MenuOption[] = [
  {
    label: '建立索引',
    key: 'build',
    icon: () => h('span', '📂'),
  },
  {
    label: '管理索引',
    key: 'manage',
    icon: () => h('span', '🗂️'),
  },
]
const addFolderDisabled = ref(false)

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
        if (!config.value.folders.includes(dir)) {
          config.value.folders.push(dir)
        }
      }
      doSave()
    }
  } catch (e) {
    message.error(`选择文件失败: ${e}`)
  } finally {
    addFolderDisabled.value = false
  }
}

function removeFolder(index: number) {
  config.value.folders.splice(index, 1)
  doSave()
}

function handleStartIndex() {
  if (config.value.folders.length === 0) {
    message.warning('请先添加要索引的文件夹')
    return
  }
  indexing.value = true
  startIndex(config.value.folders, config.value.embedThreads)
}
</script>

<template>
  <SiderLayout
    v-model:active-tab="activeTab"
    :menu-options="menuOptions"
    :menu-disabled="indexing"
    :content-max-width="800"
  >
    <IndexBuildForm
      v-if="activeTab === 'build'"
      :folders="config.folders"
      :indexing="indexing"
      :stopping="stopping"
      :add-folder-disabled="addFolderDisabled"
      :progress="progress"
      :result="result"
      @addFolder="addFolder"
      @removeFolder="removeFolder"
      @startIndex="handleStartIndex"
      @stopIndex="stopIndex"
    />
    <IndexManagePanel v-else />
  </SiderLayout>
</template>