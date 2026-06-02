export type ApiType = 'ollama' | 'llamacpp' | 'openai' | 'custom' | ''

export interface ImageProcessingConfig {
  embedImageSize: number
  thumbnailSize: number
}

export interface AppConfig {
  apiType: ApiType
  endpoint: string
  apiKey: string
  modelName: string
  folders: string[]
  customProviderName: string
  customEmbeddingPath: string
  imageProcessing: ImageProcessingConfig
}

export const DEFAULT_IMAGE_PROCESSING: ImageProcessingConfig = {
  embedImageSize: 512,
  thumbnailSize: 300,
}

export const DEFAULT_CONFIG: AppConfig = {
  apiType: '',
  endpoint: '',
  apiKey: '',
  modelName: '',
  folders: [],
  customProviderName: '',
  customEmbeddingPath: '/v1/embeddings',
  imageProcessing: { ...DEFAULT_IMAGE_PROCESSING },
}

export interface ApiTestResult {
  success: boolean
  message: string
  models?: string[]
}

export interface SearchResult {
  file_path: string
  file_name: string
  file_type: string
  file_size: number
  similarity: number
  thumbnail_path: string
  text_preview?: string
  exif?: {
    camera_make?: string
    camera_model?: string
    iso?: string
    aperture?: string
    shutter_speed?: string
    focal_length?: string
    date_taken?: string
  }
}

export interface ApiConnectionParams {
  provider: string
  base_url: string
  api_key: string | null
  model: string
  vision_model: string | null
}