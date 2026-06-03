# 🚀 LLM / Embedding 服务部署指南

本指南说明如何为本项目配置嵌入向量服务。支持两种方案：
1. **阿里云百炼托管 API**—— 无需 GPU，按量付费，即开即用
2. **vllm-windows 本地部署** —— 自持 GPU，本地运行

---

## 方案一：阿里云百炼托管 API

阿里云百炼提供托管的多模态向量化 API，无需自行部署 GPU 服务器，按量付费。
特别适合没有 GPU 硬件或不想折腾环境搭建的 Windows 用户。

### 支持的模型

| 模型 | 向量类型 | 向量维度（可选值） | 图片格式 | 单图上限 | 文本上限 | 融合向量 |
|------|---------|------------------|---------|---------|---------|---------|
| `qwen3-vl-embedding` | 独立 / 融合 | 2560/2048/1536/1024/768/512/256（默认2560） | JPEG/PNG/WEBP/BMP/TIFF/ICO/DIB/ICNS/SGI | 5 MB | 32K Token | `enable_fusion=true` |
| `qwen2.5-vl-embedding` | 仅融合 | 2048/1024/768/512（默认1024） | JPEG/PNG/WEBP/BMP/TIFF/ICO/DIB/ICNS/SGI | 5 MB | 32K Token | 始终融合 |
| `tongyi-embedding-vision-plus-2026-03-06` | 独立 / 融合 | 1152/1024/512/256/128/64（默认1152） | 同上 | 5 MB（建议） | 1024 Token | 同对象融合 |
| `tongyi-embedding-vision-flash-2026-03-06` | 独立 / 融合 | 768/512/256/128/64（默认768） | 同上 | 5 MB（建议） | 1024 Token | 同对象融合 |
| `tongyi-embedding-vision-plus` | 仅独立 | 1152（固定） | JPG/PNG/BMP | 3 MB | 1024 Token | ❌ |
| `tongyi-embedding-vision-flash` | 仅独立 | 768（固定） | 同上 | 3 MB | 1024 Token | ❌ |
| `multimodal-embedding-v1` | 独立 | 1024（固定） | JPG/PNG/BMP | 3 MB | 512 Token | ❌ |

> **推荐**：使用 `qwen3-vl-embedding`。

### 1. 获取 API Key

