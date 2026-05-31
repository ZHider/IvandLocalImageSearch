export type ApiType = 'ollama' | 'llamacpp' | 'openai' | 'custom' | ''

export interface AppConfig {
  apiType: ApiType
  endpoint: string
  apiKey: string
  modelName: string
  folders: string[]
  customProviderName: string
  customEmbeddingPath: string
}

export const DEFAULT_CONFIG: AppConfig = {
  apiType: '',
  endpoint: '',
  apiKey: '',
  modelName: '',
  folders: [],
  customProviderName: '',
  customEmbeddingPath: '/v1/embeddings',
}

export interface ApiTestResult {
  success: boolean
  message: string
}

export interface ApiConnectionParams {
  provider: string
  base_url: string
  api_key: string | null
  model: string
  vision_model: string | null
}