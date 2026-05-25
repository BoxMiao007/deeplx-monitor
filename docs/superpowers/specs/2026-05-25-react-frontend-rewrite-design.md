# DeepLX Monitor 前端 React 重构设计

## 概述

将 deeplx-monitor 前端从 Vue 3 完全重构为 React 19，UI 风格照搬 cpa-usage-keeper 项目的暖色调纸质风格。后端 API 不变，仅替换前端消费者。

## 技术栈

| 层 | 选型 |
|---|------|
| 框架 | React 19 + TypeScript |
| 构建 | Vite |
| 状态管理 | Zustand |
| 样式 | SCSS Modules（无第三方 UI 库） |
| 图表 | Chart.js（主趋势图、错误趋势图）+ Recharts（迷你图、热力图） |
| 国际化 | react-i18next（中英双语） |
| 主题 | 暖纸色（默认）+ 暗色 |

## 实现方案

原地重写：在现有 `frontend/` 目录下删除 Vue 文件，用 React 项目结构替代。构建输出路径不变（`../dist/`），Vite proxy 配置不变，后端零改动。

## Tab 结构

**概览 / 分析 / 端点 / 日志 / 设置**（5 Tab）

## 项目结构

```
frontend/
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
├── src/
│   ├── main.tsx
│   ├── App.tsx                     # ThemeProvider + i18n + Tab Router
│   ├── i18n/
│   │   ├── index.ts
│   │   ├── zh.json
│   │   └── en.json
│   ├── styles/
│   │   ├── _variables.scss         # CSS 变量（双主题）
│   │   ├── _mixins.scss
│   │   └── global.scss
│   ├── components/
│   │   ├── ui/                     # 基础 UI 组件
│   │   │   ├── Button/
│   │   │   ├── Card/
│   │   │   ├── Input/
│   │   │   ├── Select/
│   │   │   ├── Modal/
│   │   │   ├── LoadingSpinner/
│   │   │   └── EmptyState/
│   │   ├── layout/
│   │   │   ├── TopBar.tsx          # 毛玻璃胶囊顶栏
│   │   │   ├── TopBar.module.scss
│   │   │   └── PageShell.tsx       # 页面容器
│   │   ├── charts/
│   │   │   ├── MiniChart.tsx       # 迷你折线图（Recharts）
│   │   │   ├── TrendChart.tsx      # 主趋势图（Chart.js）
│   │   │   ├── Heatmap.tsx         # 热力图（Recharts）
│   │   │   └── ErrorTrend.tsx      # 错误趋势图（Chart.js）
│   │   └── shared/
│   │       ├── StatCard.tsx
│   │       ├── ThemeSwitcher.tsx
│   │       └── LanguageSwitcher.tsx
│   ├── pages/
│   │   ├── Overview.tsx
│   │   ├── Analysis.tsx
│   │   ├── Endpoints.tsx
│   │   ├── Logs.tsx
│   │   └── Settings.tsx
│   ├── stores/
│   │   ├── useStatsStore.ts
│   │   ├── useConfigStore.ts
│   │   ├── useThemeStore.ts
│   │   └── useEndpointStore.ts
│   ├── hooks/
│   │   ├── useAutoRefresh.ts
│   │   └── useApi.ts
│   └── types/
│       └── index.ts
```

## 视觉系统

### 配色方案

| 变量 | 暖纸色（默认） | 暗色 |
|------|---------------|------|
| `--bg-page` | `#faf9f5` | `#151412` |
| `--bg-primary` | `#f0eee8` | `#1d1b18` |
| `--bg-hover` | `#e9e6df` | `#2a2724` |
| `--text-primary` | `#2d2a26` | `#f6f4f1` |
| `--text-secondary` | `#6d6760` | `#a8a29e` |
| `--border-color` | `#e3e1db` | `#3a3530` |
| `--color-primary` | `#8b8680` | `#a8a29e` |
| `--color-success` | `#10b981` | `#34d399` |
| `--color-error` | `#c65746` | `#ef7564` |

### 主题切换