1. 访问 [阿里云百炼控制台](https://bailian.console.aliyun.com/) 注册并登录（需实名认证）
2. 进入 **密钥管理** 页面，创建 API Key（格式如 `sk-xxx`）
3. 保存 API Key，注意北京地域与新加坡地域密钥不互通

### 3. 配置到应用

在应用设置页面：

- **API 类型**: 选择「阿里云百炼 DashScope」
- **Endpoint**: 留空（使用默认值 `https://dashscope.aliyuncs.com`）
- **API Key**: 填写上一步获取的 `sk-xxx`
- **模型名称**: 填写 `qwen3-vl-embedding`（推荐）

> 请求格式、参数说明、响应格式等协议细节请参见 [API 协议说明](api-protocol.md#dashscope-多模态-embedding-协议)。

### 4. 常见问题

**Q: 图片大小有限制吗？**

A: `qwen3-vl-embedding` 单张图片不超过 **5 MB**；2026-03-06 系列建议不超过 **5 MB**，最大 **10 MB**。
本项目会自动将超限图片压缩为 WebP 后发送。

**Q: 支持哪些图片格式？**

A: JPEG、PNG、WEBP、BMP、TIFF、ICO、DIB、ICNS、SGI。本项目支持通过文件扩展名自动推断 MIME 类型，
HEIC/HEIF 格式的图片会自动转为 WebP 再发送。详细格式说明见 [API 协议说明](api-protocol.md#支持的图片格式)。

## 方案二：vllm-windows 原生部署（本机部署）

> 适合 Windows 上拥有 NVIDIA 显卡的用户，直接在 Windows 系统中运行 vLLM，无需虚拟机或 Docker。
> 使用预编译 wheel，**无需从源码编译**。

项目地址：
- [SystemPanic/vllm-windows](https://github.com/SystemPanic/vllm-windows)（上游，提供 pre-built wheel）
- [ZHider/vllm-CUDA13-win_amd64-whls](https://gitee.com/zhider/vllm-cuda13-win_amd64-whls)（国内镜像，含完整依赖清单）

### 1. 环境要求

- Windows 10/11（64 位）
- NVIDIA 显卡，显存 >= 8GB
- [NVIDIA 驱动程序](https://www.nvidia.com/Download/index.aspx) >= 550（需支持 CUDA 13）
- [CUDA Toolkit](https://developer.nvidia.com/cuda-downloads) >= 13.0
- [Python](https://www.python.org/downloads/) = 3.12（必须是 3.12，wheel 仅支持此版本）
- 磁盘空间 >= 20GB

> 无需安装 Visual Studio Build Tools，无需从源码编译。

### 2. 安装 CUDA 13

```powershell
# 1. 下载 CUDA Toolkit 13.x
#    访问 https://developer.nvidia.com/cuda-downloads
#    选择 Windows → exe (local)

# 2. 安装完成后验证：
nvcc --version
# 输出应显示 Cuda compilation tools, release 13.x
```

### 3. 安装 uv（Python 包管理器）

```powershell
powershell -c "irm https://astral.sh/uv/install.ps1 | iex"
# 完成后重启终端，验证：
uv --version
```

> 也可以使用 `pip` + `venv`，但 `uv` 速度更快，且是 vLLM 官方推荐。

### 4. 创建环境并安装 vllm-windows

```powershell
# 创建 Python 3.12 虚拟环境
uv venv --python 3.12 vllm-env

# 激活虚拟环境
.\vllm-env\Scripts\Activate.ps1

# 使用国内镜像的完整依赖清单安装（含 vllm 自身）
uv pip install -r https://gitee.com/zhider/vllm-cuda13-win_amd64-whls/raw/master/requirements.txt
```

> 如果 GitHub 镜像（gh-proxy.org）下载缓慢，可手动下载以下 wheel 放入本地目录后再执行 `pip install`：
> - [vllm-0.21.0+cu132-cp312-cp312-win_amd64.whl](https://github.com/SystemPanic/vllm-windows/releases/download/v0.21.0/vllm-0.21.0+cu132-cp312-cp312-win_amd64.whl)
> 直接安装此 whl 会导致所有依赖直接安装。
> pytorch 国内建议使用 南京大学镜像 <https://mirrors.nju.edu.cn/pytorch/whl/cu130> 下载。

### 5. 启动服务

```powershell
# 确保虚拟环境已激活，然后启动：
vllm serve LifetimeMistake/Qwen3-VL-Embedding-2B-AWQ-4bit ^
  --task embedding ^
  --max-model-len 8192 ^
  --gpu-memory-utilization 0.6 ^
  --dtype auto ^
  --host 0.0.0.0 ^
  --port 8000
```

> 首次启动会自动下载模型，请保持网络畅通。模型下载到 `%USERPROFILE%\.cache\huggingface\` 目录。

### Qwen3-VL-Embedding-2B-AWQ-4bit （推荐）部署说明

该模型是 [Qwen3-VL-Embedding-2B](https://huggingface.co/Qwen/Qwen3-VL-Embedding-2B) 的 4bit AWQ 量化版，
使用 `compressed-tensors` 后端，支持文本+图片多模态嵌入。

#### 下载模型

```powershell
# 安装 git-lfs（首次使用）
git lfs install

# 使用hf download下载模型
hf download LifetimeMistake/Qwen3-VL-Embedding-2B-AWQ-4bit

# 或者克隆模型到本地（国内用户替换为 hf-mirror.com）
git clone https://huggingface.co/LifetimeMistake/Qwen3-VL-Embedding-2B-AWQ-4bit

```

#### 启动服务（实测参数）

```powershell
# 在模型所在目录执行
uv run vllm serve LifetimeMistake/Qwen3-VL-Embedding-2B-AWQ-4bit ^
  --runner pooling ^
  --convert embed ^
  --trust-remote-code ^
  --quantization compressed-tensors ^
  --limit-mm-per-prompt "{""image"":1}" ^
  --gpu-memory-utilization 0.6 ^
  --port 58880 ^
  --max-model-len 8192 ^
  --kv-cache-dtype fp8 ^
  --max-num-seqs 512 ^
  --max-num-batched-tokens 32768
```

参数说明：

| 参数 | 值 | 说明 |
|------|-----|------|
| `--runner pooling` | — | Embedding 模式必需 |
| `--convert embed` | — | 将模型转换为 Embedding 格式 |
| `--quantization` | `compressed-tensors` | AWQ 4bit 量化后端 |
| `--limit-mm-per-prompt` | `{"image":1}` | 每请求最多处理 1 张图片 |
| `--gpu-memory-utilization` | `0.6` | 显存利用率，量化版可以放低以留出余量 |
| `--kv-cache-dtype` | `fp8` | KV Cache 精度，节省显存 |
| `--max-num-seqs` | `512` | 最大并发序列数 |
| `--max-num-batched-tokens` | `32768` | 单 batch 最大 Token 数 |
| `--max-model-len` | `8192` | 最大上下文长度 |
| `--port` | `58880` | 服务端口 |

#### 配置到应用

在应用设置页面：
- **Endpoint**: `http://localhost:58880`
- **模型名称**: `LifetimeMistake/Qwen3-VL-Embedding-2B-AWQ-4bit`
- **API Key**: 留空

## 常见问题

### CUDA out of memory

- 降低 `--gpu-memory-utilization`（如改为 0.6）
- 减少 `--max-model-len`（如改为 4096）

### 请求返回 404

确认请求路径为 `/v1/embeddings`（不是 `/embeddings`），且模型名与启动时一致。

### pip install 失败（网络问题）

`requirements.txt` 中的依赖已配置国内镜像（南大、gh-proxy），但如果某个文件仍下载失败，可尝试：

```powershell
# 手动下载 wheel 后本地安装
pip install .\下载的.whl
```

### 模型下载慢

HuggingFace 模型下载可配置国内镜像：

```powershell
$env:HF_ENDPOINT = "https://hf-mirror.com"
```