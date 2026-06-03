# 🔌 Embedding API 协议说明

应用目前支持的 Embedding 协议：

1. **vLLM Chat Embeddings 协议**（兼容 OpenAI）—— 用于本地 vLLM 部署（图片嵌入）
2. **DashScope 多模态 Embedding 协议** —— 用于阿里云百炼（图片检索）

---

## vLLM Chat Embeddings 协议（兼容 OpenAI）

使用 `/v1/embeddings` 端点，格式与标准的 `/v1/chat/completions` 类似，但仅用于图片嵌入。

### 图片嵌入

```json
POST /v1/embeddings
Authorization: Bearer <api-key>
Content-Type: application/json

{
  "model": "LifetimeMistake/Qwen3-VL-Embedding-2B-AWQ-4bit",
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

### 额外参数

可在设置页面配置 `extra_embedding_params`（JSON 对象），该参数会注入到请求体的 `parameters` 字段中：

```json
{
  "model": "LifetimeMistake/Qwen3-VL-Embedding-2B-AWQ-4bit",
  "messages": [...],
  "parameters": {
    "pooling_type": "cls",
    "normalize": true
  }
}
```

### 请求参数说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `model` | string | 嵌入模型名称 |
| `messages` | array | Chat 格式消息列表，包含 system / user / assistant 三条消息 |
| `encoding_format` | string | 固定为 `"float"` |
| `continue_final_message` | bool | 固定为 `true`，vLLM 兼容需要 |
| `add_special_tokens` | bool | 固定为 `true` |
| `parameters` | object | 可选，额外模型参数（如 pooling 策略） |

### 预期返回格式

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
  "model": "LifetimeMistake/Qwen3-VL-Embedding-2B-AWQ-4bit",
  "usage": {
    "prompt_tokens": 8,
    "total_tokens": 8
  }
}
```

> 应用也兼容 `data[0].embedding` 为**二维数组**（内层嵌套 `[[...]]`）的格式，以及直接返回向量数组的纯数组格式。

### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `data[0].embedding` | number[] | 浮点数向量数组，长度由模型决定 |
| `usage.prompt_tokens` | number | 请求消耗的 token 数 |

## DashScope 多模态 Embedding 协议

阿里云百炼多模态 Embedding 使用专用 HTTP API，非 OpenAI 兼容接口。

### 端点

```
POST https://dashscope.aliyuncs.com/api/v1/services/embeddings/multimodal-embedding/multimodal-embedding
```

### 请求格式

```json
{
  "model": "qwen3-vl-embedding",
  "input": {
    "contents": [
      {"text": "描述文本（可选）"},
      {"image": "data:image/jpeg;base64,..."},
      {"video": "https://...（仅支持公开URL）"}
    ]
  },
  "parameters": {
    "dimension": 2560,
    "enable_fusion": false,
    "instruct": "Represent the image for search"
  }
}
```

### contents 结构

| 字段 | 类型 | 说明 |
|------|------|------|
| `text` | string | 文本内容。也可直接传字符串 `"纯文本"` 替代 `{"text": "..."}` |
| `image` | string | 图片 URL 或 Base64 Data URI。格式：`data:image/{format};base64,{data}` |
| `video` | string | 视频 URL（仅支持公开可访问的 URL） |
| `multi_images` | array | 多图序列，每项为图片 URL 或 Data URI。仅 `tongyi-embedding-vision-plus`/`flash` 及其 2026-03-06 快照支持 |

单次请求内容元素总数限制：`qwen3-vl-embedding` 不超过 **20** 个（图片≤5，视频≤1）；2026-03-06 系列不超过 **20** 个（图片≤64，视频≤8）。

### parameters 参数

| 参数 | 类型 | 适用模型 | 说明 |
|------|------|---------|------|
| `dimension` | int | qwen3 / 2026-03-06 系列 | 输出向量维度。不同模型支持不同选项（见下方详表） |
| `enable_fusion` | bool | `qwen3-vl-embedding` | `false` 为独立向量（每个输入各生成一个向量）；`true` 为融合向量（所有输入融合为一个） |
| `instruct` | string | 全部 | 自定义任务说明，帮助模型理解查询意图。建议英文，通常可提升 1-5% 效果 |
| `res_level` | int | 2026-03-06 系列 | 分辨率档位（0/1/2/3），默认 1。高分辨率场景（IPC/文字识别）推荐 3 |
| `max_video_frames` | int | 2026-03-06 系列 | 视频最大采样帧数，默认 8，最大 64 |
| `output_type` | string | 全部 | 输出格式，当前仅支持 `dense` |

#### dimension 可选值

| 模型 | 可选维度（默认值加粗） |
|------|---------------------|
| `qwen3-vl-embedding` | **2560** / 2048 / 1536 / 1024 / 768 / 512 / 256 |
| `qwen2.5-vl-embedding` | **1024** / 2048 / 768 / 512 |
| `tongyi-embedding-vision-plus-2026-03-06` | **1152** / 1024 / 512 / 256 / 128 / 64 |
| `tongyi-embedding-vision-flash-2026-03-06` | **768** / 512 / 256 / 128 / 64 |
| 其他旧版模型 | 不支持此参数，维度固定 |

### 向量类型

| 类型 | 说明 |
|------|------|
| **独立向量** | 为 `contents` 中的每个输入分别生成一个向量。输入 1 段文本 + 1 张图片 → 返回 2 个向量。适用于逐项对比（以图搜图、以文搜图） |
| **融合向量** | 将 `contents` 中所有输入融合为 1 个向量。适用于图文联合检索、商品图文统一表征 |

融合向量的实现方式：

| 模型 | 融合方式 |
|------|---------|
| `qwen3-vl-embedding` | 设置 `parameters.enable_fusion = true` |
| `qwen2.5-vl-embedding` | **始终**返回融合向量，不支持独立向量 |
| `tongyi-embedding-vision-plus-2026-03-06` (及 flash) | 将 text、image、video 放在**同一个 content 对象**中实现融合，无需 `enable_fusion` |

### 支持的图片格式

支持以下图片格式（通过文件扩展名自动推断 MIME 类型）：

**常见格式：** JPEG、PNG、WEBP、BMP、TIFF
**扩展格式：** ICO、DIB、ICNS、SGI

> 图片可通过公开 URL 或 Base64 Data URI 传入。本项目使用 Base64 编码本地图片，格式为 `data:image/{format};base64,{data}`。

### 响应格式

```json
{
  "output": {
    "embeddings": [
      {
        "index": 0,
        "embedding": [0.0123, -0.0456, ...],
        "type": "image"
      }
    ]
  },
  "usage": {
    "input_tokens": 432,
    "input_tokens_details": {
      "image_tokens": 402,
      "text_tokens": 30
    },
    "output_tokens": 1,
    "total_tokens": 433
  },
  "request_id": "1fff9502-a6c5-9472-9ee1-73930fdd04c5"
}
```

### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `output.embeddings[].index` | int | 该结果在输入 `contents` 中的索引位置 |
| `output.embeddings[].embedding` | number[] | 浮点数向量数组 |
| `output.embeddings[].type` | string | 结果类型：`text` / `image` / `video` / `multi_images` / `fused`（融合向量）/ `fusion`（qwen 融合向量） |
| `usage.input_tokens` | int | 输入总 Token 数 |
| `usage.input_tokens_details.image_tokens` | int | 图片/视频占用的 Token 数 |
| `usage.input_tokens_details.text_tokens` | int | 文本占用的 Token 数 |
| `usage.output_tokens` | int | 输出 Token 数（通常为 1） |
| `request_id` | string | 请求唯一标识，可用于问题排查 |