- `useThemeStore` 持久化到 `localStorage`
- `<html data-theme="light|dark">` 切换 CSS 变量
- `:root[data-theme="light"]` / `:root[data-theme="dark"]` 定义变量

### 核心视觉元素

- **顶栏**：`backdrop-filter: blur(18px)`，半透明背景，`border-radius: 24px`（胶囊形），sticky
- **卡片**：`border-radius: 12px`，`1px solid var(--border-color)`，`box-shadow: 0 1px 2px rgb(0 0 0 / 0.08)`
- **页面背景**：径向渐变叠加
- **字体**：`'SF Pro Text', 'Segoe UI', system-ui, sans-serif`
- **动效**：hover 150ms ease，modal scale-in

## 页面设计

### 概览 (Overview)

- 统计卡片网格（4-5 张）：总调用次数、总字符数、成功率、缓存命中率、平均延迟
  - 每张含：标签、大数值、迷你折线图、趋势标注
- 主趋势图（Chart.js）：请求量随时间变化，范围切换（今天/7天/30天/90天/全部）
- 端点健康摘要：状态指示条，各上游在线/离线状态

### 分析 (Analysis)

- 热力图：按小时/天展示请求分布（Recharts）
- 错误趋势图：错误率随时间变化（Chart.js）
- 语言统计：源/目标语言分布
- 端点对比：多端点延迟/成功率对比

### 端点 (Endpoints)

- 端点卡片列表：URL、状态徽章、延迟、成功率
- 端点详情（展开或 modal）：详细统计、最近请求、健康检查按钮
- 缓存管理：缓存统计、命中日志、清除按钮

### 日志 (Logs)

- 请求日志表格：时间、源语言、目标语言、端点、延迟、状态
- 筛选栏：按状态、端点、语言筛选
- 分页

### 设置 (Settings)

- 上游端点管理：添加/编辑/删除
- 代理设置：host、port（标注需重启）
- 监控设置：日志保留天数、清理间隔
- 缓存设置：启用/禁用、TTL、最大容量
- 健康检查设置：间隔、超时
- 数据导出按钮

## 顶栏布局

```
┌─────────────────────────────────────────────────────────────────┐
│  DeepLX 监控    [概览] [分析] [端点] [日志] [设置]       🌙 🌐  │
└─────────────────────────────────────────────────────────────────┘
```

- 左侧：Logo + 品牌名
- 中间：Tab 导航
- 右侧：主题切换 + 语言切换

## 全局交互

- 范围选择器和自动刷新控件放在概览页顶部（非顶栏），分析页复用同一状态
- Tab 切换无路由跳转，URL hash 同步（`#overview`、`#analysis` 等）
- Toast 通知从右侧滑入，3s 自动消失
- 危险操作弹出确认 Modal
- 加载状态使用骨架屏

## 数据流

### API 请求

- 基于 `fetch` 的轻量封装（`useApi.ts`），不引入 axios
- 统一错误处理：网络错误 → toast，HTTP 错误 → 解析 message
- 开发环境 Vite proxy，生产环境同源

### Store 分工

| Store | 职责 | API |
|-------|------|-----|
| `useStatsStore` | 统计、图表、自动刷新 | `/api/stats`, `/api/chart`, `/api/heatmap`, `/api/error-trend`, `/api/lang-stats` |
| `useEndpointStore` | 端点、健康、缓存 | `/api/upstream-status`, `/api/health`, `/api/cache-stats`, `/api/cache-hitlogs` |
| `useLogsStore` | 日志、筛选、分页 | `/api/requests` |
| `useConfigStore` | 配置读写 | `GET/POST /api/config` |
| `useThemeStore` | 主题 | 纯 `localStorage` |

### 自动刷新

- `useAutoRefresh` hook：`setInterval` + `useEffect` 清理
- 页面不可见时暂停（`visibilitychange`）
- 概览和分析页共享刷新状态

## 构建与部署

- 输出目录：`../dist/`（与当前一致）
- Vite proxy：`/api/*` 和 `/translate` → `localhost:55551`
- Docker 多阶段构建：Node 阶段改为 React 构建命令
- `npm run build` 命令不变
