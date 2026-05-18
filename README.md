# DeepLX Monitor

DeepLX 翻译代理与监控面板。作为客户端与 DeepLX API 之间的中间层，支持多上游负载均衡、翻译缓存、请求日志记录，并提供 Vue 3 可视化面板。

## 特性

- 多上游端点负载均衡（round-robin + 故障转移）
- 翻译结果缓存（LRU，可配置 TTL 和容量）
- 配置热加载（修改 config.toml 自动生效，无需重启）
- 请求日志与统计（字符数、语言对、延迟、成功率）
- 可视化面板（调用趋势、语言统计、错误率、活动热力图）
- 数据导出（CSV / JSON）
- 演示模式（一键生成假数据用于 UI 验收）

## 项目结构

```
src/
├── main.rs       # Axum 路由、SPA 静态资源、后台任务
├── proxy.rs      # /translate 翻译代理（负载均衡 + 缓存）
├── api.rs        # /api/* 监控与配置接口
├── db.rs         # SQLite 数据库操作
├── config.rs     # TOML 配置管理 + 热加载监听
├── state.rs      # 应用状态（Config + DB + Health + Cache + LB）
├── upstream.rs   # 多上游负载均衡与故障转移
├── cache.rs      # 翻译缓存（LRU + TTL）
└── utils.rs      # 工具函数

frontend/
├── src/
│   ├── pages/
│   │   └── Dashboard.vue   # 监控面板（含设置抽屉）
│   ├── stores/monitor.ts   # Pinia 状态管理
│   └── App.vue             # 根组件
└── vite.config.ts          # Vite 配置，含 API 代理
```

## 技术栈

- **后端**: Rust (Axum + Tokio), SQLite (rusqlite), rust-embed, moka (缓存), notify (文件监听)
- **前端**: Vue 3 + TypeScript + Vite, Pinia, Chart.js

## 快速开始

### 1. 配置

复制配置模板并填写上游 API 地址：

```bash
cp config.toml.example config.toml
```

编辑 `config.toml`，配置 DeepLX API 端点。支持多个端点实现负载均衡：

```toml
[[upstream.endpoints]]
name = "primary"
url = "https://api.deeplx.org/YOUR_KEY/translate"
api_key = ""

[[upstream.endpoints]]
name = "secondary"
url = "https://api2.deeplx.org/YOUR_KEY/translate"
api_key = ""
```

### 2. 构建前端

```bash
cd frontend
npm install
npm run build
```

前端会构建到 `../dist/`，后端在编译时会嵌入此目录。

### 3. 运行后端

```bash
cargo run
```

服务默认监听 `127.0.0.1:5555`（可在 config.toml 中修改）。

### 4. 开发模式

前端开发服务器支持热重载，并自动代理 API 请求到后端：

```bash
cd frontend
npm run dev
```

访问 `http://localhost:5173`，API 请求会代理到 `localhost:5555`。

## 配置说明

配置文件支持热加载——修改 `config.toml` 保存后约 1 秒自动生效（`proxy.host`/`proxy.port` 除外，需重启）。也可通过 Web UI 设置面板修改，会自动写回文件。

| 配置段 | 说明 |
|--------|------|
| `[upstream]` | 端点列表、最大失败次数、探活间隔 |
| `[proxy]` | 监听地址和端口（需重启生效） |
| `[monitor]` | 自动刷新间隔、最大日志条数 |
| `[health_check]` | 健康检查使用的源/目标语言 |
| `[cache]` | 缓存开关、TTL、最大条目数、内存限制 |
| `[demo]` | 演示模式开关和种子 |

## 演示模式

开启演示模式可自动生成假数据，方便 UI 验收：

```toml
[demo]
enabled = true
seed = 20260511
```

重启后服务会自动填充监控数据到 `deeplx-monitor.db`。

## API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/translate` | 翻译代理（负载均衡 + 缓存） |
| GET | `/api/stats` | 统计概览（总量、周期） |
| GET | `/api/chart` | 图表数据（按小时/按天） |
| GET | `/api/requests` | 请求日志分页列表 |
| GET | `/api/health` | 获取健康状态 |
| POST | `/api/health/check` | 手动触发健康检查 |
| GET | `/api/config` | 获取完整配置 |
| POST | `/api/config` | 更新配置（持久化到 config.toml） |
| GET | `/api/lang-stats` | 语言统计 |
| GET | `/api/lang-hourly-stats` | 语言小时统计 |
| GET | `/api/upstream/status` | 上游端点状态 |
| GET | `/api/cache/stats` | 缓存统计 |
| POST | `/api/cache/clear` | 清空缓存 |
| GET | `/api/cache/hits` | 缓存命中日志（最近 100 条） |
| GET | `/api/analytics/heatmap` | 活动热力图 |
| GET | `/api/analytics/error-trend` | 错误率趋势 |
| GET | `/api/export` | 数据导出（CSV/JSON） |

## Docker

```bash
docker compose up -d        # 构建并运行
docker compose build        # 仅重新构建
```

多阶段构建：Node.js 构建前端，Rust 编译后端。挂载 `config.toml` 作为配置卷。

## 构建发布版本

### Linux

```bash
cd frontend && npm install && npm run build && cd ..
cargo build --release
```

产物位于 `target/release/deeplx_monitor`。

### macOS

```bash
cd frontend && npm install && npm run build && cd ..
cargo build --release
```

产物位于 `target/release/deeplx_monitor`。

如需交叉编译 Apple Silicon (aarch64)：

```bash
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

### Windows

#### 从 Linux 交叉编译

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install gcc-mingw-w64-x86-64  # Debian/Ubuntu

cd frontend && npm install && npm run build && cd ..
cargo build --release --target x86_64-pc-windows-gnu
```

产物位于 `target/x86_64-pc-windows-gnu/release/deeplx_monitor.exe`。

#### 在 Windows 上原生构建

```powershell
cd frontend && npm install && npm run build && cd ..
cargo build --release
```

产物位于 `target\release\deeplx_monitor.exe`。

### 分发

打包二进制文件 + `config.toml.example`，用户复制为 `config.toml` 后填写即可运行。
