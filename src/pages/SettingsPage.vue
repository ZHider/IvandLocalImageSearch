<script setup lang="ts">
import { ref, h } from 'vue'
import type { MenuOption } from 'naive-ui'
import { useAppConfig } from '../composables/useAppConfig'
import SiderLayout from '../components/SiderLayout.vue'
import ApiSettingsForm from '../components/ApiSettingsForm.vue'
import ImageProcessingForm from '../components/ImageProcessingForm.vue'
import AdvancedOptionsForm from '../components/AdvancedOptionsForm.vue'

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
  {
    label: '高级选项',
    key: 'advanced',
    icon: () => h('span', '⚙️'),
  },
]

</script>

<template>
  <SiderLayout
    v-model:active-tab="activeTab"
    :menu-options="menuOptions"
    :content-max-width="700"
  >
    <AdvancedOptionsForm
      v-if="activeTab === 'advanced'"
      :config="config"
      :saving="saving"
      :loading="loading"
      @save="doSave"
    />
    <ImageProcessingForm
      v-else-if="activeTab === 'image'"
      :config="config"
      :saving="saving"
      :loading="loading"
      @save="doSave"
    />
    <ApiSettingsForm
      v-else-if="activeTab === 'api'"
      :config="config"
      :saving="saving"
      :loading="loading"
      :is-custom-api="isCustomApi"
      :needs-api-key="needsApiKey"
      @update:api-type="onApiTypeChange"
      @save="doSave"
    />
  </SiderLayout>
</template>