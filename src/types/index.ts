export type ApiType = 'ollama' | 'llamacpp' | 'openai' | 'custom' | ''

export interface ImageProcessingConfig {
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
  embedThreads: number
  advancedOptions: AdvancedOptions
}

export interface ImageProcessingConfig {
  embedImageSize: number
  thumbnailSize: number
}

export const DEFAULT_IMAGE_PROCESSING: ImageProcessingConfig = {
  embedImageSize: 1920,
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
  embedThreads: 1,
  advancedOptions: { ...DEFAULT_ADVANCED_OPTIONS },
}

export const DEFAULT_NATIVE_EXTENSIONS = [
  'jpg', 'jpeg', 'png', 'webp', 'bmp', 'tiff', 'ico',
]

export interface AdvancedOptions {
  webpThresholdMB: number
  nativeExtensions: string[]
  extraEmbeddingParams: string
}

export const DEFAULT_ADVANCED_OPTIONS: AdvancedOptions = {
  webpThresholdMB: 4,
  nativeExtensions: [...DEFAULT_NATIVE_EXTENSIONS],
  extraEmbeddingParams: '',
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

export interface IndexProgress {
  phase: string
  current: number
  total: number
  percentage: number
  currentFile?: string
  deletedFile?: string
  newCount: number
  modifiedCount: number
  deletedCount: number
  errorCount?: number
}

export interface IndexResultFile {
  file_path: string
  file_size: number
  modified_at: number
  file_hash: string
}


export interface IndexFileMeta {
  filePath: string
  fileHash: string
  indexedAt: string
}

export interface QueryIndexParams {
  pathFilter?: string
  timeAfter?: string
  timeBefore?: string
  page?: number
  pageSize?: number
}

export interface QueryIndexResult {
  files: IndexFileMeta[]
  total: number
}

export interface IndexResult {
  files: IndexResultFile[]
  total: number
  newCount: number
  modifiedCount: number
  deletedCount: number
  errorCount?: number
}

export interface ApiConnectionParams {
  provider: string
  base_url: string
  api_key: string | null
  model: string
  vision_model: string | null
}