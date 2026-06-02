<script setup lang="ts">
import { ref, computed } from 'vue'
import type { FormInst } from 'naive-ui'
import { useMessage } from 'naive-ui'
import { useSettings, API_TYPE_OPTIONS, FORM_RULES } from '../composables/useSettings'
import { useApiTest } from '../composables/useApiTest'

const message = useMessage()
const formRef = ref<FormInst | null>(null)

const {
  config,
  saving,
  loading,
  needsApiKey,
  isCustomApi,
  onApiTypeChange,
  saveConfig: doSave,
} = useSettings()

const { testing, testConnection: doTest, availableModels } = useApiTest(config)


const modelOptions = computed(() =>
  availableModels.value.map((m) => ({ label: m, value: m }))
)

async function saveConfig() {
  try {
    await formRef.value?.validate()
    doSave()
  } catch (errors) {
    const fields = (errors as Array<{ field?: string }>)
      ?.map((e) => e.field)
      .filter(Boolean)
      .join(', ') || '表单'
    message.error(`请完善必填项: ${fields}`)
  }
}

async function testConnection() {
  try {
    await formRef.value?.validate()
    doTest()
  } catch (errors) {
    const fields = (errors as Array<{ field?: string }>)
      ?.map((e) => e.field)
      .filter(Boolean)
      .join(', ') || '表单'
    message.error(`请完善必填项: ${fields}`)
  }
}
</script>

<template>
  <div class="settings-container">
    <n-card title="⚙️ 设置" class="settings-card" content-style="padding-bottom: 0">
      <n-form
        ref="formRef"
        :model="config"
        :rules="FORM_RULES"
        label-placement="left"
        label-width="120"
        require-mark-placement="right-hanging"
      >
        <n-form-item label="API 类型" path="apiType">
          <n-select
            v-model:value="config.apiType"
            :options="API_TYPE_OPTIONS"
            placeholder="选择 API 类型"
            @update:value="onApiTypeChange"
          />
        </n-form-item>

        <n-form-item v-if="isCustomApi" label="提供商名称" path="customProviderName">
          <n-input
            v-model:value="config.customProviderName"
            placeholder="例如: SiliconFlow, Together AI"
          />
        </n-form-item>

        <n-form-item label="Endpoint" path="endpoint">
          <n-input
            v-model:value="config.endpoint"
            placeholder="例如: http://localhost:11434"
          />
        </n-form-item>

        <n-form-item v-if="isCustomApi" label="Embedding 路径" path="customEmbeddingPath">
          <n-input
            v-model:value="config.customEmbeddingPath"
            placeholder="例如: /v1/embeddings"
          />
        </n-form-item>

        <n-form-item v-if="needsApiKey" label="API Key" path="apiKey">
          <n-input
            v-model:value="config.apiKey"
            type="password"
            placeholder="可选"
            show-password-on="click"
          />
        </n-form-item>

        <n-form-item label="模型名称" path="modelName">
          <n-select
            v-model:value="config.modelName"
            :options="modelOptions"
            :filterable="true"
            :tag="true"
            placeholder="选择或输入模型名称，点击测试连接获取可用模型列表"
          />
        </n-form-item>

        <n-form-item>
          <n-space>
            <n-button
              type="primary"
              @click="saveConfig"
              :loading="saving"
              :disabled="loading"
            >
              保存配置
            </n-button>
            <n-button
              @click="testConnection"
              :loading="testing"
              :disabled="testing"
            >
              测试连接
            </n-button>
          </n-space>
        </n-form-item>
      </n-form>
    </n-card>
  </div>
</template>

<style scoped>
.settings-container {
  padding: 20px 20px 0;
}

.settings-card {
  max-width: 700px;
  margin: 0 auto;
}

</style>