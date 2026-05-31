import { ref, computed, onMounted } from 'vue'
import type { FormRules, MessageReactive } from 'naive-ui'
import { useMessage } from 'naive-ui'
import { useExtension } from './useExtension'
import { DEFAULT_CONFIG, type AppConfig, type ApiType } from '../types'

export interface ApiTypeOption {
  label: string
  value: string
}

export const API_TYPE_OPTIONS: ApiTypeOption[] = [
  { label: 'Ollama', value: 'ollama' },
  { label: 'llama.cpp', value: 'llamacpp' },
  { label: 'OpenAI Compatible', value: 'openai' },
  { label: '自定义 API', value: 'custom' },
]

export const FORM_RULES: FormRules = {
  apiType: { required: true, message: '请选择 API 类型', trigger: 'change' },
  endpoint: { required: true, message: '请输入 Endpoint 地址', trigger: 'blur' },
  modelName: { required: true, message: '请输入模型名称', trigger: 'blur' },
  customProviderName: { required: true, message: '请输入自定义提供商名称', trigger: 'blur' },
  customEmbeddingPath: { required: true, message: '请输入 Embedding 路径', trigger: 'blur' },
}

export function useSettings() {
  const message = useMessage()
  const { send, on } = useExtension()

  const config = ref<AppConfig>({ ...DEFAULT_CONFIG })
  const saving = ref(false)
  const loading = ref(false)
  let savingMsg: MessageReactive | null = null

  const needsApiKey = computed(() => config.value.apiType === 'openai' || config.value.apiType === 'custom')
  const isCustomApi = computed(() => config.value.apiType === 'custom')

  function onApiTypeChange(value: string) {
    config.value.apiType = value as ApiType
    if (value === 'ollama' && !config.value.endpoint) {
      config.value.endpoint = 'http://localhost:11434'
    } else if (value === 'openai' && !config.value.endpoint) {
      config.value.endpoint = 'https://api.openai.com'
      config.value.customEmbeddingPath = '/v1/embeddings'
    } else if (value === 'custom') {
      config.value.customProviderName = ''
      config.value.customEmbeddingPath = '/v1/embeddings'
    }
  }


  function saveConfig() {
    saving.value = true
    savingMsg = message.loading('正在保存配置...', { duration: 0 })
    send('saveConfig', {
      apiType: config.value.apiType,
      endpoint: config.value.endpoint,
      apiKey: needsApiKey.value ? config.value.apiKey : null,
      modelName: config.value.modelName,
      folders: config.value.folders,
      customProviderName: isCustomApi.value ? config.value.customProviderName : null,
      customEmbeddingPath: isCustomApi.value ? config.value.customEmbeddingPath : null,
    })
  }

  function loadConfig() {
    loading.value = true
    send('loadConfig', {})
  }

  function setupEvents() {
    on('configLoaded', (data: unknown) => {
      const configData = data as Partial<AppConfig> | null
      if (configData) {
        config.value.apiType = (configData.apiType as ApiType) || ''
        config.value.endpoint = configData.endpoint || ''
        config.value.apiKey = configData.apiKey || ''
        config.value.modelName = configData.modelName || ''
        config.value.folders = (configData.folders as string[]) || []
        config.value.customProviderName = configData.customProviderName || ''
        config.value.customEmbeddingPath = configData.customEmbeddingPath || '/v1/embeddings'
      }
      loading.value = false
    })

    on('configSaved', () => {
      savingMsg?.destroy()
      saving.value = false
      message.success('配置保存成功 ✅')
    })

    on('configSaveError', (data: unknown) => {
      savingMsg?.destroy()
      saving.value = false
      const err = data as { error?: string }
      message.error(`保存配置失败: ${err?.error || '未知错误'}`)
    })
  }

  onMounted(() => {
    loadConfig()
  })

  setupEvents()
  return {
    config,
    saving,
    loading,
    needsApiKey,
    isCustomApi,
    onApiTypeChange,
    saveConfig,
    loadConfig,
  }
}