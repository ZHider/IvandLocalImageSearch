import { ref, type Ref } from 'vue'
import { useMessage } from 'naive-ui'
import { useExtension } from './useExtension'
import type { AppConfig, ApiTestResult } from '../types'

export function useApiTest(config: Ref<AppConfig>) {
  const message = useMessage()
  const { send, on } = useExtension()

  const testing = ref(false)
  const testResult = ref<ApiTestResult | null>(null)

  function testConnection() {
    testing.value = true
    testResult.value = null

    const needsApiKey = config.value.apiType === 'openai' || config.value.apiType === 'custom'
    const provider = config.value.apiType === 'ollama' ? 'ollama' : 'openai'

    send('testApiConnection', {
      provider,
      base_url: config.value.endpoint,
      api_key: needsApiKey ? config.value.apiKey : null,
      model: config.value.modelName,
      vision_model: null,
    })
  }

  function setupEvents() {
    on('apiConnectionResult', (data: unknown) => {
      testing.value = false
      const result = data as ApiTestResult
      testResult.value = result
      if (result.success) {
        message.success('连接成功 ✅')
      } else {
        message.error(`连接失败: ${result.message}`)
      }
    })
  }

  setupEvents()

  return {
    testing,
    testResult,
    testConnection,
  }
}