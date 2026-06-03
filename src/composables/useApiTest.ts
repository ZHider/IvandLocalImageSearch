import { ref, onMounted, onUnmounted, type Ref } from 'vue'
import { useMessage } from 'naive-ui'
import { useExtension } from './useExtension'
import type { AppConfig, ApiTestResult } from '../types'

export function useApiTest(config: Ref<AppConfig>) {
  const message = useMessage()
  const { send, on } = useExtension()

  const testing = ref(false)
  const testResult = ref<ApiTestResult | null>(null)
  const availableModels = ref<string[]>([])

  const cleanups: (() => void)[] = []

  onMounted(() => {
    cleanups.push(
      on('apiConnectionResult', (data: unknown) => {
        testing.value = false
        const result = data as ApiTestResult
        testResult.value = result
        if (result.models && result.models.length > 0) {
          availableModels.value = result.models
        }
        if (result.success) {
          message.success('连接成功 ✅')
        } else {
          message.error(`连接失败: ${result.message}`)
        }
      })
    )
  })

  onUnmounted(() => {
    cleanups.forEach(stop => stop())
    cleanups.length = 0
  })

  function testConnection() {
    testing.value = true
    testResult.value = null

    // 映射前端 apiType 到后端 provider
    const providerMap: Record<string, string> = {
      vllm: 'vllm',
      dashscope: 'dashscope',
      custom: 'vllm',
    }
    const provider = providerMap[config.value.apiType] || 'vllm'
    const needsApiKey = config.value.apiType === 'vllm' || config.value.apiType === 'dashscope' || config.value.apiType === 'custom'

    send('testApiConnection', {
      provider,
      base_url: config.value.endpoint,
      api_key: needsApiKey ? config.value.apiKey : null,
      model: config.value.modelName,
      vision_model: null,
    })
  }

  return {
    testing,
    testResult,
    testConnection,
    availableModels,
  }
}
