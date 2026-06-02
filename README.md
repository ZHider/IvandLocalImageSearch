# 🖼️ 魔王图片搜索

基于 AI 向量化技术的本地图片搜索引擎，支持语义搜索、EXIF 信息提取、智能缩略图生成等功能。使用 NeutralinoJS + Vue3 + Rust 构建的跨平台桌面应用。

## ✨ 核心特性

- 🔍 **语义搜索** - 使用 AI 向量嵌入技术，支持自然语言搜索图片
- 📁 **多文件夹索引** - 支持索引多个本地文件夹中的图片和文本文件
- 🖼️ **智能缩略图** - 自动生成缩略图，支持 HEIC/HEIF 格式转换
- 📊 **EXIF 信息提取** - 自动提取并显示图片的拍摄信息（相机、ISO、光圈等）
- 🗂️ **索引管理** - 查看、查询、删除已索引的文件记录
- 🎨 **现代化 UI** - 基于 Naive UI 的简洁美观界面
- 💾 **本地存储** - 所有数据存储在本地，保护隐私
- ⚡ **高性能** - Rust 后端提供强大的处理能力

## 🛠️ 技术栈

### 前端
- **框架**: Vue 3.5 + TypeScript 6.0
- **UI 库**: Naive UI 2.44
- **构建工具**: Vite 8.0
- **路由**: Vue Router 4.6
- **桌面框架**: NeutralinoJS 6.7

### 后端 (Rust)
- **语言**: Rust 2021 Edition
- **异步运行时**: Tokio
- **向量数据库**: LanceDB 0.30
- **元数据存储**: SQLite (rusqlite 0.34)
- **AI 嵌入**: async-openai 0.40 (支持 OpenAI/Ollama/llama.cpp)
- **图片处理**: image 0.25 + heic + webp
- **EXIF 提取**: nom-exif 3
- **文件哈希**: blake3
- **通信**: WebSocket (tokio-tungstenite)

## 📋 系统要求

- **Node.js**: >= 18.0
- **pnpm**: >= 8.0
- **Rust**: >= 1.75 (2021 Edition)
- **操作系统**: Windows 10/11, macOS, Linux

## 🚀 快速开始

### 1. 克隆项目

```bash
git clone https://github.com/your-username/LocalImageSearch.git
cd LocalImageSearch
```

### 2. 安装前端依赖

```bash
pnpm install
```

### 3. 编译 Rust 后端

```bash
cd rust-extension
cargo check
```

> 💡 **提示**: 首次编译可能需要较长时间下载依赖。使用 `cargo check` 可以快速验证代码是否正确。

### 4. 配置 Protobuf（仅 Windows）

如果编译时提示缺少 protoc，需要设置环境变量：

```powershell
# 在 rust-extension\build.bat 中修改路径
set PROTOC_INCLUDE=D:\your\path\to\protoc-include
set PROTOC=D:\your\path\to\protoc.exe
```

### 5. 启动开发模式

在项目根目录运行：

```bash
pnpm neu:dev
```

