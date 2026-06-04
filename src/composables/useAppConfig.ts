import { ref, computed } from 'vue'
import type { MessageReactive } from 'naive-ui'
import { useMessage } from 'naive-ui'
import { useExtension } from './useExtension'
import { DEFAULT_CONFIG, type AppConfig, type ApiType } from '../types'

const config = ref<AppConfig>({ ...DEFAULT_CONFIG })
const loaded = ref(false)
const loading = ref(false)
const saving = ref(false)

let initialized = false
let savingMsg: MessageReactive | null = null
let initialLoadDispatched = false

function init() {
  if (initialized) return
  initialized = true

  const message = useMessage()
  const { on } = useExtension()

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
      config.value.imageProcessing = {
        embedImageSize: configData.imageProcessing?.embedImageSize ?? DEFAULT_CONFIG.imageProcessing.embedImageSize,
        thumbnailSize: configData.imageProcessing?.thumbnailSize ?? DEFAULT_CONFIG.imageProcessing.thumbnailSize,
      }
      config.value.embedThreads = configData.embedThreads ?? DEFAULT_CONFIG.embedThreads
    }
    loaded.value = true
    loading.value = false
  })

  on('configSaved', () => {
    savingMsg?.destroy()
    saving.value = false
    message.success('配置保存成功')
  })

  on('configSaveError', (data: unknown) => {
    savingMsg?.destroy()
    saving.value = false
    const err = data as { error?: string }
    message.error(`保存配置失败: ${err?.error || '未知错误'}`)
  })
}

export function useAppConfig() {
  const message = useMessage()
  const { send } = useExtension()

  init()

  if (!initialLoadDispatched) {
    initialLoadDispatched = true
    loading.value = true
    send('loadConfig', {})
    setTimeout(() => {
      if (!loaded.value) {
        send('loadConfig', {})
      }
    }, 300)
  }

  const needsApiKey = computed(() => config.value.apiType === 'vllm' || config.value.apiType === 'dashscope' || config.value.apiType === 'custom')
  const isCustomApi = computed(() => config.value.apiType === 'custom')

  function onApiTypeChange(value: string) {
    config.value.apiType = value as ApiType
    if (value === 'vllm') {
      config.value.endpoint = 'http://localhost:8000'
    } else if (value === 'dashscope') {
      config.value.endpoint = 'https://dashscope.aliyuncs.com'
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
      imageProcessing: config.value.imageProcessing,
      embedThreads: config.value.embedThreads,
    })
  }

  function loadConfig() {
    loading.value = true
    send('loadConfig', {})
  }

  return {
    config,
    loaded,
    loading,
    saving,
    needsApiKey,
    isCustomApi,
    saveConfig,
    loadConfig,
    onApiTypeChange,
  }
}
