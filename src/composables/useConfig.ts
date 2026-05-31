import { ref, readonly } from 'vue'
import { useExtension } from './useExtension'
import { DEFAULT_CONFIG, type AppConfig, type ApiType } from '../types'

// 模块级单例——所有组件共享同一份配置
const config = ref<AppConfig>({ ...DEFAULT_CONFIG })
const loaded = ref(false)
const loading = ref(false)

let initialized = false

function init() {
  if (initialized) return
  initialized = true

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
    }
    loaded.value = true
    loading.value = false
  })

  // 配置保存后自动重新加载
  on('configSaved', () => {
    const { send } = useExtension()
    send('loadConfig', {})
  })

  // 首次加载被 configSaved 触发时重设 loaded 状态
  on('configSaveError', () => {
    // 不处理保存失败
  })
}

export function useConfig() {
  init()

  function load() {
    loading.value = true
    const { send } = useExtension()
    send('loadConfig', {})
  }

  function reload() {
    loaded.value = false
    loading.value = true
    const { send } = useExtension()
    send('loadConfig', {})
  }

  return {
    config: readonly(config),
    loaded: readonly(loaded),
    loading: readonly(loading),
    load,
    reload,
  }
}
