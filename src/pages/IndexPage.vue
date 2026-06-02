<script setup lang="ts">
import { ref, h } from 'vue'
import type { MenuOption } from 'naive-ui'
import { useMessage } from 'naive-ui'
import { os } from '@neutralinojs/lib'
import { useAppConfig } from '../composables/useAppConfig'
import { useIndex } from '../composables/useIndex'
import IndexBuildForm from '../components/IndexBuildForm.vue'
import IndexManagePanel from '../components/IndexManagePanel.vue'

const message = useMessage()
const { config, saveConfig: doSave } = useAppConfig()
const { indexing, progress, result, start: startIndex } = useIndex()

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
  startIndex(config.value.folders)
}
</script>

<template>
  <div class="index-container">
    <n-layout class="index-layout" has-sider>
      <n-layout-sider
        bordered
        content-style="padding: 0;"
        width="200"
        :native-scrollbar="false"
      >
        <n-menu
          v-model:value="activeTab"
          :options="menuOptions"
          :collapsed="false"
        />
      </n-layout-sider>
      <n-layout-content class="index-content" :native-scrollbar="false">
        <div class="index-body">
          <Transition name="slide" mode="out-in">
            <div :key="activeTab" class="index-section">
              <IndexBuildForm
                v-if="activeTab === 'build'"
                :folders="config.folders"
                :indexing="indexing"
                :add-folder-disabled="addFolderDisabled"
                :progress="progress"
                :result="result"
                @addFolder="addFolder"
                @removeFolder="removeFolder"
                @startIndex="handleStartIndex"
              />
              <IndexManagePanel v-else />
            </div>
          </Transition>
        </div>
      </n-layout-content>
    </n-layout>
  </div>
</template>

<style scoped>

.slide-enter-active,
.slide-leave-active {
  transition: all 0.1s ease;
}

.slide-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.slide-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

.index-container {
  height: 100%;
}

.index-layout {
  height: 100%;
}

.index-content {
  padding: 0;
  display: flex;
  flex-direction: column;
}
.index-body {
  flex: 1;
  padding: 20px 24px;
  overflow-y: auto;
}

.index-section {
  max-width: 800px;
}
</style>
