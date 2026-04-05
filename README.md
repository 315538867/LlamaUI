# LlamaUI

llama.cpp 桌面管理工具 — 多模型并发部署、智能代理路由，为本地大模型提供可视化管理界面。

支持 Claude Code / Codex CLI 工具调用（MCP Server）。

## 功能

- **多模型部署** — 同时运行多个 llama.cpp 实例，每个实例独立端口
- **智能代理路由** — 单一代理入口，按 Codex 请求中的 `model` 字段自动路由到对应实例
- **双层 API Key** — 代理层保护 (proxy_api_key) + 实例层直传 llama.cpp (--api-key)
- **LM Studio 风格 UI** — 左侧实例列表，右侧配置/日志面板
- **实时日志** — 按实例隔离的 stdout/stderr 流式输出 + 代理日志
- **模型管理** — 自动扫描 GGUF 文件，识别量化类型和文件大小
- **配置预设** — 保存/加载不同模型的启动参数组合
- **自动检测** — 自动发现已安装的 llama.cpp 路径
- **MCP Server** — stdio 模式，供 Claude Code / Codex 通过工具调用管理模型
- **高性能低占用** — Tauri 原生窗口，包体约 5MB，内存占用极低

## 技术栈

| 层级 | 技术 |
|------|------|
| 框架 | Tauri v2 |
| 前端 | Svelte 5 + TypeScript |
| 样式 | Tailwind CSS 4 |
| 后端 | Rust |
| 存储 | JSON (原子写入) |

## 代理架构

```
Codex CLI                    LlamaUI 代理              llama.cpp 实例
─────────────────────────────────────────────────────────────────────
POST /v1/responses           ┌─────────────┐           ┌──────────┐
  body: { model: "qwen" } ──►│ 认证检查      │──► qwen ──► 随机端口 │
                             │ 模型路由      │           └──────────┘
POST /v1/responses           │ 协议转换      │           ┌──────────┐
  body: { model: "llama" }──►│ (Responses → │──► llama──► 随机端口 │
                             │  Messages)   │           └──────────┘
                             └─────────────┘
```

- 代理在 app 启动时以空路由表启动
- 实例启动后自动注册到路由表（`name → port`）
- 模型不存在时返回 404 + 可用实例列表
- 未配置 proxy_api_key 时开放访问

## 环境要求

- **Node.js** 18+
- **pnpm** 8+
- **Rust** 1.77+
- **系统依赖** (见下方各平台说明)

### Windows 额外依赖

- [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
  - 安装时勾选「使用 C++ 的桌面开发」
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (Win10 1803+ 和 Win11 已预装)

### macOS 额外依赖

```bash
xcode-select --install
```

### Linux 额外依赖 (Ubuntu/Debian)

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

## 开发

```bash
# 安装依赖
pnpm install

# 开发模式 (热重载)
pnpm tauri dev

# 仅构建前端
pnpm build

# Rust 类型检查
cd src-tauri && cargo check
```

## 打包发布

### Windows 打包

1. 确保环境就绪:

```powershell
# 检查 Rust
rustc --version
cargo --version

# 检查 Node.js
node --version
pnpm --version
```

2. 安装依赖并构建:

```powershell
pnpm install
pnpm tauri build
```

3. 产出目录:

```
src-tauri/target/release/bundle/
├── msi/
│   └── LlamaUI_0.1.0_x64_en-US.msi    # MSI 安装包 (推荐)
└── nsis/
    └── LlamaUI_0.1.0_x64-setup.exe     # NSIS 安装包
```

> MSI 和 NSIS 二选一分发即可，MSI 更适合企业环境，NSIS 更适合个人用户。

### macOS 打包

```bash
pnpm install
pnpm tauri build
# 产出: src-tauri/target/release/bundle/dmg/LlamaUI_0.1.0_aarch64.dmg
```

### Linux 打包

```bash
pnpm install
pnpm tauri build
# 产出: src-tauri/target/release/bundle/deb/ 和 appimage/
```

### 自定义图标

替换 `src-tauri/icons/` 下的图标文件后重新打包。可使用 Tauri 内置工具从一张 1024x1024 PNG 生成全部尺寸:

```bash
pnpm tauri icon path/to/icon.png
```

## MCP Server (Claude Code / Codex 集成)

LlamaUI 内置 MCP Server，支持 Claude Code 和 Codex CLI 通过工具调用管理本地模型。

### 配置方法

在 `~/.claude/settings.json` 中添加:

```json
{
  "mcpServers": {
    "llamaui": {
      "command": "C:\\Program Files\\LlamaUI\\LlamaUI.exe",
      "args": ["mcp"]
    }
  }
}
```

### 可用工具

| 工具名 | 功能 |
|--------|------|
| `llamaui_start` | 启动模型实例 (支持指定名称、模型、模式、GPU 层数等) |
| `llamaui_stop` | 停止指定实例 |
| `llamaui_status` | 查询所有实例运行状态 |
| `llamaui_list_models` | 列出所有本地 GGUF 模型 |
| `llamaui_get_config` | 获取当前配置 |

## 项目结构

```
LlamaUI/
├── src/                          # Svelte 前端
│   ├── App.svelte                # 根组件 (路由)
│   └── lib/
│       ├── components/           # UI 组件
│       │   ├── ModelLibrary.svelte   # 多实例管理面板 (主页)
│       │   ├── ModelBrowser.svelte   # GGUF 文件扫描浏览
│       │   ├── ConfigEditor.svelte   # 设置 + 代理配置 + 客户端接入
│       │   ├── LogTerminal.svelte    # 日志终端 (接受 logs prop)
│       │   ├── Sidebar.svelte        # 导航栏
│       │   └── StatusBar.svelte      # 运行状态栏
│       ├── stores/               # Svelte 5 状态 ($state/$derived)
│       │   ├── process.svelte.ts     # 实例状态 (InstanceMap)
│       │   ├── proxy.svelte.ts       # 代理日志
│       │   └── config.svelte.ts      # AppConfig
│       ├── services/             # Tauri invoke 封装
│       │   └── tauri-bridge.ts
│       └── types/                # TypeScript 类型
│           └── index.ts
├── src-tauri/                    # Rust 后端
│   └── src/
│       ├── commands/             # Tauri 命令
│       │   ├── instance.rs       # 实例启停/预设 CRUD
│       │   ├── proxy.rs          # ProxyState + 路由注册/注销
│       │   ├── config.rs         # 全局配置 + 预设
│       │   └── models.rs         # 模型扫描
│       ├── services/             # 业务逻辑
│       │   ├── instance_registry.rs  # 多进程管理 (随机端口)
│       │   ├── model_scanner.rs      # GGUF 扫描与元数据解析
│       │   ├── config_store.rs       # JSON 持久化 (原子写入)
│       │   └── llama_detector.rs     # llama.cpp 安装检测
│       ├── proxy/                # 代理服务器 (axum)
│       │   ├── server.rs         # ProxyConfig + 路由器
│       │   ├── handler.rs        # 认证/路由/转发/日志
│       │   └── convert/          # Responses API → Messages API 转换
│       └── mcp/                  # MCP Server (stdio JSON-RPC)
└── .github/workflows/            # CI 构建 (可选)
```

## 事件系统

| 事件 | 方向 | Payload |
|------|------|---------|
| `llama://instances` | Rust → TS | `HashMap<name, InstanceInfo>` 完整快照 |
| `llama://log` | Rust → TS | `{ instance, stream, line }` |
| `proxy://log` | Rust → TS | `{ timestamp, level, message }` |

## 许可证

MIT
