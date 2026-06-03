# 🚀 vLLM 部署指南

本指南说明如何在 Windows 环境下部署 vLLM 服务，为魔王图片搜索提供嵌入 API。

> 如果不想自行部署 vLLM，可以直接使用阿里云百炼的托管 API 服务，无需管理 GPU 服务器。特别推荐其 **QwenVL Embedding 系列多模态模型**，支持文本、图片融合向量化，非常适合魔王图片搜索的场景。

## 方案一：阿里云百炼托管 API（推荐）

> 阿里云百炼提供托管的向量化（Embedding）API 服务，无需自行部署 GPU 服务器，按量付费，即开即用。
> 特别适合没有 GPU 硬件或不想折腾环境搭建的 Windows 用户。

### 支持的模型

| 模型 | 类型 | 向量维度 | 融合向量 | 独立向量 | 适用场景 |
|------|------|---------|---------|---------|---------|
| `qwen3-vl-embedding` | 多模态 | 2560（默认）/ 2048 / 1536 / 1024 / 768 / 512 / 256 | ✅（`enable_fusion=True`） | ✅ | **推荐**。文本、图片、视频融合/独立向量，文搜图、图搜图、跨模态检索 |
| `qwen2.5-vl-embedding` | 多模态 | 2048 | ✅（仅融合） | ❌ | 跨模态检索 |
| `tongyi-embedding-vision-plus-2026-03-06` | 多模态 | 1152（默认）/ 1024 / 512 / 256 / 128 / 64 | ✅（同对象融合） | ✅ | 图文检索，同时支持融合和独立向量 |
| `tongyi-embedding-vision-flash-2026-03-06` | 多模态 | 兼容版 | ✅（同对象融合） | ✅ | 轻量级图文检索 |
| `text-embedding-v4` | 纯文本 | 2048 / 1536 / 1024（默认）/ 768 / 512 / 256 / 128 / 64 | — | ✅ | 纯文本检索，性能最强，100+语种支持 |

> 对于魔王图片搜索的**图片嵌入**场景，推荐使用 `qwen3-vl-embedding` 多模态模型，可直接将图片内容编码为向量。

### 1. 获取 API Key

