<script setup lang="ts">
import { ref, h } from 'vue'
import type { MenuOption } from 'naive-ui'
import { useAppConfig } from '../composables/useAppConfig'
import ApiSettingsForm from '../components/ApiSettingsForm.vue'
import ImageProcessingForm from '../components/ImageProcessingForm.vue'

const {
  config,
  saving,
  loading,
  needsApiKey,
  isCustomApi,
  onApiTypeChange,
  saveConfig: doSave,
} = useAppConfig()

const activeTab = ref<string>('api')

const menuOptions: MenuOption[] = [
  {
    label: 'API 设置',
    key: 'api',
    icon: () => h('span', '🔌'),
  },
  {
    label: '图片处理',
    key: 'image',
    icon: () => h('span', '🖼️'),
  },
]

</script>

<template>
  <div class="settings-container">
    <n-layout class="settings-layout" has-sider>
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
      <n-layout-content class="settings-content" :native-scrollbar="false">
        <div class="settings-body">
          <Transition name="slide" mode="out-in">
            <div :key="activeTab" class="settings-section">
              <ImageProcessingForm
                v-if="activeTab === 'image'"
                :config="config"
                :saving="saving"
                :loading="loading"
                @save="doSave"
              />
              <ApiSettingsForm
                v-else
                :config="config"
                :saving="saving"
                :loading="loading"
                :is-custom-api="isCustomApi"
                :needs-api-key="needsApiKey"
                @update:api-type="onApiTypeChange"
                @save="doSave"
              />
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

.settings-container {
  height: 100%;
}

.settings-layout {
  height: 100%;
}

.settings-content {
  padding: 0;
  display: flex;
  flex-direction: column;
}
.settings-body {
  flex: 1;
  padding: 20px 24px;
  overflow-y: auto;
}

.settings-section {
  max-width: 700px;
}
</style>
