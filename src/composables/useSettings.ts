import type { FormRules } from 'naive-ui'

export interface ApiTypeOption {
  label: string
  value: string
}

export const API_TYPE_OPTIONS: ApiTypeOption[] = [
  { label: 'vLLM', value: 'vllm' },
  { label: 'DashScope', value: 'dashscope' },
  { label: '自定义 API', value: 'custom' },
]

export const FORM_RULES: FormRules = {
  apiType: { required: true, message: '请选择 API 类型', trigger: 'change' },
  endpoint: { required: true, message: '请输入 Endpoint 地址', trigger: 'blur' },
  customProviderName: { required: true, message: '请输入自定义提供商名称', trigger: 'blur' },
  customEmbeddingPath: { required: true, message: '请输入 Embedding 路径', trigger: 'blur' },
}