1. 访问 [阿里云百炼控制台](https://bailian.console.aliyun.com/) 注册并登录（需实名认证）
2. 进入 **密钥管理** 页面，创建 API Key（格式如 `sk-xxx`）
3. 保存 API Key，注意北京地域与新加坡地域密钥不互通

### 2. API 地址

阿里云百炼提供 OpenAI 兼容接口：

```
Base URL: https://dashscope.aliyuncs.com/compatible-mode/v1
Embedding: POST https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings
```

### 3. 配置到魔王图片搜索

在应用设置页面：

- **API 类型**: 选择「OpenAI 兼容」
- **Endpoint**: `https://dashscope.aliyuncs.com/compatible-mode/v1`
- **API Key**: 填写上一步获取的 `sk-xxx`
- **模型名称**: 填写 `qwen3-vl-embedding`（推荐）或 `text-embedding-v4`

### 4. 验证服务（可选）

```powershell
# PowerShell 测试文本嵌入
$body = @"
{
  "model": "text-embedding-v4",
  "input": "一只猫在沙发上睡觉",
  "encoding_format": "float"
}
"@

Invoke-RestMethod -Uri "https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings" `
  -Method Post `
  -Headers @{ "Authorization" = "Bearer $env:DASHSCOPE_API_KEY" } `
  -ContentType "application/json" `
  -Body $body
```

### 5. 计费与免费额度

| 模型 | 单价 | 免费额度 |
|------|------|---------|
| `text-embedding-v4` | 0.0005 元/千 Token | 100 万 Token（开通后 90 天内） |
| `qwen3-vl-embedding` | 按图片/视频尺寸计费 | 请参考官方文档 |

> 详细价格请参考：[阿里云百炼计费说明](https://www.alibabacloud.com/help/zh/model-studio/billing-for-model-studio)

### 6. 多模态 Embedding 详解（进阶）

向量化模型可将文本、图像、视频等数据转换为数值向量，用于语义搜索、推荐、聚类、分类、异常检测等下游任务。

#### 准备工作

已获取 API Key 并配置到环境变量 `DASHSCOPE_API_KEY`。如需通过 Python DashScope SDK 调用，还需安装 SDK：

```bash
pip install dashscope
```

#### 融合向量 vs 独立向量

- **独立向量**：为每个输入（如图片和其对应的文字标题）分别生成独立的向量
- **融合向量**：将文本、图片、视频等不同模态的内容融合成一个向量，适用于文搜图、图搜图、文搜视频、跨模态检索等场景

> **重要说明**：多模态融合向量功能需要通过 **Python DashScope SDK 或 HTTP API** 来调用，暂不支持 OpenAI 兼容接口、Java DashScope SDK 调用或在控制台直接使用。如果需要在应用中调用融合向量，需在 Rust 扩展中直接请求阿里云百炼的 DashScope HTTP API。

#### Python 示例：使用 qwen3-vl-embedding 生成融合向量

```python
import dashscope
import json
import os

# 多模态融合向量：将文本、图片、视频融合成一个融合向量
# 适用于跨模态检索、图搜等场景
text = "这是一段测试文本，用于生成多模态融合向量"
image = "https://dashscope.oss-cn-beijing.aliyuncs.com/images/256_1.png"
video = "https://help-static-aliyun-doc.aliyuncs.com/file-manage-files/zh-CN/20250107/lbcemt/new+video.mp4"

# 输入包含文本、图片、视频，通过 enable_fusion 参数生成融合向量
input_data = [
    {"text": text},
    {"image": image},
    {"video": video}
]

resp = dashscope.MultiModalEmbedding.call(
    api_key=os.getenv("DASHSCOPE_API_KEY"),
    model="qwen3-vl-embedding",
    input=input_data,
    enable_fusion=True,
    # 可选参数：指定向量维度（支持 2560, 2048, 1536, 1024, 768, 512, 256，默认 2560）
    # dimension=1024
)

print(json.dumps(resp.output, indent=4))
```

#### Python 示例：使用 tongyi-embedding-vision-plus-2026-03-06 生成融合向量

与 qwen3-vl-embedding 不同，该模型通过将 text、image、video 放在同一个 content 对象中实现融合，无需 `enable_fusion` 参数：

```python
import dashscope
import json
import os

# 将 text、image 放在同一个 content 对象中，无需 enable_fusion 参数
text = "白色运动鞋，轻量透气，适合跑步和日常穿着"
image = "https://dashscope.oss-cn-beijing.aliyuncs.com/images/256_1.png"

# 同一对象中的多模态内容会被融合为 1 个向量（type 为 "fused"）
input_data = [
    {"text": text, "image": image}
]

resp = dashscope.MultiModalEmbedding.call(
    api_key=os.getenv("DASHSCOPE_API_KEY"),
    model="tongyi-embedding-vision-plus-2026-03-06",
    input=input_data,
    # 可选参数：指定向量维度（支持 1152, 1024, 512, 256, 128, 64，默认 1152）
    dimension=1152
)

print(json.dumps(resp.output, indent=4))
```

#### 模型选择指南

| 场景 | 推荐模型 | 说明 |
|------|---------|------|
| 纯文本/代码检索 | `text-embedding-v4` | 性能最强，支持任务指令（instruct）、稀疏向量等高级功能 |
| 跨模态检索（文搜图、图搜图） | `qwen3-vl-embedding`（融合向量模式） | 输入一张图片并附加文本指令，模型将图像和文本融合成一个向量进行理解 |
| 图文独立向量 | `tongyi-embedding-vision-plus-2026-03-06` | 为每个输入（图片、文字）分别生成独立向量 |
| 大规模非实时文本处理 | `text-embedding-v4` + Batch 调用 | 显著降低成本 |

## 方案二：vllm-windows 原生部署（本机部署）

> 适合 Windows 上拥有 NVIDIA 显卡的用户，直接在 Windows 系统中运行 vLLM，无需虚拟机或 Docker。
>
> 项目地址：[github.com/SystemPanic/vllm-windows](https://github.com/SystemPanic/vllm-windows)

### 1. 环境要求

- Windows 10/11（64 位）
- NVIDIA 显卡，显存 >= 8GB（推荐 16GB+）
- [NVIDIA 驱动程序](https://www.nvidia.com/Download/index.aspx) >= 535
- [CUDA Toolkit](https://developer.nvidia.com/cuda-downloads) >= 12.4
- [Python](https://www.python.org/downloads/) >= 3.9（安装时勾选"Add to PATH"）
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（安装时勾选"Desktop development with C++"）
- 磁盘空间 >= 20GB

### 2. 安装 CUDA

```powershell
# 1. 下载 CUDA Toolkit 12.4
#    访问 https://developer.nvidia.com/cuda-12-4-0-download-archive
#    选择 Windows → exe (local)

# 2. 安装完成后设置环境变量（通常安装程序会自动配置）
#    验证安装：
nvcc --version
```

### 3. 创建虚拟环境并安装 vllm-windows

```powershell
# 创建虚拟环境
python -m venv vllm-env

# 激活虚拟环境
.\vllm-env\Scripts\Activate.ps1

# 安装 vllm-windows（从 GitHub 直接安装预编译 wheel）
pip install https://github.com/SystemPanic/vllm-windows/releases/download/v0.8.3/vllm-0.8.3-cp312-cp312-win_amd64.whl
```

> 如果提示找不到 wheel 文件，请前往 [Releases 页面](https://github.com/SystemPanic/vllm-windows/releases) 查看最新版本号并替换上面的 URL。

### 5. 启动服务

```powershell
# 确保虚拟环境已激活，然后启动：
vllm serve BAAI/bge-m3 ^
  --task embedding ^
  --max-model-len 8192 ^
  --gpu-memory-utilization 0.9 ^
  --dtype auto ^
  --host 0.0.0.0 ^
  --port 8000
```

> 首次启动会自动下载模型，请保持网络畅通。模型下载到 `%USERPROFILE%\.cache\huggingface\` 目录。

### 6. 验证服务

打开另一个 PowerShell 窗口：

```powershell
# 检查服务状态
Invoke-RestMethod -Uri http://localhost:8000/v1/models
```

### 7. 配置到魔王图片搜索

在应用设置页面：
- **Endpoint**: `http://localhost:8000`
- **模型名称**: `BAAI/bge-m3`
- **API Key**: 留空

## 推荐模型

以下模型均支持 Chat Embeddings 协议，可直接用于魔王图片搜索：

| 模型 | 向量维度 | 显存需求 | 语言支持 |
|------|---------|---------|---------|
| `BAAI/bge-m3` | 1024 | ~4GB | 多语言（中英文效果优秀） |
| `BAAI/bge-large-zh-v1.5` | 1024 | ~4GB | 中文为主 |
| `intfloat/multilingual-e5-large` | 1024 | ~4GB | 多语言 |
| `Alibaba-NLP/gte-Qwen2-1.5B-instruct` | 1536 | ~6GB | 多语言 |

> 对于纯文本搜索场景，以上模型可直接使用。如果需要进行**图片嵌入**，请使用支持多模态的模型（如 `Qwen2-VL` 系列），且需确认云平台提供足够的显存（推荐 24GB+）。

## 验证服务

服务启动后，可用 curl（Windows 10/11 自带）或 PowerShell 测试：

### 检查服务状态

```powershell
# PowerShell
Invoke-RestMethod -Uri http://localhost:8000/v1/models
```

### 测试文本嵌入

```powershell
# PowerShell
$body = @"
{
  "model": "BAAI/bge-m3",
  "messages": [
    {"role": "system", "content": [{"type": "text", "text": "Represent the user's input."}]},
    {"role": "user", "content": [{"type": "text", "text": "一只猫在沙发上睡觉"}]},
    {"role": "assistant", "content": [{"type": "text", "text": ""}]}
  ],
  "encoding_format": "float",
  "continue_final_message": true,
  "add_special_tokens": true
}
"@

Invoke-RestMethod -Uri http://localhost:8000/v1/embeddings `
  -Method Post `
  -ContentType "application/json" `
  -Body $body
```

正常返回应包含 `data[0].embedding` 数组（1024 个浮点数）。

## 性能调优

### 关键启动参数

| 参数 | 说明 | 建议值 |
|------|------|--------|
| `--max-model-len` | 最大上下文长度 | 8192（根据显存调整） |
| `--gpu-memory-utilization` | GPU 显存利用率 | 0.85~0.95 |
| `--dtype` | 精度模式 | `auto` 或 `bfloat16` |
| `--max-num-seqs` | 最大并发序列数 | 256 |
| `--enable-chunked-prefill` | 分块预填充 | 长文本场景推荐启用 |

### 多 GPU 部署

```bash
vllm serve BAAI/bge-m3 \
  --task embedding \
  --tensor-parallel-size 2 \
  --gpu-memory-utilization 0.9 \
  --dtype auto
```

## 常见问题

### CUDA out of memory

- 降低 `--gpu-memory-utilization`（如改为 0.6）
- 使用更小的模型（如 `BAAI/bge-small-en-v1.5`，仅需 ~2GB 显存）
- 减少 `--max-model-len`（如改为 4096）

### 请求返回 404

确认请求路径为 `/v1/embeddings`（不是 `/embeddings`），且模型名与启动时一致。

### 请求超时

大图片或长文本可能需要更长时间处理，可调长超时时间：

```bash
vllm serve BAAI/bge-m3 \
  --task embedding \
  --max-model-len 16384
```