这将同时启动：
- Vite 开发服务器 (http://localhost:5173)
- NeutralinoJS 桌面应用窗口

应用启动后，窗口标题为"魔王图片搜索"。

## 📖 使用指南

### 首次使用

1. **配置 AI 服务**
   - 进入"设置"页面
   - 选择 AI 提供商（Ollama / OpenAI / llama.cpp / 自定义）
   - 填写 API 端点、密钥和模型名称
   - 点击"测试连接"验证配置

2. **添加搜索文件夹**
   - 进入"索引"页面
   - 点击"选择文件夹"添加要索引的目录
   - 支持添加多个文件夹

3. **构建索引**
   - 点击"开始索引"
   - 等待索引完成（会显示进度）
   - 索引完成后即可搜索

4. **开始搜索**
   - 进入"搜索"页面
   - 输入自然语言描述
   - 查看搜索结果和缩略图

### 支持的图片格式

- JPEG (.jpg, .jpeg)
- PNG (.png)
- GIF (.gif)
- BMP (.bmp)
- WebP (.webp)
- HEIC/HEIF (.heic, .heif) - 自动转换为 WebP

### 支持的文本格式

- 纯文本 (.txt)
- Markdown (.md)

## 🏗️ 项目架构

### 目录结构

```
LocalImageSearch/
├── rust-extension/              # Rust 后端扩展
│   ├── src/
│   │   ├── main.rs              # 入口点
│   │   ├── event_dispatcher.rs  # 事件分发器
│   │   ├── config.rs            # 配置管理
│   │   ├── constants.rs         # 全局常量
│   │   ├── embedding.rs         # AI 向量嵌入
│   │   ├── scanner.rs           # 文件扫描
│   │   ├── hasher.rs            # 文件哈希计算
│   │   ├── image_processing.rs  # 图片处理（缩略图、HEIC转换）
│   │   ├── metadata.rs          # 元数据存储（SQLite）
│   │   ├── vector_store.rs      # 向量存储（LanceDB）
│   │   ├── text_chunker.rs      # 文本分块
│   │   ├── file_utils.rs        # 文件工具
│   │   ├── ws_client.rs         # WebSocket 客户端
│   │   └── events/              # 事件处理器
│   │       ├── mod.rs           # 事件模块入口
│   │       ├── config_events.rs # 配置相关事件
│   │       ├── index_events.rs  # 索引构建事件
│   │       ├── search_events.rs # 搜索事件
│   │       ├── manage_events.rs # 索引管理事件
│   │       └── media_events.rs  # 媒体处理事件
│   ├── Cargo.toml               # Rust 依赖
│   └── build.bat                # Windows 构建脚本
├── src/                         # Vue3 前端
│   ├── main.ts                  # 入口文件
│   ├── App.vue                  # 根组件
│   ├── router/                  # 路由配置
│   ├── types/                   # TypeScript 类型定义
│   ├── pages/                   # 页面组件
│   │   ├── HomePage.vue         # 首页
│   │   ├── IndexPage.vue        # 索引页面
│   │   ├── SearchPage.vue       # 搜索页面
│   │   └── SettingsPage.vue     # 设置页面
│   ├── components/              # 可复用组件
│   │   ├── SearchBar.vue        # 搜索栏
│   │   ├── ResultCard.vue       # 结果卡片
│   │   ├── PreviewModal.vue     # 预览模态框
│   │   ├── IndexBuildForm.vue   # 索引构建表单
│   │   ├── IndexProgressPanel.vue # 索引进度面板
│   │   ├── IndexResultPanel.vue # 索引结果面板
│   │   ├── IndexManagePanel.vue # 索引管理面板
│   │   ├── FolderManager.vue    # 文件夹管理
│   │   ├── ApiSettingsForm.vue  # API 设置表单
│   │   └── ImageProcessingForm.vue # 图片处理设置
│   └── composables/             # 组合式函数
│       ├── useExtension.ts      # 扩展通信
│       ├── useAppConfig.ts      # 应用配置
│       ├── useIndex.ts          # 索引操作
│       ├── useIndexManage.ts    # 索引管理
│       ├── useApiTest.ts        # API 测试
│       ├── useSettings.ts       # 设置管理
│       └── useThumbnailManager.ts # 缩略图管理
├── neutralino.config.json       # NeutralinoJS 配置
├── package.json                 # 前端依赖
└── vite.config.ts               # Vite 配置
```

### 系统架构

```
┌─────────────────────────────────────────────────────────┐
│                    NeutralinoJS 主进程                  │
│  ┌───────────────────────────────────────────────────┐  │
│  │              Vue3 前端界面                        │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐            │  │
│  │  │ 首页    │  │ 索引页  │  │ 搜索页  │  ...       │  │
│  │  └─────────┘  └─────────┘  └─────────┘            │  │
│  └───────────────────────────────────────────────────┘  │
│                          ↕ WebSocket                    │
│  ┌───────────────────────────────────────────────────┐  │
│  │              Rust 扩展进程                        │  │
│  │  ┌─────────────────────────────────────────────┐  │  │
│  │  │           事件分发器                        │  │  │
│  │  └─────────────────────────────────────────────┘  │  │
│  │           ↕              ↕              ↕         │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐   │  │
│  │  │ 索引管道   │  │ 搜索管道   │  │ 媒体处理   │   │  │
│  │  └────────────┘  └────────────┘  └────────────┘   │  │
│  │           ↕              ↕              ↕         │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐   │  │
│  │  │ LanceDB    │  │ SQLite     │  │ 文件系统   │   │  │
│  │  │ (向量存储) │  │ (元数据)   │  │ (图片/文本)│   │  │
│  │  └────────────┘  └────────────┘  └────────────┘   │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 数据流

#### 索引流程

```
用户点击"开始索引"
    ↓
前端发送 startIndex 事件
    ↓
后端接收 → 扫描文件夹
    ↓
阶段 1: 扫描文件 → 发送 indexProgress (scanning)
    ↓
阶段 2: 计算哈希 → 对比已有索引
    ↓
阶段 3: 处理新/修改文件
    ├── 图片: 提取 EXIF → 生成缩略图 → 计算向量嵌入
    └── 文本: 分块处理 → 计算向量嵌入
    ↓
阶段 4: 更新数据库
    ├── LanceDB: 更新向量索引
    └── SQLite: 更新元数据
    ↓
阶段 5: 清理已删除文件
    ↓
发送 indexComplete 事件 → 前端显示结果
```

#### 搜索流程

```
用户输入搜索词
    ↓
前端发送 search 事件
    ↓
后端接收 → 调用 AI API 生成查询向量
    ↓
阶段 1: 向量搜索 → LanceDB 检索相似向量
    ↓
阶段 2: 获取元数据 → SQLite 查询文件信息
    ↓
阶段 3: 生成缩略图 → 异步生成缩略图
    ↓
发送 searchResult 事件 → 前端显示结果
    ↓
缩略图就绪 → 发送 thumbnailReady 事件 → 前端更新图片
```

### 数据库设计

#### SQLite 元数据表

```
image_metadata/
├── file_path (TEXT, PRIMARY KEY)    # 文件绝对路径
├── file_size (INTEGER)              # 文件大小（字节）
├── file_hash (TEXT)                 # BLAKE3 哈希值
├── modified_at (TEXT)               # 最后修改时间
├── indexed_at (TEXT)                # 索引时间
└── file_type (TEXT)                 # 文件类型 (image/text)
```

#### LanceDB 向量表

```
vector_store/
├── id (INT, PRIMARY KEY)            # 自增 ID
├── file_path (TEXT)                 # 文件路径
├── vector (FixedSizeList<f32>)      # 向量嵌入
├── text_preview (TEXT)              # 文本预览（文本文件）
└── exif (JSON)                      # EXIF 信息（图片文件）
```

## 🔌 可用脚本

| 命令 | 说明 |
|------|------|
| `pnpm dev` | 启动 Vite 开发服务器（仅前端） |
| `pnpm build` | 构建前端生产版本 |
| `pnpm preview` | 预览生产构建 |
| `pnpm neu:dev` | 启动 NeutralinoJS 开发模式（推荐） |
| `pnpm neu:build` | 构建 NeutralinoJS 应用（debug 版本） |
| `pnpm neu:release` | 构建 NeutralinoJS 发布版本 |
| `cargo check` | 检查 Rust 代码（不生成二进制） |
| `cargo build` | 编译 Rust 扩展（debug 版本） |
| `cargo build --release` | 编译 Rust 扩展（release 版本） |

## ⚙️ 环境变量

### 前端环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `VITE_APP_TITLE` | 应用标题 | 魔王图片搜索 |

### NeutralinoJS 运行时变量

应用运行时由 NeutralinoJS 注入的全局变量：

| 变量 | 说明 |
|------|------|
| `NL_PORT` | NeutralinoJS 服务端口 |
| `NL_TOKEN` | 认证令牌 |
| `NL_CONNECT_TOKEN` | WebSocket 连接令牌 |
| `NL_EXTENSION_ID` | 扩展 ID |
| `NL_PATH` | 应用安装路径 |
| `NL_CWD` | 当前工作目录 |
| `NL_PID` | 进程 ID |

### Rust 后端配置

后端通过 `config.json` 管理配置（存储在 `data/` 目录）：

| 字段 | 类型 | 说明 |
|------|------|------|
| `apiType` | string | AI 提供商: `ollama` / `openai` / `llamacpp` / `custom` |
| `endpoint` | string | API 端点 URL |
| `apiKey` | string \| null | API 密钥（可选） |
| `modelName` | string | 嵌入模型名称 |
| `folders` | string[] | 要索引的文件夹路径列表 |
| `customProviderName` | string \| null | 自定义提供商名称 |
| `customEmbeddingPath` | string | 自定义嵌入 API 路径 |
| `imageProcessing.embedImageSize` | number | 嵌入前图片缩放尺寸 |
| `imageProcessing.thumbnailSize` | number | 缩略图尺寸 |

## 🧪 测试

### 前端测试

```bash
# 运行类型检查
pnpm build
```

### 后端测试

```bash
# 检查代码质量
cd rust-extension
cargo check
```

## 📦 构建与部署

### 开发模式

```bash
pnpm neu:dev
```

### 生产构建

```bash
# 1. 编译 Rust 后端（release 版本）
cd rust-extension
cargo build --release
cp target/release/ai-search-extension.exe ../extensions/

# 2. 构建前端
cd ..
pnpm build

# 3. 打包 NeutralinoJS 应用
pnpm neu:release
```

### 发布产物

构建完成后，发布文件位于：

```
output/
└── localimagesearch/
    ├── localimagesearch.exe      # Windows 可执行文件
    ├── resources/
    │   ├── neutralino.config.json
    │   └── ...
    └── extensions/
        └── ai-search-extension.exe
```

## 🔧 故障排除

### 常见问题

#### 1. Rust 编译失败

```bash
# 确保 Rust 版本 >= 1.75
rustc --version

# 清理并重新编译
cd rust-extension
cargo clean
cargo build
```

#### 2. Protobuf 相关错误

Windows 用户需要配置 protoc 路径：

```powershell
# 编辑 build.bat
set PROTOC_INCLUDE=D:\your\path\to\protoc-include
set PROTOC=D:\your\path\to\protoc.exe
```

#### 3. NeutralinoJS 无法启动

```bash
# 确保已安装依赖
pnpm install

# 检查 NeutralinoJS CLI 版本
pnpm neu --version

# 重新初始化
rm -rf .neutralino
pnpm neu:dev
```

#### 4. 前端端口冲突

如果 5173 端口被占用，Vite 会自动选择其他端口。确保 NeutralinoJS 配置中的 `frontendLibrary.devUrl` 与实际端口一致。

#### 5. AI API 连接失败

- 检查网络连接
- 验证 API 端点和密钥
- 确认模型名称正确
- 使用"测试连接"功能验证配置

## 📝 开发指南

### 添加新功能

#### 前端组件

```bash
# 1. 在 src/components/ 创建新组件
# 2. 在 src/composables/ 创建业务逻辑
# 3. 在 src/types/index.ts 添加类型定义
# 4. 在路由中添加新页面（如需要）
```

#### 后端事件处理器

```bash
# 1. 在 rust-extension/src/events/ 创建新事件文件
# 2. 在 mod.rs 中导出新 handler
# 3. 在 event_dispatcher.rs 中注册事件
# 4. 使用 send_broadcast 发送响应
```

### 代码规范

- **前端**: 使用 TypeScript 严格模式，遵循 Vue 3 组合式 API 规范
- **后端**: 遵循 Rust 官方规范，使用 `cargo fmt` 格式化代码
- **提交信息**: 遵循约定式提交规范（Conventional Commits）

## 📄 许可证

本项目采用 MIT 许可证。

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

1. Fork 本项目
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'feat: add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 提交 Pull Request

## 📞 联系方式

如有问题或建议，请提交 Issue 或联系项目维护者。
