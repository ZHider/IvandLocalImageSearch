# 🔌 Embedding API 协议说明

应用使用 **vLLM Chat Embeddings 扩展协议** 调用嵌入 API，格式与标准的 `/v1/chat/completions` 类似，但使用 `/v1/embeddings` 端点。

## 请求格式

### 文本嵌入

应用将搜索关键词或文本文件内容编码为以下 JSON 发送：

```json
POST /v1/embeddings
Authorization: Bearer <api-key>  // 可选
Content-Type: application/json

{
  "model": "BAAI/bge-m3",
  "messages": [
    {
      "role": "system",
      "content": [
        { "type": "text", "text": "Represent the user's input." }
      ]
    },
    {
      "role": "user",
      "content": [
        { "type": "text", "text": "一只猫在沙发上睡觉" }
      ]
    },
    {
      "role": "assistant",
      "content": [
        { "type": "text", "text": "" }
      ]
    }
  ],
  "encoding_format": "float",
  "continue_final_message": true,
  "add_special_tokens": true
}
```

### 图片嵌入

应用将图片读取为 Base64 Data URL，构造如下请求：

```json
POST /v1/embeddings
Authorization: Bearer <api-key>  // 可选
Content-Type: application/json

{
  "model": "BAAI/bge-m3",
  "messages": [
    {
      "role": "system",
      "content": [
        { "type": "text", "text": "Represent the user's input." }
      ]
    },
    {
      "role": "user",
      "content": [
        {
          "type": "image_url",
          "image_url": {
            "url": "data:image/webp;base64,/9j/4AAQ..."
          }
        }
      ]
    },
    {
      "role": "assistant",
      "content": [
        { "type": "text", "text": "" }
      ]
    }
  ],
  "encoding_format": "float",
  "continue_final_message": true,
  "add_special_tokens": true
}
```

> 图片支持同时附带文本说明：在 `user` 角色的 `content` 数组中同时放入 `image_url` 和 `text` 类型的元素即可。

### 额外参数

可在设置页面配置 `extra_embedding_params`（JSON 对象），该参数会注入到请求体的 `parameters` 字段中：

```json
{
  "model": "BAAI/bge-m3",
  "messages": [...],
  "parameters": {
    "pooling_type": "cls",
    "normalize": true
  }
}
```

## 预期返回格式

应用解析以下响应结构（支持多种变体）：

### OpenAI 标准格式（推荐）

```json
{
  "object": "list",
  "data": [
    {
      "object": "embedding",
      "index": 0,
      "embedding": [0.0123, -0.0456, 0.0789, ...]
    }
  ],
  "model": "BAAI/bge-m3",
  "usage": {
    "prompt_tokens": 8,
    "total_tokens": 8
  }
}
```

### vLLM Chat Embeddings 格式

```json
{
  "id": "embd-xxx",
  "object": "list",
  "created": 1700000000,
  "model": "BAAI/bge-m3",
  "data": [
    {
      "index": 0,
      "embedding": [
        [0.0123, -0.0456, 0.0789, ...]  // 注意：内层嵌套数组
      ]
    }
  ],
  "usage": {
    "prompt_tokens": 8,
    "total_tokens": 8
  }
}
```

> 应用兼容 `data[0].embedding` 为**一维数组**或**二维数组**（内层嵌套）两种格式。

### 纯数组格式

部分 API 直接返回向量数组：

```json
[
  [0.0123, -0.0456, 0.0789, ...]
]
```

## 请求参数说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `model` | string | 嵌入模型名称，如 `BAAI/bge-m3`、`intfloat/multilingual-e5-large` |
| `messages` | array | Chat 格式消息列表，包含 system / user / assistant 三条消息 |
| `encoding_format` | string | 固定为 `"float"` |
| `continue_final_message` | bool | 固定为 `true`，vLLM 兼容需要 |
| `add_special_tokens` | bool | 固定为 `true` |
| `parameters` | object | 可选，额外模型参数（如 pooling 策略） |

## 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `data[0].embedding` | number[] | 浮点数向量数组，长度由模型决定（如 1024、768） |
| `usage.prompt_tokens` | number | 请求消耗的 token 数 |