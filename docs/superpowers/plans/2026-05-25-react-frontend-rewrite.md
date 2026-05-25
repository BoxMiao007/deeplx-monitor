# React Frontend Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Vue 3 frontend with a React 19 app that copies cpa-usage-keeper's warm paper visual style, reorganized into 5 tabs (Overview/Analysis/Endpoints/Logs/Settings).

**Architecture:** Single-page React app with Zustand stores per domain, SCSS Modules for styling with CSS custom properties for theming, Chart.js + Recharts for charts, react-i18next for i18n. No router library — tab state managed via hash and local state.

**Tech Stack:** React 19, TypeScript, Vite, Zustand, SCSS Modules, Chart.js, Recharts, react-i18next

---

## File Map

| Action | Path | Responsibility |
|--------|------|---------------|
| Delete | `frontend/src/**/*.vue`, `frontend/src/router.ts`, `frontend/src/shims-vue.d.ts`, `frontend/src/stores/monitor.ts`, `frontend/src/style.css` | Remove all Vue code |
| Create | `frontend/src/main.tsx` | React entry point |
| Create | `frontend/src/App.tsx` | Root: theme provider + tab routing |
| Create | `frontend/src/types/index.ts` | All shared TypeScript interfaces |
| Create | `frontend/src/i18n/index.ts`, `zh.json`, `en.json` | i18n config and translations |
| Create | `frontend/src/styles/_variables.scss` | CSS custom properties (light + dark) |
| Create | `frontend/src/styles/_mixins.scss` | Reusable SCSS mixins |
| Create | `frontend/src/styles/global.scss` | Reset, fonts, scrollbar, base styles |
| Create | `frontend/src/stores/useThemeStore.ts` | Theme state + localStorage persistence |
| Create | `frontend/src/stores/useStatsStore.ts` | Stats, chart data, auto-refresh |
| Create | `frontend/src/stores/useEndpointStore.ts` | Endpoints, health, cache |
| Create | `frontend/src/stores/useLogsStore.ts` | Request logs, filters, pagination |
| Create | `frontend/src/stores/useConfigStore.ts` | Full config CRUD |
| Create | `frontend/src/hooks/useApi.ts` | Fetch wrapper with error handling |
| Create | `frontend/src/hooks/useAutoRefresh.ts` | Interval + visibility pause |
| Create | `frontend/src/components/ui/Button/` | Button component + styles |
| Create | `frontend/src/components/ui/Card/` | Card component + styles |
| Create | `frontend/src/components/ui/Input/` | Input component + styles |
| Create | `frontend/src/components/ui/Select/` | Select component + styles |
| Create | `frontend/src/components/ui/Modal/` | Modal component + styles |
| Create | `frontend/src/components/ui/LoadingSpinner/` | Spinner component |
| Create | `frontend/src/components/ui/EmptyState/` | Empty state component |
| Create | `frontend/src/components/layout/TopBar.tsx` | Frosted glass pill nav bar |
| Create | `frontend/src/components/layout/PageShell.tsx` | Page container with gradient bg |
| Create | `frontend/src/components/shared/StatCard.tsx` | Stat card with mini chart |
| Create | `frontend/src/components/shared/ThemeSwitcher.tsx` | Light/dark toggle |
| Create | `frontend/src/components/shared/LanguageSwitcher.tsx` | zh/en toggle |
| Create | `frontend/src/components/shared/Toast.tsx` | Toast notification system |
| Create | `frontend/src/components/charts/MiniChart.tsx` | Sparkline (Recharts) |
| Create | `frontend/src/components/charts/TrendChart.tsx` | Main line chart (Chart.js) |
| Create | `frontend/src/components/charts/Heatmap.tsx` | Heatmap grid (Recharts) |
| Create | `frontend/src/components/charts/ErrorTrend.tsx` | Error trend line (Chart.js) |
| Create | `frontend/src/pages/Overview.tsx` | Overview tab page |
| Create | `frontend/src/pages/Analysis.tsx` | Analysis tab page |
| Create | `frontend/src/pages/Endpoints.tsx` | Endpoints tab page |
| Create | `frontend/src/pages/Logs.tsx` | Logs tab page |
| Create | `frontend/src/pages/Settings.tsx` | Settings tab page |
| Modify | `frontend/index.html` | Change entry to main.tsx |
| Rewrite | `frontend/package.json` | React deps, remove Vue deps |
| Rewrite | `frontend/tsconfig.json` | React JSX config |
| Rewrite | `frontend/vite.config.ts` | React plugin, keep proxy + output |

---

## Task 1: Project Scaffolding

**Files:**
- Delete: all `*.vue` files, `router.ts`, `shims-vue.d.ts`, `stores/monitor.ts`, `style.css`
- Rewrite: `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`
- Create: `src/main.tsx`, `src/App.tsx`

- [ ] **Step 1: Remove Vue source files**

```bash
cd frontend
rm -f src/App.vue src/router.ts src/shims-vue.d.ts src/style.css src/main.ts
rm -f src/stores/monitor.ts
rm -rf src/pages src/components
rm -f package-lock.json
```

- [ ] **Step 2: Rewrite package.json**

```json
{
  "name": "deeplx-monitor-frontend",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "chart.js": "^4.4.0",
    "i18next": "^24.0.0",
    "react": "^19.0.0",
    "react-chartjs-2": "^5.2.0",
    "react-dom": "^19.0.0",
    "react-i18next": "^15.0.0",
    "recharts": "^2.15.0",
    "zustand": "^5.0.0"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "sass": "^1.80.0",
    "typescript": "^5.7.0",
    "vite": "^6.2.0"
  }
}
```

- [ ] **Step 3: Rewrite vite.config.ts**

```typescript
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { resolve } from 'path'

export default defineConfig({
  plugins: [react()],
  root: '.',
  base: '/',
  build: {
    outDir: '../dist',
    emptyOutDir: true,
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:5555',
        changeOrigin: true,
      },
      '/translate': {
        target: 'http://localhost:5555',
        changeOrigin: true,
      },
    },
  },
})
```

- [ ] **Step 4: Rewrite tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "forceConsistentCasingInFileNames": true,
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  },
  "include": ["src"]
}
```

- [ ] **Step 5: Update index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/favicon.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>DeepLX Monitor</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 6: Create src/main.tsx**

```tsx
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import App from './App'
import './styles/global.scss'
import './i18n'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>
)
```

- [ ] **Step 7: Create src/App.tsx (minimal placeholder)**

```tsx
function App() {
  return <div id="app">DeepLX Monitor</div>
}

export default App
```

- [ ] **Step 8: Install dependencies and verify build**

```bash
cd frontend
npm install
npx tsc --noEmit
npm run build
```

Expected: Build succeeds, `../dist/index.html` exists.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat(frontend): scaffold React project, remove Vue"
```

---

## Task 2: Styles Foundation & Theme System

**Files:**
- Create: `src/styles/_variables.scss`, `src/styles/_mixins.scss`, `src/styles/global.scss`
- Create: `src/stores/useThemeStore.ts`

- [ ] **Step 1: Create _variables.scss**

```scss
:root[data-theme="light"] {
  --bg-page: #faf9f5;
  --bg-primary: #f0eee8;
  --bg-secondary: #f7f6f2;
  --bg-hover: #e9e6df;
  --bg-topbar: rgba(240, 238, 232, 0.78);
  --text-primary: #2d2a26;
  --text-secondary: #6d6760;
  --text-tertiary: #9c9590;
  --border-color: #e3e1db;
  --border-hover: #d4d1ca;
  --color-primary: #8b8680;
  --color-primary-hover: #7a756f;
  --color-success: #10b981;
  --color-success-bg: rgba(16, 185, 129, 0.1);
  --color-error: #c65746;
  --color-error-bg: rgba(198, 87, 70, 0.1);
  --color-warning: #d97706;
  --shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.05);
  --shadow-md: 0 1px 3px 0 rgb(0 0 0 / 0.08);
  --shadow-lg: 0 4px 12px 0 rgb(0 0 0 / 0.08);
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
  --radius-pill: 24px;
}

:root[data-theme="dark"] {
  --bg-page: #151412;
  --bg-primary: #1d1b18;
  --bg-secondary: #242220;
  --bg-hover: #2a2724;
  --bg-topbar: rgba(29, 27, 24, 0.78);
  --text-primary: #f6f4f1;
  --text-secondary: #a8a29e;
  --text-tertiary: #78716c;
  --border-color: #3a3530;
  --border-hover: #4a4540;
  --color-primary: #a8a29e;
  --color-primary-hover: #d6d3d1;
  --color-success: #34d399;
  --color-success-bg: rgba(52, 211, 153, 0.1);
  --color-error: #ef7564;
  --color-error-bg: rgba(239, 117, 100, 0.1);
  --color-warning: #fbbf24;
  --shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.2);
  --shadow-md: 0 1px 3px 0 rgb(0 0 0 / 0.3);
  --shadow-lg: 0 4px 12px 0 rgb(0 0 0 / 0.3);
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
  --radius-pill: 24px;
}
```

- [ ] **Step 2: Create _mixins.scss**

```scss
@mixin card {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
  padding: 24px;
}

@mixin glass {
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

@mixin transition($props: all) {
  transition: $props 150ms ease;
}

@mixin truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@mixin responsive($breakpoint: 768px) {
  @media (max-width: $breakpoint) {
    @content;
  }
}
```

- [ ] **Step 3: Create global.scss**

```scss
@use 'variables';
@use 'mixins';

*,
*::before,
*::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html {
  font-family: 'SF Pro Text', 'Segoe UI', system-ui, -apple-system, sans-serif;
  font-size: 16px;
  line-height: 1.5;
  color: var(--text-primary);
  background: var(--bg-page);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

body {
  min-height: 100vh;
  background: var(--bg-page);
}

#root {
  min-height: 100vh;
  position: relative;
}

::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--text-tertiary);
}

code, pre {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
}

button {
  cursor: pointer;
  border: none;
  background: none;
  font: inherit;
  color: inherit;
}

input, select, textarea {
  font: inherit;
  color: inherit;
}

a {
  color: var(--color-primary);
  text-decoration: none;
}
```

- [ ] **Step 4: Create useThemeStore.ts**

```typescript
import { create } from 'zustand'

type Theme = 'light' | 'dark'

interface ThemeState {
  theme: Theme
  setTheme: (theme: Theme) => void
  toggleTheme: () => void
}

const getInitialTheme = (): Theme => {
  const stored = localStorage.getItem('theme')
  if (stored === 'light' || stored === 'dark') return stored
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

export const useThemeStore = create<ThemeState>((set) => ({
  theme: getInitialTheme(),
  setTheme: (theme) => {
    document.documentElement.setAttribute('data-theme', theme)
    localStorage.setItem('theme', theme)
    set({ theme })
  },
  toggleTheme: () => {
    set((state) => {
      const next = state.theme === 'light' ? 'dark' : 'light'
      document.documentElement.setAttribute('data-theme', next)
      localStorage.setItem('theme', next)
      return { theme: next }
    })
  },
}))

// 初始化 DOM 属性
document.documentElement.setAttribute('data-theme', getInitialTheme())
```

- [ ] **Step 5: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

Expected: No type errors.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(frontend): add theme system and SCSS foundation"
```

---

## Task 3: i18n Setup

**Files:**
- Create: `src/i18n/index.ts`, `src/i18n/zh.json`, `src/i18n/en.json`

- [ ] **Step 1: Create src/i18n/index.ts**

```typescript
import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import zh from './zh.json'
import en from './en.json'

const savedLang = localStorage.getItem('language') || 'zh'

i18n.use(initReactI18next).init({
  resources: {
    zh: { translation: zh },
    en: { translation: en },
  },
  lng: savedLang,
  fallbackLng: 'zh',
  interpolation: { escapeValue: false },
})

export default i18n
```

- [ ] **Step 2: Create src/i18n/zh.json**

```json
{
  "brand": "DeepLX 监控",
  "tabs": {
    "overview": "概览",
    "analysis": "分析",
    "endpoints": "端点",
    "logs": "日志",
    "settings": "设置"
  },
  "overview": {
    "totalRequests": "总调用次数",
    "totalChars": "总字符数",
    "successRate": "成功率",
    "cacheHitRate": "缓存命中率",
    "avgLatency": "平均延迟",
    "cumulative": "累计总数",
    "trend": "趋势",
    "endpointHealth": "端点健康",
    "range": "范围",
    "today": "今天",
    "days7": "7 天",
    "days30": "30 天",
    "days90": "90 天",
    "all": "所有",
    "autoRefresh": "自动刷新",
    "off": "关闭",
    "seconds": "秒",
    "refresh": "刷新"
  },
  "analysis": {
    "heatmap": "请求热力图",
    "errorTrend": "错误趋势",
    "langStats": "语言统计",
    "endpointCompare": "端点对比",
    "weekday": "按星期",
    "hourly": "按小时"
  },
  "endpoints": {
    "title": "上游端点",
    "online": "在线",
    "offline": "离线",
    "latency": "延迟",
    "successRate": "成功率",
    "requests": "请求数",
    "healthCheck": "健康检查",
    "cache": "缓存",
    "cacheStats": "缓存统计",
    "cacheHits": "命中日志",
    "clearCache": "清除缓存",
    "size": "大小",
    "hitRate": "命中率",
    "enabled": "已启用",
    "disabled": "已禁用"
  },
  "logs": {
    "title": "请求日志",
    "time": "时间",
    "sourceLang": "源语言",
    "targetLang": "目标语言",
    "endpoint": "端点",
    "latency": "延迟",
    "status": "状态",
    "success": "成功",
    "failed": "失败",
    "filter": "筛选",
    "all": "全部",
    "noData": "暂无数据"
  },
  "settings": {
    "title": "设置",
    "upstream": "上游端点",
    "proxy": "代理设置",
    "monitor": "监控设置",
    "cacheConfig": "缓存设置",
    "healthCheck": "健康检查",
    "export": "数据导出",
    "name": "名称",
    "url": "地址",
    "apiKey": "API Key",
    "add": "添加",
    "delete": "删除",
    "save": "保存",
    "cancel": "取消",
    "host": "主机",
    "port": "端口",
    "restartRequired": "需要重启",
    "logRetention": "日志保留天数",
    "cleanupInterval": "清理间隔",
    "ttl": "TTL (秒)",
    "maxEntries": "最大条目数",
    "maxMemory": "最大内存 (MB)",
    "interval": "间隔",
    "timeout": "超时",
    "exportCsv": "导出 CSV",
    "saved": "保存成功",
    "confirmDelete": "确认删除？"
  },
  "common": {
    "confirm": "确认",
    "cancel": "取消",
    "loading": "加载中...",
    "error": "出错了",
    "retry": "重试",
    "ms": "ms",
    "empty": "暂无数据"
  }
}
```

- [ ] **Step 3: Create src/i18n/en.json**

```json
{
  "brand": "DeepLX Monitor",
  "tabs": {
    "overview": "Overview",
    "analysis": "Analysis",
    "endpoints": "Endpoints",
    "logs": "Logs",
    "settings": "Settings"
  },
  "overview": {
    "totalRequests": "Total Requests",
    "totalChars": "Total Characters",
    "successRate": "Success Rate",
    "cacheHitRate": "Cache Hit Rate",
    "avgLatency": "Avg Latency",
    "cumulative": "Cumulative",
    "trend": "Trend",
    "endpointHealth": "Endpoint Health",
    "range": "Range",
    "today": "Today",
    "days7": "7 Days",
    "days30": "30 Days",
    "days90": "90 Days",
    "all": "All",
    "autoRefresh": "Auto Refresh",
    "off": "Off",
    "seconds": "s",
    "refresh": "Refresh"
  },
  "analysis": {
    "heatmap": "Request Heatmap",
    "errorTrend": "Error Trend",
    "langStats": "Language Stats",
    "endpointCompare": "Endpoint Comparison",
    "weekday": "By Weekday",
    "hourly": "By Hour"
  },
  "endpoints": {
    "title": "Upstream Endpoints",
    "online": "Online",
    "offline": "Offline",
    "latency": "Latency",
    "successRate": "Success Rate",
    "requests": "Requests",
    "healthCheck": "Health Check",
    "cache": "Cache",
    "cacheStats": "Cache Stats",
    "cacheHits": "Hit Logs",
    "clearCache": "Clear Cache",
    "size": "Size",
    "hitRate": "Hit Rate",
    "enabled": "Enabled",
    "disabled": "Disabled"
  },
  "logs": {
    "title": "Request Logs",
    "time": "Time",
    "sourceLang": "Source",
    "targetLang": "Target",
    "endpoint": "Endpoint",
    "latency": "Latency",
    "status": "Status",
    "success": "Success",
    "failed": "Failed",
    "filter": "Filter",
    "all": "All",
    "noData": "No data"
  },
  "settings": {
    "title": "Settings",
    "upstream": "Upstream Endpoints",
    "proxy": "Proxy Settings",
    "monitor": "Monitor Settings",
    "cacheConfig": "Cache Settings",
    "healthCheck": "Health Check",
    "export": "Data Export",
    "name": "Name",
    "url": "URL",
    "apiKey": "API Key",
    "add": "Add",
    "delete": "Delete",
    "save": "Save",
    "cancel": "Cancel",
    "host": "Host",
    "port": "Port",
    "restartRequired": "Restart required",
    "logRetention": "Log Retention (days)",
    "cleanupInterval": "Cleanup Interval",
    "ttl": "TTL (seconds)",
    "maxEntries": "Max Entries",
    "maxMemory": "Max Memory (MB)",
    "interval": "Interval",
    "timeout": "Timeout",
    "exportCsv": "Export CSV",
    "saved": "Saved successfully",
    "confirmDelete": "Confirm delete?"
  },
  "common": {
    "confirm": "Confirm",
    "cancel": "Cancel",
    "loading": "Loading...",
    "error": "Something went wrong",
    "retry": "Retry",
    "ms": "ms",
    "empty": "No data"
  }
}
```

- [ ] **Step 4: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(frontend): add i18n with Chinese and English translations"
```

---

## Task 4: TypeScript Types & API Hook

**Files:**
- Create: `src/types/index.ts`, `src/hooks/useApi.ts`

- [ ] **Step 1: Create src/types/index.ts**

```typescript
export interface HealthStatus {
  status: string
  latency_ms: number | null
  checked_at: string | null
  error: string | null
}

export interface ConfigInfo {
  upstream_url: string
  api_key: string
  auto_refresh_seconds: number
}

export interface PeriodStats {
  today_requests: number
  today_chars: number
  week_requests: number
  week_chars: number
  month_requests: number
  month_chars: number
}

export interface StatsResponse {
  total_requests: number
  total_chars: number
  health: HealthStatus
  config: ConfigInfo
  period: PeriodStats
}

export interface RequestLog {
  id: number
  chars: number
  source_lang: string
  target_lang: string
  source_chars: number
  target_chars: number
  status: string
  error_msg: string | null
  created_at: string
  endpoint_name?: string
  latency_ms?: number
}

export interface RequestsResponse {
  items: RequestLog[]
  total: number
  page: number
  page_size: number
}

export interface HourlyStat {
  hour: string
  count: number
  chars: number
}

export interface DailyStat {
  day: string
  count: number
  chars: number
}

export interface ChartData {
  hourly: HourlyStat[]
  daily: DailyStat[]
}

export interface LangStat {
  lang: string
  source_chars: number
  target_chars: number
}

export interface LangHourlyUsage {
  hour: string
  lang: string
  count: number
}

export interface EndpointStatus {
  name: string
  url: string
  healthy: boolean
  consecutive_failures: number
  total_requests: number
  total_successes: number
  avg_latency_ms: number
  last_error: string | null
}

export interface CacheStats {
  enabled: boolean
  hits: number
  misses: number
  today_hits: number
  today_misses: number
  size: number
  max_entries: number
  ttl_secs: number
  hit_rate: number
  today_hit_rate: number
  max_memory_mb: number
  estimated_memory_bytes: number
}

export interface CacheHitEntry {
  source_lang: string
  target_lang: string
  text_preview: string
  timestamp: string
}

export interface FullEndpointInfo {
  name: string
  url: string
  api_key: string
}

export interface FullUpstreamInfo {
  endpoints: FullEndpointInfo[]
  max_failures: number
  probe_interval_secs: number
}

export interface FullProxyInfo {
  host: string
  port: number
}

export interface FullMonitorInfo {
  auto_refresh_seconds: number
  max_log_entries: number
}

export interface FullHealthCheckInfo {
  source_lang: string
  target_lang: string
}

export interface FullCacheInfo {
  enabled: boolean
  ttl_secs: number
  max_entries: number
  max_memory_mb: number
}

export interface FullDemoInfo {
  enabled: boolean
  seed: number
}

export interface FullConfig {
  upstream: FullUpstreamInfo
  proxy: FullProxyInfo
  monitor: FullMonitorInfo
  health_check: FullHealthCheckInfo
  cache: FullCacheInfo
  demo: FullDemoInfo
}

export interface HeatmapCell {
  x: string
  y: number
  count: number
}

export interface HeatmapResponse {
  view: string
  data: HeatmapCell[]
}

export interface ErrorTrendPoint {
  time: string
  total: number
  errors: number
  error_rate: number
}

export type Tab = 'overview' | 'analysis' | 'endpoints' | 'logs' | 'settings'
```

- [ ] **Step 2: Create src/hooks/useApi.ts**

```typescript
type ToastFn = (msg: string) => void

let globalToast: ToastFn = () => {}

export function setToastHandler(fn: ToastFn) {
  globalToast = fn
}

export async function apiFetch<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(url, options)
  if (!res.ok) {
    let msg = `HTTP ${res.status}`
    try {
      const body = await res.json()
      if (body.error) msg = body.error
      else if (body.message) msg = body.message
    } catch {}
    globalToast(msg)
    throw new Error(msg)
  }
  return res.json()
}

export async function apiPost<T>(url: string, body?: unknown): Promise<T> {
  return apiFetch<T>(url, {
    method: 'POST',
    headers: body ? { 'Content-Type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  })
}
```

- [ ] **Step 3: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(frontend): add TypeScript types and API fetch hook"
```

---

## Task 5: Zustand Stores

**Files:**
- Create: `src/stores/useStatsStore.ts`, `src/stores/useEndpointStore.ts`, `src/stores/useLogsStore.ts`, `src/stores/useConfigStore.ts`

- [ ] **Step 1: Create src/stores/useStatsStore.ts**

```typescript
import { create } from 'zustand'
import { apiFetch } from '@/hooks/useApi'
import type { StatsResponse, ChartData, LangStat, LangHourlyUsage, HeatmapResponse, ErrorTrendPoint } from '@/types'

interface StatsState {
  stats: StatsResponse | null
  chartData: ChartData | null
  langStats: LangStat[]
  langHourlyStats: LangHourlyUsage[]
  heatmapData: HeatmapResponse | null
  errorTrend: ErrorTrendPoint[]
  loading: boolean
  error: string | null
  days: number | null
  refreshInterval: number
  appVersion: string

  setDays: (days: number | null) => void
  setRefreshInterval: (seconds: number) => void
  fetchStats: (days?: number | null, endpoint?: string) => Promise<void>
  fetchChart: (endpoint?: string) => Promise<void>
  fetchLangStats: (days?: number | null) => Promise<void>
  fetchLangHourlyStats: (days?: number) => Promise<void>
  fetchHeatmap: (view?: string, days?: number) => Promise<void>
  fetchErrorTrend: (days?: number, granularity?: string) => Promise<void>
  fetchVersion: () => Promise<void>
  fetchAll: (days?: number | null) => Promise<void>
}

export const useStatsStore = create<StatsState>((set, get) => ({
  stats: null,
  chartData: null,
  langStats: [],
  langHourlyStats: [],
  heatmapData: null,
  errorTrend: [],
  loading: false,
  error: null,
  days: 1,
  refreshInterval: 0,
  appVersion: '',

  setDays: (days) => set({ days }),
  setRefreshInterval: (seconds) => set({ refreshInterval: seconds }),

  fetchStats: async (days, endpoint) => {
    try {
      const params = new URLSearchParams()
      const d = days ?? get().days
      if (d !== null) params.set('days', String(d))
      if (endpoint) params.set('endpoint', endpoint)
      const query = params.toString()
      const data = await apiFetch<StatsResponse>(query ? `/api/stats?${query}` : '/api/stats')
      set({ stats: data, error: null })
    } catch (e) {
      set({ error: (e as Error).message })
    }
  },

  fetchChart: async (endpoint) => {
    try {
      const params = endpoint ? `?endpoint=${endpoint}` : ''
      const data = await apiFetch<ChartData>(`/api/chart${params}`)
      set({ chartData: data })
    } catch (e) {
      set({ error: (e as Error).message })
    }
  },

  fetchLangStats: async (days) => {
    try {
      const d = days ?? get().days
      const params = d !== null ? `?days=${d}` : ''
      const data = await apiFetch<LangStat[]>(`/api/lang-stats${params}`)
      set({ langStats: data })
    } catch (e) {
      set({ error: (e as Error).message })
    }
  },

  fetchLangHourlyStats: async (days = 1) => {
    try {
      const params = days > 1 ? `?days=${days}` : ''
      const data = await apiFetch<LangHourlyUsage[]>(`/api/lang-hourly-stats${params}`)
      set({ langHourlyStats: data })
    } catch (e) {
      set({ error: (e as Error).message })
    }
  },

  fetchHeatmap: async (view = 'weekday', days = 30) => {
    try {
      const data = await apiFetch<HeatmapResponse>(`/api/analytics/heatmap?view=${view}&days=${days}`)
      set({ heatmapData: data })
    } catch (e) {
      set({ error: (e as Error).message })
    }
  },

  fetchErrorTrend: async (days = 7, granularity) => {
    try {
      const params = new URLSearchParams({ days: String(days) })
      if (granularity) params.set('granularity', granularity)
      const data = await apiFetch<ErrorTrendPoint[]>(`/api/analytics/error-trend?${params}`)
      set({ errorTrend: data })
    } catch (e) {
      set({ error: (e as Error).message })
    }
  },

  fetchVersion: async () => {
    try {
      const data = await apiFetch<{ version: string }>('/api/version')
      set({ appVersion: data.version ?? '' })
    } catch {}
  },

  fetchAll: async (days) => {
    set({ loading: true })
    const d = days ?? get().days
    await Promise.all([
      get().fetchStats(d),
      get().fetchChart(),
    ])
    set({ loading: false })
  },
}))
```

- [ ] **Step 2: Create src/stores/useEndpointStore.ts**

```typescript
import { create } from 'zustand'
import { apiFetch, apiPost } from '@/hooks/useApi'
import type { EndpointStatus, CacheStats, CacheHitEntry, HealthStatus } from '@/types'

interface EndpointState {
  endpoints: EndpointStatus[]
  cacheStats: CacheStats | null
  cacheHitLogs: CacheHitEntry[]
  loading: boolean

  fetchUpstreamStatus: () => Promise<void>
  fetchCacheStats: () => Promise<void>
  fetchCacheHitLogs: () => Promise<void>
  clearCache: () => Promise<boolean>
  triggerHealthCheck: () => Promise<HealthStatus | null>
}

export const useEndpointStore = create<EndpointState>((set) => ({
  endpoints: [],
  cacheStats: null,
  cacheHitLogs: [],
  loading: false,

  fetchUpstreamStatus: async () => {
    try {
      const data = await apiFetch<EndpointStatus[]>('/api/upstream/status')
      set({ endpoints: data })
    } catch {}
  },

  fetchCacheStats: async () => {
    try {
      const data = await apiFetch<CacheStats>('/api/cache/stats')
      set({ cacheStats: data })
    } catch {}
  },

  fetchCacheHitLogs: async () => {
    try {
      const data = await apiFetch<CacheHitEntry[]>('/api/cache/hits')
      set({ cacheHitLogs: data })
    } catch {}
  },

  clearCache: async () => {
    try {
      await apiPost('/api/cache/clear')
      const data = await apiFetch<CacheStats>('/api/cache/stats')
      set({ cacheStats: data })
      return true
    } catch {
      return false
    }
  },

  triggerHealthCheck: async () => {
    try {
      const health = await apiPost<HealthStatus>('/api/health/check')
      const endpoints = await apiFetch<EndpointStatus[]>('/api/upstream/status')
      set({ endpoints })
      return health
    } catch {
      return null
    }
  },
}))
```

- [ ] **Step 3: Create src/stores/useLogsStore.ts**

```typescript
import { create } from 'zustand'
import { apiFetch } from '@/hooks/useApi'
import type { RequestLog, RequestsResponse } from '@/types'

interface LogsState {
  logs: RequestLog[]
  total: number
  page: number
  pageSize: number
  loading: boolean
  filterStatus: string
  filterEndpoint: string
  filterLang: string

  setFilter: (key: 'filterStatus' | 'filterEndpoint' | 'filterLang', value: string) => void
  fetchLogs: (page?: number, pageSize?: number) => Promise<void>
  exportLogs: (format?: string, filters?: { start?: string; end?: string; lang?: string; status?: string }) => void
}

export const useLogsStore = create<LogsState>((set, get) => ({
  logs: [],
  total: 0,
  page: 1,
  pageSize: 50,
  loading: false,
  filterStatus: '',
  filterEndpoint: '',
  filterLang: '',

  setFilter: (key, value) => {
    set({ [key]: value, page: 1 })
    get().fetchLogs(1)
  },

  fetchLogs: async (page, pageSize) => {
    const p = page ?? get().page
    const ps = pageSize ?? get().pageSize
    set({ loading: true })
    try {
      const params = new URLSearchParams({ page: String(p), page_size: String(ps) })
      const { filterStatus, filterEndpoint, filterLang } = get()
      if (filterStatus) params.set('status', filterStatus)
      if (filterEndpoint) params.set('endpoint', filterEndpoint)
      if (filterLang) params.set('lang', filterLang)
      const data = await apiFetch<RequestsResponse>(`/api/requests?${params}`)
      set({ logs: data.items, total: data.total, page: data.page, pageSize: data.page_size })
    } catch {} finally {
      set({ loading: false })
    }
  },

  exportLogs: (format = 'csv', filters) => {
    const params = new URLSearchParams({ format })
    if (filters?.start) params.set('start', filters.start)
    if (filters?.end) params.set('end', filters.end)
    if (filters?.lang) params.set('lang', filters.lang)
    if (filters?.status) params.set('status', filters.status)
    const url = `/api/export?${params}`
    const a = document.createElement('a')
    a.href = url
    a.download = `export.${format}`
    a.click()
  },
}))
```

- [ ] **Step 4: Create src/stores/useConfigStore.ts**

```typescript
import { create } from 'zustand'
import { apiFetch, apiPost } from '@/hooks/useApi'
import type { FullConfig } from '@/types'

interface ConfigState {
  config: FullConfig | null
  loading: boolean
  error: string | null

  fetchConfig: () => Promise<FullConfig | null>
  updateConfig: (payload: FullConfig) => Promise<boolean>
}

export const useConfigStore = create<ConfigState>((set) => ({
  config: null,
  loading: false,
  error: null,

  fetchConfig: async () => {
    set({ loading: true })
    try {
      const data = await apiFetch<FullConfig>('/api/config')
      set({ config: data, error: null })
      return data
    } catch (e) {
      set({ error: (e as Error).message })
      return null
    } finally {
      set({ loading: false })
    }
  },

  updateConfig: async (payload) => {
    try {
      const data = await apiPost<FullConfig>('/api/config', payload)
      set({ config: data, error: null })
      return true
    } catch (e) {
      set({ error: (e as Error).message })
      return false
    }
  },
}))
```

- [ ] **Step 5: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(frontend): add Zustand stores for stats, endpoints, logs, config"
```

---

## Task 6: UI Components

**Files:**
- Create: `src/components/ui/Button/Button.tsx`, `src/components/ui/Button/Button.module.scss`
- Create: `src/components/ui/Card/Card.tsx`, `src/components/ui/Card/Card.module.scss`
- Create: `src/components/ui/Input/Input.tsx`, `src/components/ui/Input/Input.module.scss`
- Create: `src/components/ui/Select/Select.tsx`, `src/components/ui/Select/Select.module.scss`
- Create: `src/components/ui/Modal/Modal.tsx`, `src/components/ui/Modal/Modal.module.scss`
- Create: `src/components/ui/LoadingSpinner/LoadingSpinner.tsx`, `src/components/ui/LoadingSpinner/LoadingSpinner.module.scss`
- Create: `src/components/ui/EmptyState/EmptyState.tsx`, `src/components/ui/EmptyState/EmptyState.module.scss`

- [ ] **Step 1: Create Button component**

`src/components/ui/Button/Button.tsx`:
```tsx
import { ButtonHTMLAttributes } from 'react'
import styles from './Button.module.scss'

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost'
  size?: 'sm' | 'md' | 'lg'
}

export function Button({ variant = 'primary', size = 'md', className, children, ...props }: ButtonProps) {
  return (
    <button className={`${styles.btn} ${styles[variant]} ${styles[size]} ${className ?? ''}`} {...props}>
      {children}
    </button>
  )
}
```

`src/components/ui/Button/Button.module.scss`:
```scss
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border-radius: var(--radius-sm);
  font-weight: 500;
  transition: all 150ms ease;
  white-space: nowrap;

  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
}

.sm { padding: 6px 12px; font-size: 13px; }
.md { padding: 8px 16px; font-size: 14px; }
.lg { padding: 10px 20px; font-size: 15px; }

.primary {
  background: var(--color-primary);
  color: #fff;
  &:hover:not(:disabled) { background: var(--color-primary-hover); }
}

.secondary {
  background: var(--bg-hover);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
  &:hover:not(:disabled) { background: var(--border-color); }
}

.danger {
  background: var(--color-error);
  color: #fff;
  &:hover:not(:disabled) { opacity: 0.9; }
}

.ghost {
  background: transparent;
  color: var(--text-secondary);
  &:hover:not(:disabled) { background: var(--bg-hover); color: var(--text-primary); }
}
```

- [ ] **Step 2: Create Card component**

`src/components/ui/Card/Card.tsx`:
```tsx
import { HTMLAttributes } from 'react'
import styles from './Card.module.scss'

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  padding?: 'sm' | 'md' | 'lg'
}

export function Card({ padding = 'md', className, children, ...props }: CardProps) {
  return (
    <div className={`${styles.card} ${styles[padding]} ${className ?? ''}`} {...props}>
      {children}
    </div>
  )
}
```

`src/components/ui/Card/Card.module.scss`:
```scss
.card {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
}

.sm { padding: 16px; }
.md { padding: 24px; }
.lg { padding: 32px; }
```

- [ ] **Step 3: Create Input component**

`src/components/ui/Input/Input.tsx`:
```tsx
import { InputHTMLAttributes, forwardRef } from 'react'
import styles from './Input.module.scss'

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string
}

export const Input = forwardRef<HTMLInputElement, InputProps>(({ label, className, ...props }, ref) => (
  <label className={styles.wrapper}>
    {label && <span className={styles.label}>{label}</span>}
    <input ref={ref} className={`${styles.input} ${className ?? ''}`} {...props} />
  </label>
))

Input.displayName = 'Input'
```

`src/components/ui/Input/Input.module.scss`:
```scss
.wrapper {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
}

.input {
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
  transition: border-color 150ms ease;

  &:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  &::placeholder {
    color: var(--text-tertiary);
  }
}
```

- [ ] **Step 4: Create Select component**

`src/components/ui/Select/Select.tsx`:
```tsx
import { SelectHTMLAttributes } from 'react'
import styles from './Select.module.scss'

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string
  options: { value: string | number; label: string }[]
}

export function Select({ label, options, className, ...props }: SelectProps) {
  return (
    <label className={styles.wrapper}>
      {label && <span className={styles.label}>{label}</span>}
      <select className={`${styles.select} ${className ?? ''}`} {...props}>
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>{opt.label}</option>
        ))}
      </select>
    </label>
  )
}
```

`src/components/ui/Select/Select.module.scss`:
```scss
.wrapper {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
}

.select {
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
  transition: border-color 150ms ease;
  cursor: pointer;

  &:focus {
    outline: none;
    border-color: var(--color-primary);
  }
}
```

- [ ] **Step 5: Create Modal component**

`src/components/ui/Modal/Modal.tsx`:
```tsx
import { useEffect, useRef, type ReactNode } from 'react'
import styles from './Modal.module.scss'

interface ModalProps {
  open: boolean
  onClose: () => void
  title?: string
  children: ReactNode
}

export function Modal({ open, onClose, title, children }: ModalProps) {
  const overlayRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const handler = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose() }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [open, onClose])

  if (!open) return null

  return (
    <div className={styles.overlay} ref={overlayRef} onClick={(e) => { if (e.target === overlayRef.current) onClose() }}>
      <div className={styles.modal}>
        {title && (
          <div className={styles.header}>
            <h3>{title}</h3>
            <button className={styles.close} onClick={onClose} aria-label="Close">&times;</button>
          </div>
        )}
        <div className={styles.body}>{children}</div>
      </div>
    </div>
  )
}
```

`src/components/ui/Modal/Modal.module.scss`:
```scss
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  animation: fadeIn 150ms ease;
}

.modal {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  min-width: 360px;
  max-width: 90vw;
  max-height: 85vh;
  overflow-y: auto;
  animation: scaleIn 200ms cubic-bezier(0.34, 1.56, 0.64, 1);
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 0;

  h3 {
    font-size: 16px;
    font-weight: 600;
  }
}

.close {
  font-size: 22px;
  color: var(--text-tertiary);
  padding: 4px 8px;
  border-radius: var(--radius-sm);

  &:hover { background: var(--bg-hover); color: var(--text-primary); }
}

.body {
  padding: 20px 24px 24px;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes scaleIn {
  from { transform: scale(0.95); opacity: 0; }
  to { transform: scale(1); opacity: 1; }
}
```

- [ ] **Step 6: Create LoadingSpinner component**

`src/components/ui/LoadingSpinner/LoadingSpinner.tsx`:
```tsx
import styles from './LoadingSpinner.module.scss'

export function LoadingSpinner({ size = 24 }: { size?: number }) {
  return <div className={styles.spinner} style={{ width: size, height: size }} />
}
```

`src/components/ui/LoadingSpinner/LoadingSpinner.module.scss`:
```scss
.spinner {
  border: 2px solid var(--border-color);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 600ms linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
```

- [ ] **Step 7: Create EmptyState component**

`src/components/ui/EmptyState/EmptyState.tsx`:
```tsx
import { useTranslation } from 'react-i18next'
import styles from './EmptyState.module.scss'

export function EmptyState({ message }: { message?: string }) {
  const { t } = useTranslation()
  return (
    <div className={styles.empty}>
      <p>{message ?? t('common.empty')}</p>
    </div>
  )
}
```

`src/components/ui/EmptyState/EmptyState.module.scss`:
```scss
.empty {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
  color: var(--text-tertiary);
  font-size: 14px;
}
```

- [ ] **Step 8: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat(frontend): add UI component library (Button, Card, Input, Select, Modal, Spinner, EmptyState)"
```

---

## Task 7: Layout Components & Shared Components

**Files:**
- Create: `src/components/layout/TopBar.tsx`, `src/components/layout/TopBar.module.scss`
- Create: `src/components/layout/PageShell.tsx`, `src/components/layout/PageShell.module.scss`
- Create: `src/components/shared/ThemeSwitcher.tsx`
- Create: `src/components/shared/LanguageSwitcher.tsx`
- Create: `src/components/shared/Toast.tsx`, `src/components/shared/Toast.module.scss`
- Create: `src/hooks/useAutoRefresh.ts`

- [ ] **Step 1: Create TopBar**

`src/components/layout/TopBar.tsx`:
```tsx
import { useTranslation } from 'react-i18next'
import { ThemeSwitcher } from '@/components/shared/ThemeSwitcher'
import { LanguageSwitcher } from '@/components/shared/LanguageSwitcher'
import type { Tab } from '@/types'
import styles from './TopBar.module.scss'

interface TopBarProps {
  activeTab: Tab
  onTabChange: (tab: Tab) => void
}

const TABS: Tab[] = ['overview', 'analysis', 'endpoints', 'logs', 'settings']

export function TopBar({ activeTab, onTabChange }: TopBarProps) {
  const { t } = useTranslation()

  return (
    <header className={styles.topbar}>
      <div className={styles.brand}>
        <img src="/favicon.svg" alt="" width={20} height={20} />
        <span className={styles.brandText}>{t('brand')}</span>
      </div>

      <nav className={styles.tabs}>
        {TABS.map((tab) => (
          <button
            key={tab}
            className={`${styles.tab} ${activeTab === tab ? styles.active : ''}`}
            onClick={() => onTabChange(tab)}
          >
            {t(`tabs.${tab}`)}
          </button>
        ))}
      </nav>

      <div className={styles.actions}>
        <ThemeSwitcher />
        <LanguageSwitcher />
      </div>
    </header>
  )
}
```

`src/components/layout/TopBar.module.scss`:
```scss
@use '../../styles/mixins' as *;

.topbar {
  position: sticky;
  top: 12px;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 24px;
  margin: 12px auto 0;
  max-width: 1245px;
  border-radius: var(--radius-pill);
  background: var(--bg-topbar);
  border: 1px solid var(--border-color);
  box-shadow: var(--shadow-md);
  @include glass;

  @include responsive {
    flex-wrap: wrap;
    gap: 12px;
    border-radius: var(--radius-md);
    top: 0;
    margin-top: 0;
  }
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
}

.brandText {
  font-size: 15px;
  font-weight: 700;
  letter-spacing: 0.04em;
}

.tabs {
  display: flex;
  gap: 4px;

  @include responsive {
    width: 100%;
    order: 3;
    overflow-x: auto;
  }
}

.tab {
  padding: 6px 14px;
  border-radius: var(--radius-sm);
  font-size: 14px;
  font-weight: 500;
  color: var(--text-secondary);
  @include transition;

  &:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
}

.active {
  background: var(--bg-hover);
  color: var(--text-primary);
  font-weight: 600;
}

.actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
```

- [ ] **Step 2: Create PageShell**

`src/components/layout/PageShell.tsx`:
```tsx
import { type ReactNode } from 'react'
import styles from './PageShell.module.scss'

export function PageShell({ children }: { children: ReactNode }) {
  return (
    <main className={styles.shell}>
      <div className={styles.content}>{children}</div>
    </main>
  )
}
```

`src/components/layout/PageShell.module.scss`:
```scss
.shell {
  min-height: calc(100vh - 80px);
  padding: 24px 16px 48px;
  position: relative;

  &::before {
    content: '';
    position: fixed;
    inset: 0;
    background: radial-gradient(ellipse at 50% 0%, var(--bg-hover) 0%, transparent 60%);
    opacity: 0.4;
    pointer-events: none;
    z-index: -1;
  }
}

.content {
  max-width: 1245px;
  margin: 0 auto;
}
```

- [ ] **Step 3: Create ThemeSwitcher**

`src/components/shared/ThemeSwitcher.tsx`:
```tsx
import { useThemeStore } from '@/stores/useThemeStore'
import styles from './Switcher.module.scss'

export function ThemeSwitcher() {
  const { theme, toggleTheme } = useThemeStore()

  return (
    <button className={styles.switcher} onClick={toggleTheme} aria-label="Toggle theme">
      {theme === 'light' ? (
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
        </svg>
      ) : (
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="5"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
        </svg>
      )}
    </button>
  )
}
```

- [ ] **Step 4: Create LanguageSwitcher**

`src/components/shared/LanguageSwitcher.tsx`:
```tsx
import { useTranslation } from 'react-i18next'
import styles from './Switcher.module.scss'

export function LanguageSwitcher() {
  const { i18n } = useTranslation()

  const toggle = () => {
    const next = i18n.language === 'zh' ? 'en' : 'zh'
    i18n.changeLanguage(next)
    localStorage.setItem('language', next)
  }

  return (
    <button className={styles.switcher} onClick={toggle} aria-label="Switch language">
      <span style={{ fontSize: 13, fontWeight: 600 }}>{i18n.language === 'zh' ? 'EN' : '中'}</span>
    </button>
  )
}
```

`src/components/shared/Switcher.module.scss`:
```scss
.switcher {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  transition: all 150ms ease;

  &:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
}
```

- [ ] **Step 5: Create Toast system**

`src/components/shared/Toast.tsx`:
```tsx
import { useEffect, useState, useCallback } from 'react'
import { setToastHandler } from '@/hooks/useApi'
import styles from './Toast.module.scss'

interface ToastItem {
  id: number
  message: string
}

let nextId = 0

export function ToastContainer() {
  const [toasts, setToasts] = useState<ToastItem[]>([])

  const addToast = useCallback((message: string) => {
    const id = nextId++
    setToasts((prev) => [...prev, { id, message }])
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id))
    }, 3000)
  }, [])

  useEffect(() => {
    setToastHandler(addToast)
  }, [addToast])

  return (
    <div className={styles.container}>
      {toasts.map((toast) => (
        <div key={toast.id} className={styles.toast}>{toast.message}</div>
      ))}
    </div>
  )
}

export { setToastHandler as showToast }
```

`src/components/shared/Toast.module.scss`:
```scss
.container {
  position: fixed;
  top: 16px;
  right: 16px;
  z-index: 2000;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.toast {
  padding: 12px 20px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-lg);
  font-size: 14px;
  color: var(--text-primary);
  animation: slideIn 200ms ease;
  max-width: 320px;
}

@keyframes slideIn {
  from { transform: translateX(100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}
```

- [ ] **Step 6: Create useAutoRefresh hook**

`src/hooks/useAutoRefresh.ts`:
```typescript
import { useEffect, useRef } from 'react'

export function useAutoRefresh(callback: () => void, intervalSeconds: number) {
  const callbackRef = useRef(callback)
  callbackRef.current = callback

  useEffect(() => {
    if (intervalSeconds <= 0) return

    let timer: ReturnType<typeof setTimeout>
    let paused = false

    const tick = () => {
      if (!paused) callbackRef.current()
      timer = setTimeout(tick, intervalSeconds * 1000)
    }

    timer = setTimeout(tick, intervalSeconds * 1000)

    const onVisibility = () => {
      paused = document.hidden
      if (!paused) callbackRef.current()
    }

    document.addEventListener('visibilitychange', onVisibility)

    return () => {
      clearTimeout(timer)
      document.removeEventListener('visibilitychange', onVisibility)
    }
  }, [intervalSeconds])
}
```

- [ ] **Step 7: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(frontend): add layout (TopBar, PageShell), shared components, Toast, useAutoRefresh"
```

---

## Task 8: Chart Components

**Files:**
- Create: `src/components/charts/MiniChart.tsx`
- Create: `src/components/charts/TrendChart.tsx`, `src/components/charts/TrendChart.module.scss`
- Create: `src/components/charts/Heatmap.tsx`, `src/components/charts/Heatmap.module.scss`
- Create: `src/components/charts/ErrorTrend.tsx`
- Create: `src/components/shared/StatCard.tsx`, `src/components/shared/StatCard.module.scss`

- [ ] **Step 1: Create MiniChart (Recharts sparkline)**

`src/components/charts/MiniChart.tsx`:
```tsx
import { LineChart, Line, ResponsiveContainer } from 'recharts'

interface MiniChartProps {
  data: { value: number }[]
  color?: string
  height?: number
}

export function MiniChart({ data, color = 'var(--color-primary)', height = 40 }: MiniChartProps) {
  if (!data.length) return null

  return (
    <ResponsiveContainer width="100%" height={height}>
      <LineChart data={data}>
        <Line
          type="monotone"
          dataKey="value"
          stroke={color}
          strokeWidth={1.5}
          dot={false}
          isAnimationActive={false}
        />
      </LineChart>
    </ResponsiveContainer>
  )
}
```

- [ ] **Step 2: Create StatCard**

`src/components/shared/StatCard.tsx`:
```tsx
import { MiniChart } from '@/components/charts/MiniChart'
import styles from './StatCard.module.scss'

interface StatCardProps {
  label: string
  value: string
  subtitle?: string
  chartData?: { value: number }[]
  chartColor?: string
}

export function StatCard({ label, value, subtitle, chartData, chartColor }: StatCardProps) {
  return (
    <article className={styles.card}>
      <div className={styles.head}>
        <span className={styles.label}>{label}</span>
        {subtitle && <span className={styles.subtitle}>{subtitle}</span>}
      </div>
      <div className={styles.value}>{value}</div>
      {chartData && chartData.length > 0 && (
        <div className={styles.chart}>
          <MiniChart data={chartData} color={chartColor} />
        </div>
      )}
    </article>
  )
}
```

`src/components/shared/StatCard.module.scss`:
```scss
.card {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-sm);
  padding: 20px;
}

.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.label {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
}

.subtitle {
  font-size: 12px;
  color: var(--text-tertiary);
}

.value {
  font-size: 24px;
  font-weight: 800;
  color: var(--text-primary);
  margin-bottom: 12px;
  font-variant-numeric: tabular-nums;
}

.chart {
  margin-top: 8px;
}
```

- [ ] **Step 3: Create TrendChart (Chart.js)**

`src/components/charts/TrendChart.tsx`:
```tsx
import { useRef, useEffect } from 'react'
import { Chart, registerables } from 'chart.js'
import styles from './TrendChart.module.scss'

Chart.register(...registerables)

interface TrendChartProps {
  labels: string[]
  datasets: {
    label: string
    data: number[]
    borderColor?: string
    backgroundColor?: string
  }[]
  height?: number
}

export function TrendChart({ labels, datasets, height = 280 }: TrendChartProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const chartRef = useRef<Chart | null>(null)

  useEffect(() => {
    if (!canvasRef.current) return

    if (chartRef.current) {
      chartRef.current.destroy()
    }

    const ctx = canvasRef.current.getContext('2d')!
    chartRef.current = new Chart(ctx, {
      type: 'line',
      data: {
        labels,
        datasets: datasets.map((ds) => ({
          ...ds,
          borderColor: ds.borderColor ?? getComputedStyle(document.documentElement).getPropertyValue('--color-primary').trim(),
          backgroundColor: ds.backgroundColor ?? 'transparent',
          borderWidth: 2,
          pointRadius: 0,
          pointHoverRadius: 4,
          tension: 0.3,
          fill: !!ds.backgroundColor,
        })),
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        interaction: { mode: 'index', intersect: false },
        plugins: {
          legend: { display: datasets.length > 1, position: 'top', labels: { boxWidth: 12, padding: 16 } },
          tooltip: { backgroundColor: 'rgba(0,0,0,0.8)', padding: 10, cornerRadius: 8 },
        },
        scales: {
          x: { grid: { display: false }, ticks: { maxTicksLimit: 8, font: { size: 11 } } },
          y: { beginAtZero: true, grid: { color: 'var(--border-color)' }, ticks: { font: { size: 11 } } },
        },
      },
    })

    return () => { chartRef.current?.destroy() }
  }, [labels, datasets])

  return (
    <div className={styles.wrapper} style={{ height }}>
      <canvas ref={canvasRef} />
    </div>
  )
}
```

`src/components/charts/TrendChart.module.scss`:
```scss
.wrapper {
  position: relative;
  width: 100%;
}
```

- [ ] **Step 4: Create Heatmap (Recharts)**

`src/components/charts/Heatmap.tsx`:
```tsx
import type { HeatmapCell } from '@/types'
import styles from './Heatmap.module.scss'

interface HeatmapProps {
  data: HeatmapCell[]
  xLabels: string[]
  yLabels: string[]
}

function getIntensity(count: number, max: number): number {
  if (max === 0) return 0
  return Math.min(count / max, 1)
}

export function Heatmap({ data, xLabels, yLabels }: HeatmapProps) {
  const max = Math.max(...data.map((d) => d.count), 1)

  const cellMap = new Map<string, number>()
  data.forEach((d) => cellMap.set(`${d.x}-${d.y}`, d.count))

  return (
    <div className={styles.container}>
      <div className={styles.grid} style={{ gridTemplateColumns: `auto repeat(${xLabels.length}, 1fr)` }}>
        <div />
        {xLabels.map((x) => (
          <div key={x} className={styles.xLabel}>{x}</div>
        ))}
        {yLabels.map((y) => (
          <>
            <div key={`y-${y}`} className={styles.yLabel}>{y}</div>
            {xLabels.map((x) => {
              const count = cellMap.get(`${x}-${y}`) ?? 0
              const intensity = getIntensity(count, max)
              return (
                <div
                  key={`${x}-${y}`}
                  className={styles.cell}
                  style={{ opacity: 0.1 + intensity * 0.9, backgroundColor: `var(--color-primary)` }}
                  title={`${x}, ${y}: ${count}`}
                />
              )
            })}
          </>
        ))}
      </div>
    </div>
  )
}
```

`src/components/charts/Heatmap.module.scss`:
```scss
.container {
  overflow-x: auto;
}

.grid {
  display: grid;
  gap: 2px;
  min-width: fit-content;
}

.xLabel, .yLabel {
  font-size: 11px;
  color: var(--text-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
}

.yLabel {
  justify-content: flex-end;
  padding-right: 8px;
}

.cell {
  width: 100%;
  aspect-ratio: 1;
  border-radius: 3px;
  min-width: 16px;
  min-height: 16px;
}
```

- [ ] **Step 5: Create ErrorTrend (Chart.js)**

`src/components/charts/ErrorTrend.tsx`:
```tsx
import { useRef, useEffect } from 'react'
import { Chart, registerables } from 'chart.js'

Chart.register(...registerables)

interface ErrorTrendProps {
  labels: string[]
  errorRates: number[]
  height?: number
}

export function ErrorTrend({ labels, errorRates, height = 220 }: ErrorTrendProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const chartRef = useRef<Chart | null>(null)

  useEffect(() => {
    if (!canvasRef.current) return
    if (chartRef.current) chartRef.current.destroy()

    const ctx = canvasRef.current.getContext('2d')!
    chartRef.current = new Chart(ctx, {
      type: 'line',
      data: {
        labels,
        datasets: [{
          label: 'Error Rate %',
          data: errorRates,
          borderColor: getComputedStyle(document.documentElement).getPropertyValue('--color-error').trim(),
          backgroundColor: 'rgba(198, 87, 70, 0.1)',
          borderWidth: 2,
          pointRadius: 0,
          tension: 0.3,
          fill: true,
        }],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { display: false },
          tooltip: { backgroundColor: 'rgba(0,0,0,0.8)', padding: 10, cornerRadius: 8 },
        },
        scales: {
          x: { grid: { display: false }, ticks: { maxTicksLimit: 8, font: { size: 11 } } },
          y: { beginAtZero: true, max: 100, ticks: { callback: (v) => `${v}%`, font: { size: 11 } } },
        },
      },
    })

    return () => { chartRef.current?.destroy() }
  }, [labels, errorRates])

  return (
    <div style={{ position: 'relative', height }}>
      <canvas ref={canvasRef} />
    </div>
  )
}
```

- [ ] **Step 6: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(frontend): add chart components (MiniChart, TrendChart, Heatmap, ErrorTrend, StatCard)"
```

---

## Task 9: Overview Page

**Files:**
- Create: `src/pages/Overview.tsx`, `src/pages/Overview.module.scss`

- [ ] **Step 1: Create Overview page**

`src/pages/Overview.tsx`:
```tsx
import { useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useStatsStore } from '@/stores/useStatsStore'
import { useEndpointStore } from '@/stores/useEndpointStore'
import { useAutoRefresh } from '@/hooks/useAutoRefresh'
import { Card } from '@/components/ui/Card/Card'
import { Select } from '@/components/ui/Select/Select'
import { Button } from '@/components/ui/Button/Button'
import { StatCard } from '@/components/shared/StatCard'
import { TrendChart } from '@/components/charts/TrendChart'
import styles from './Overview.module.scss'

export function Overview() {
  const { t } = useTranslation()
  const {
    stats, chartData, days, refreshInterval,
    setDays, setRefreshInterval, fetchStats, fetchChart, fetchAll,
  } = useStatsStore()
  const { endpoints, fetchUpstreamStatus } = useEndpointStore()

  useEffect(() => {
    fetchAll()
    fetchUpstreamStatus()
  }, [fetchAll, fetchUpstreamStatus])

  const refresh = useCallback(() => {
    fetchAll()
    fetchUpstreamStatus()
  }, [fetchAll, fetchUpstreamStatus])

  useAutoRefresh(refresh, refreshInterval)

  const onDaysChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const v = Number(e.target.value)
    setDays(v === 0 ? null : v)
    fetchStats(v === 0 ? null : v)
    fetchChart()
  }

  const onRefreshChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setRefreshInterval(Number(e.target.value))
  }

  const successRate = stats
    ? stats.total_requests > 0
      ? ((stats.total_requests - (stats.health.status === 'error' ? 1 : 0)) / stats.total_requests * 100).toFixed(1)
      : '100.0'
    : '-'

  const hourlyData = chartData?.hourly.map((h) => ({ value: h.count })) ?? []
  const dailyData = chartData?.daily.map((d) => ({ value: d.count })) ?? []
  const chartLabels = chartData?.daily.map((d) => d.day) ?? chartData?.hourly.map((h) => h.hour) ?? []
  const chartValues = chartData?.daily.map((d) => d.count) ?? chartData?.hourly.map((h) => h.count) ?? []

  const formatNum = (n: number) => {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
    return String(n)
  }

  return (
    <div className={styles.page}>
      <div className={styles.controls}>
        <Select
          label={t('overview.range')}
          value={days ?? 0}
          onChange={onDaysChange}
          options={[
            { value: 1, label: t('overview.today') },
            { value: 7, label: t('overview.days7') },
            { value: 30, label: t('overview.days30') },
            { value: 90, label: t('overview.days90') },
            { value: 0, label: t('overview.all') },
          ]}
        />
        <Select
          label={t('overview.autoRefresh')}
          value={refreshInterval}
          onChange={onRefreshChange}
          options={[
            { value: 0, label: t('overview.off') },
            { value: 5, label: `5${t('overview.seconds')}` },
            { value: 10, label: `10${t('overview.seconds')}` },
            { value: 30, label: `30${t('overview.seconds')}` },
            { value: 60, label: `60${t('overview.seconds')}` },
          ]}
        />
        <Button variant="ghost" size="sm" onClick={refresh}>{t('overview.refresh')}</Button>
      </div>

      <div className={styles.statsGrid}>
        <StatCard
          label={t('overview.totalRequests')}
          value={formatNum(stats?.total_requests ?? 0)}
          subtitle={t('overview.cumulative')}
          chartData={hourlyData}
        />
        <StatCard
          label={t('overview.totalChars')}
          value={formatNum(stats?.total_chars ?? 0)}
          subtitle={t('overview.cumulative')}
          chartData={hourlyData}
          chartColor="var(--color-success)"
        />
        <StatCard
          label={t('overview.successRate')}
          value={`${successRate}%`}
          subtitle={t('overview.trend')}
        />
        <StatCard
          label={t('overview.avgLatency')}
          value={stats?.health.latency_ms != null ? `${stats.health.latency_ms}ms` : '-'}
          subtitle={t('overview.trend')}
        />
      </div>

      <Card className={styles.chartCard}>
        <TrendChart
          labels={chartLabels}
          datasets={[{ label: t('overview.totalRequests'), data: chartValues }]}
        />
      </Card>

      <Card className={styles.healthCard}>
        <h3 className={styles.sectionTitle}>{t('overview.endpointHealth')}</h3>
        <div className={styles.healthGrid}>
          {endpoints.map((ep) => (
            <div key={ep.name} className={styles.healthItem}>
              <span className={`${styles.dot} ${ep.healthy ? styles.online : styles.offline}`} />
              <span className={styles.epName}>{ep.name}</span>
              <span className={styles.epLatency}>{ep.avg_latency_ms.toFixed(0)}ms</span>
            </div>
          ))}
        </div>
      </Card>
    </div>
  )
}
```

- [ ] **Step 2: Create Overview.module.scss**

`src/pages/Overview.module.scss`:
```scss
@use '../styles/mixins' as *;

.page {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.controls {
  display: flex;
  align-items: flex-end;
  gap: 16px;
  flex-wrap: wrap;
}

.statsGrid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;

  @include responsive {
    grid-template-columns: repeat(2, 1fr);
  }
}

.chartCard {
  padding: 24px;
}

.healthCard {
  padding: 24px;
}

.sectionTitle {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 16px;
}

.healthGrid {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.healthItem {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.online { background: var(--color-success); }
.offline { background: var(--color-error); }

.epName {
  flex: 1;
  font-size: 14px;
  font-weight: 500;
}

.epLatency {
  font-size: 13px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}
```

- [ ] **Step 3: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(frontend): add Overview page with stats cards, trend chart, endpoint health"
```

---

## Task 10: Analysis Page

**Files:**
- Create: `src/pages/Analysis.tsx`, `src/pages/Analysis.module.scss`

- [ ] **Step 1: Create Analysis page**

`src/pages/Analysis.tsx`:
```tsx
import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useStatsStore } from '@/stores/useStatsStore'
import { useEndpointStore } from '@/stores/useEndpointStore'
import { Card } from '@/components/ui/Card/Card'
import { Select } from '@/components/ui/Select/Select'
import { Heatmap } from '@/components/charts/Heatmap'
import { ErrorTrend } from '@/components/charts/ErrorTrend'
import { TrendChart } from '@/components/charts/TrendChart'
import { EmptyState } from '@/components/ui/EmptyState/EmptyState'
import styles from './Analysis.module.scss'

const HOURS = Array.from({ length: 24 }, (_, i) => String(i))
const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']

export function Analysis() {
  const { t } = useTranslation()
  const {
    heatmapData, errorTrend, langStats, days,
    fetchHeatmap, fetchErrorTrend, fetchLangStats,
  } = useStatsStore()
  const { endpoints, fetchUpstreamStatus } = useEndpointStore()

  useEffect(() => {
    fetchHeatmap('weekday', 30)
    fetchErrorTrend(7)
    fetchLangStats(days)
    fetchUpstreamStatus()
  }, [fetchHeatmap, fetchErrorTrend, fetchLangStats, fetchUpstreamStatus, days])

  const onHeatmapViewChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    fetchHeatmap(e.target.value, 30)
  }

  const onErrorDaysChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    fetchErrorTrend(Number(e.target.value))
  }

  return (
    <div className={styles.page}>
      {/* 热力图 */}
      <Card>
        <div className={styles.cardHeader}>
          <h3>{t('analysis.heatmap')}</h3>
          <Select
            value={heatmapData?.view ?? 'weekday'}
            onChange={onHeatmapViewChange}
            options={[
              { value: 'weekday', label: t('analysis.weekday') },
              { value: 'hourly', label: t('analysis.hourly') },
            ]}
          />
        </div>
        {heatmapData && heatmapData.data.length > 0 ? (
          <Heatmap
            data={heatmapData.data}
            xLabels={HOURS}
            yLabels={heatmapData.view === 'weekday' ? WEEKDAYS : HOURS}
          />
        ) : (
          <EmptyState />
        )}
      </Card>

      {/* 错误趋势 */}
      <Card>
        <div className={styles.cardHeader}>
          <h3>{t('analysis.errorTrend')}</h3>
          <Select
            value="7"
            onChange={onErrorDaysChange}
            options={[
              { value: 7, label: '7d' },
              { value: 14, label: '14d' },
              { value: 30, label: '30d' },
            ]}
          />
        </div>
        {errorTrend.length > 0 ? (
          <ErrorTrend
            labels={errorTrend.map((p) => p.time)}
            errorRates={errorTrend.map((p) => p.error_rate * 100)}
          />
        ) : (
          <EmptyState />
        )}
      </Card>

      {/* 语言统计 */}
      <Card>
        <h3 className={styles.cardTitle}>{t('analysis.langStats')}</h3>
        {langStats.length > 0 ? (
          <div className={styles.langGrid}>
            {langStats.map((ls) => (
              <div key={ls.lang} className={styles.langItem}>
                <span className={styles.langName}>{ls.lang || 'auto'}</span>
                <div className={styles.langBar}>
                  <div className={styles.langBarFill} style={{ width: `${Math.min((ls.source_chars + ls.target_chars) / Math.max(...langStats.map(l => l.source_chars + l.target_chars)) * 100, 100)}%` }} />
                </div>
                <span className={styles.langCount}>{(ls.source_chars + ls.target_chars).toLocaleString()}</span>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState />
        )}
      </Card>

      {/* 端点对比 */}
      <Card>
        <h3 className={styles.cardTitle}>{t('analysis.endpointCompare')}</h3>
        {endpoints.length > 1 ? (
          <TrendChart
            labels={endpoints.map((ep) => ep.name)}
            datasets={[
              {
                label: t('endpoints.latency'),
                data: endpoints.map((ep) => ep.avg_latency_ms),
                borderColor: 'var(--color-primary)',
              },
            ]}
            height={200}
          />
        ) : (
          <EmptyState />
        )}
      </Card>
    </div>
  )
}
```

- [ ] **Step 2: Create Analysis.module.scss**

`src/pages/Analysis.module.scss`:
```scss
@use '../styles/mixins' as *;

.page {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.cardHeader {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;

  h3 {
    font-size: 15px;
    font-weight: 600;
  }
}

.cardTitle {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 16px;
}

.langGrid {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.langItem {
  display: flex;
  align-items: center;
  gap: 12px;
}

.langName {
  width: 60px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
  text-align: right;
}

.langBar {
  flex: 1;
  height: 8px;
  background: var(--bg-hover);
  border-radius: 4px;
  overflow: hidden;
}

.langBarFill {
  height: 100%;
  background: var(--color-primary);
  border-radius: 4px;
  transition: width 300ms ease;
}

.langCount {
  font-size: 12px;
  color: var(--text-tertiary);
  font-variant-numeric: tabular-nums;
  min-width: 80px;
  text-align: right;
}
```

- [ ] **Step 3: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(frontend): add Analysis page with heatmap, error trend, lang stats, endpoint comparison"
```

---

## Task 11: Endpoints Page

**Files:**
- Create: `src/pages/Endpoints.tsx`, `src/pages/Endpoints.module.scss`

- [ ] **Step 1: Create Endpoints page**

`src/pages/Endpoints.tsx`:
```tsx
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useEndpointStore } from '@/stores/useEndpointStore'
import { Card } from '@/components/ui/Card/Card'
import { Button } from '@/components/ui/Button/Button'
import { Modal } from '@/components/ui/Modal/Modal'
import { EmptyState } from '@/components/ui/EmptyState/EmptyState'
import styles from './Endpoints.module.scss'

export function Endpoints() {
  const { t } = useTranslation()
  const {
    endpoints, cacheStats, cacheHitLogs,
    fetchUpstreamStatus, fetchCacheStats, fetchCacheHitLogs,
    clearCache, triggerHealthCheck,
  } = useEndpointStore()

  const [confirmClear, setConfirmClear] = useState(false)
  const [checking, setChecking] = useState(false)

  useEffect(() => {
    fetchUpstreamStatus()
    fetchCacheStats()
    fetchCacheHitLogs()
  }, [fetchUpstreamStatus, fetchCacheStats, fetchCacheHitLogs])

  const onHealthCheck = async () => {
    setChecking(true)
    await triggerHealthCheck()
    setChecking(false)
  }

  const onClearCache = async () => {
    await clearCache()
    setConfirmClear(false)
  }

  return (
    <div className={styles.page}>
      {/* 端点列表 */}
      <div className={styles.header}>
        <h3>{t('endpoints.title')}</h3>
        <Button variant="secondary" size="sm" onClick={onHealthCheck} disabled={checking}>
          {t('endpoints.healthCheck')}
        </Button>
      </div>

      {endpoints.length > 0 ? (
        <div className={styles.epGrid}>
          {endpoints.map((ep) => (
            <Card key={ep.name} className={styles.epCard}>
              <div className={styles.epHeader}>
                <span className={`${styles.badge} ${ep.healthy ? styles.online : styles.offline}`}>
                  {ep.healthy ? t('endpoints.online') : t('endpoints.offline')}
                </span>
                <span className={styles.epName}>{ep.name}</span>
              </div>
              <div className={styles.epUrl}>{ep.url}</div>
              <div className={styles.epStats}>
                <div className={styles.epStat}>
                  <span className={styles.epStatLabel}>{t('endpoints.latency')}</span>
                  <span className={styles.epStatValue}>{ep.avg_latency_ms.toFixed(0)}ms</span>
                </div>
                <div className={styles.epStat}>
                  <span className={styles.epStatLabel}>{t('endpoints.successRate')}</span>
                  <span className={styles.epStatValue}>
                    {ep.total_requests > 0 ? ((ep.total_successes / ep.total_requests) * 100).toFixed(1) : '0'}%
                  </span>
                </div>
                <div className={styles.epStat}>
                  <span className={styles.epStatLabel}>{t('endpoints.requests')}</span>
                  <span className={styles.epStatValue}>{ep.total_requests.toLocaleString()}</span>
                </div>
              </div>
              {ep.last_error && (
                <div className={styles.epError}>{ep.last_error}</div>
              )}
            </Card>
          ))}
        </div>
      ) : (
        <EmptyState />
      )}

      {/* 缓存管理 */}
      <div className={styles.cacheSection}>
        <div className={styles.header}>
          <h3>{t('endpoints.cache')}</h3>
          <Button variant="danger" size="sm" onClick={() => setConfirmClear(true)}>
            {t('endpoints.clearCache')}
          </Button>
        </div>

        {cacheStats && (
          <Card className={styles.cacheCard}>
            <div className={styles.cacheGrid}>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>{t('endpoints.hitRate')}</span>
                <span className={styles.cacheValue}>{(cacheStats.hit_rate * 100).toFixed(1)}%</span>
              </div>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>{t('endpoints.size')}</span>
                <span className={styles.cacheValue}>{cacheStats.size} / {cacheStats.max_entries}</span>
              </div>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>TTL</span>
                <span className={styles.cacheValue}>{cacheStats.ttl_secs}s</span>
              </div>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>{cacheStats.enabled ? t('endpoints.enabled') : t('endpoints.disabled')}</span>
                <span className={`${styles.cacheValue} ${cacheStats.enabled ? styles.successText : styles.errorText}`}>
                  {cacheStats.enabled ? '●' : '○'}
                </span>
              </div>
            </div>
          </Card>
        )}

        {cacheHitLogs.length > 0 && (
          <Card>
            <h4 className={styles.subTitle}>{t('endpoints.cacheHits')}</h4>
            <div className={styles.hitList}>
              {cacheHitLogs.slice(0, 20).map((hit, i) => (
                <div key={i} className={styles.hitItem}>
                  <span className={styles.hitLang}>{hit.source_lang}→{hit.target_lang}</span>
                  <span className={styles.hitText}>{hit.text_preview}</span>
                  <span className={styles.hitTime}>{hit.timestamp}</span>
                </div>
              ))}
            </div>
          </Card>
        )}
      </div>

      <Modal open={confirmClear} onClose={() => setConfirmClear(false)} title={t('settings.confirmDelete')}>
        <p style={{ marginBottom: 16 }}>{t('endpoints.clearCache')}?</p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={() => setConfirmClear(false)}>{t('common.cancel')}</Button>
          <Button variant="danger" onClick={onClearCache}>{t('common.confirm')}</Button>
        </div>
      </Modal>
    </div>
  )
}
```

- [ ] **Step 2: Create Endpoints.module.scss**

`src/pages/Endpoints.module.scss`:
```scss
@use '../styles/mixins' as *;

.page {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;

  h3 { font-size: 16px; font-weight: 600; }
}

.epGrid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
}

.epCard { padding: 20px; }

.epHeader {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.badge {
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
}

.online { background: var(--color-success-bg); color: var(--color-success); }
.offline { background: var(--color-error-bg); color: var(--color-error); }

.epName { font-size: 15px; font-weight: 600; }

.epUrl {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-bottom: 12px;
  word-break: break-all;
}

.epStats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}

.epStat { text-align: center; }

.epStatLabel {
  display: block;
  font-size: 11px;
  color: var(--text-tertiary);
  margin-bottom: 2px;
}

.epStatValue {
  font-size: 14px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.epError {
  margin-top: 10px;
  padding: 8px 10px;
  background: var(--color-error-bg);
  border-radius: var(--radius-sm);
  font-size: 12px;
  color: var(--color-error);
}

.cacheSection { margin-top: 8px; display: flex; flex-direction: column; gap: 16px; }
.cacheCard { padding: 20px; }

.cacheGrid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;

  @include responsive { grid-template-columns: repeat(2, 1fr); }
}

.cacheStat { text-align: center; }
.cacheLabel { display: block; font-size: 12px; color: var(--text-tertiary); margin-bottom: 4px; }
.cacheValue { font-size: 16px; font-weight: 700; }
.successText { color: var(--color-success); }
.errorText { color: var(--color-error); }

.subTitle { font-size: 14px; font-weight: 600; margin-bottom: 12px; }

.hitList { display: flex; flex-direction: column; gap: 6px; }

.hitItem {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
  font-size: 13px;
}

.hitLang { font-weight: 500; min-width: 70px; }
.hitText { flex: 1; color: var(--text-secondary); @include truncate; }
.hitTime { font-size: 11px; color: var(--text-tertiary); white-space: nowrap; }
```

- [ ] **Step 3: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(frontend): add Endpoints page with status cards, cache management"
```

---

## Task 12: Logs Page

**Files:**
- Create: `src/pages/Logs.tsx`, `src/pages/Logs.module.scss`

- [ ] **Step 1: Create Logs page**

`src/pages/Logs.tsx`:
```tsx
import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useLogsStore } from '@/stores/useLogsStore'
import { useEndpointStore } from '@/stores/useEndpointStore'
import { Card } from '@/components/ui/Card/Card'
import { Select } from '@/components/ui/Select/Select'
import { Button } from '@/components/ui/Button/Button'
import { EmptyState } from '@/components/ui/EmptyState/EmptyState'
import { LoadingSpinner } from '@/components/ui/LoadingSpinner/LoadingSpinner'
import styles from './Logs.module.scss'

export function Logs() {
  const { t } = useTranslation()
  const {
    logs, total, page, pageSize, loading,
    filterStatus, filterEndpoint, filterLang,
    setFilter, fetchLogs, exportLogs,
  } = useLogsStore()
  const { endpoints, fetchUpstreamStatus } = useEndpointStore()

  useEffect(() => {
    fetchLogs()
    fetchUpstreamStatus()
  }, [fetchLogs, fetchUpstreamStatus])

  const totalPages = Math.ceil(total / pageSize)

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h3>{t('logs.title')}</h3>
        <Button variant="secondary" size="sm" onClick={() => exportLogs()}>
          {t('settings.exportCsv')}
        </Button>
      </div>

      <div className={styles.filters}>
        <Select
          label={t('logs.status')}
          value={filterStatus}
          onChange={(e) => setFilter('filterStatus', e.target.value)}
          options={[
            { value: '', label: t('logs.all') },
            { value: 'success', label: t('logs.success') },
            { value: 'error', label: t('logs.failed') },
          ]}
        />
        <Select
          label={t('logs.endpoint')}
          value={filterEndpoint}
          onChange={(e) => setFilter('filterEndpoint', e.target.value)}
          options={[
            { value: '', label: t('logs.all') },
            ...endpoints.map((ep) => ({ value: ep.name, label: ep.name })),
          ]}
        />
      </div>

      <Card padding="sm">
        {loading ? (
          <div className={styles.loadingWrap}><LoadingSpinner /></div>
        ) : logs.length > 0 ? (
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>{t('logs.time')}</th>
                  <th>{t('logs.sourceLang')}</th>
                  <th>{t('logs.targetLang')}</th>
                  <th>{t('logs.endpoint')}</th>
                  <th>{t('logs.latency')}</th>
                  <th>{t('logs.status')}</th>
                </tr>
              </thead>
              <tbody>
                {logs.map((log) => (
                  <tr key={log.id}>
                    <td className={styles.mono}>{log.created_at}</td>
                    <td>{log.source_lang || 'auto'}</td>
                    <td>{log.target_lang}</td>
                    <td>{log.endpoint_name ?? '-'}</td>
                    <td className={styles.mono}>{log.latency_ms != null ? `${log.latency_ms}ms` : '-'}</td>
                    <td>
                      <span className={`${styles.statusBadge} ${log.status === 'success' ? styles.success : styles.error}`}>
                        {log.status === 'success' ? t('logs.success') : t('logs.failed')}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <EmptyState />
        )}
      </Card>

      {totalPages > 1 && (
        <div className={styles.pagination}>
          <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => fetchLogs(page - 1)}>
            ←
          </Button>
          <span className={styles.pageInfo}>{page} / {totalPages}</span>
          <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => fetchLogs(page + 1)}>
            →
          </Button>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Create Logs.module.scss**

`src/pages/Logs.module.scss`:
```scss
@use '../styles/mixins' as *;

.page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;

  h3 { font-size: 16px; font-weight: 600; }
}

.filters {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.loadingWrap {
  display: flex;
  justify-content: center;
  padding: 48px;
}

.tableWrap {
  overflow-x: auto;
}

.table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;

  th, td {
    padding: 10px 12px;
    text-align: left;
    border-bottom: 1px solid var(--border-color);
  }

  th {
    font-weight: 600;
    color: var(--text-secondary);
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    background: var(--bg-secondary);
  }

  tbody tr:hover {
    background: var(--bg-hover);
  }
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 12px;
}

.statusBadge {
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 600;
}

.success { background: var(--color-success-bg); color: var(--color-success); }
.error { background: var(--color-error-bg); color: var(--color-error); }

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
}

.pageInfo {
  font-size: 13px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}
```

- [ ] **Step 3: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(frontend): add Logs page with table, filters, pagination, export"
```

---

## Task 13: Settings Page

**Files:**
- Create: `src/pages/Settings.tsx`, `src/pages/Settings.module.scss`

- [ ] **Step 1: Create Settings page**

`src/pages/Settings.tsx`:
```tsx
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useConfigStore } from '@/stores/useConfigStore'
import { useLogsStore } from '@/stores/useLogsStore'
import { Card } from '@/components/ui/Card/Card'
import { Button } from '@/components/ui/Button/Button'
import { Input } from '@/components/ui/Input/Input'
import { Modal } from '@/components/ui/Modal/Modal'
import type { FullConfig, FullEndpointInfo } from '@/types'
import styles from './Settings.module.scss'

export function Settings() {
  const { t } = useTranslation()
  const { config, fetchConfig, updateConfig } = useConfigStore()
  const { exportLogs } = useLogsStore()
  const [draft, setDraft] = useState<FullConfig | null>(null)
  const [saving, setSaving] = useState(false)
  const [toast, setToast] = useState('')
  const [deleteIdx, setDeleteIdx] = useState<number | null>(null)

  useEffect(() => {
    fetchConfig()
  }, [fetchConfig])

  useEffect(() => {
    if (config) setDraft(structuredClone(config))
  }, [config])

  if (!draft) return null

  const save = async () => {
    setSaving(true)
    const ok = await updateConfig(draft)
    setSaving(false)
    if (ok) {
      setToast(t('settings.saved'))
      setTimeout(() => setToast(''), 2000)
    }
  }

  const updateEndpoint = (idx: number, field: keyof FullEndpointInfo, value: string) => {
    const eps = [...draft.upstream.endpoints]
    eps[idx] = { ...eps[idx], [field]: value }
    setDraft({ ...draft, upstream: { ...draft.upstream, endpoints: eps } })
  }

  const addEndpoint = () => {
    const eps = [...draft.upstream.endpoints, { name: '', url: '', api_key: '' }]
    setDraft({ ...draft, upstream: { ...draft.upstream, endpoints: eps } })
  }

  const removeEndpoint = (idx: number) => {
    const eps = draft.upstream.endpoints.filter((_, i) => i !== idx)
    setDraft({ ...draft, upstream: { ...draft.upstream, endpoints: eps } })
    setDeleteIdx(null)
  }

  return (
    <div className={styles.page}>
      {toast && <div className={styles.toast}>{toast}</div>}

      {/* 上游端点 */}
      <Card>
        <div className={styles.sectionHeader}>
          <h3>{t('settings.upstream')}</h3>
          <Button variant="secondary" size="sm" onClick={addEndpoint}>{t('settings.add')}</Button>
        </div>
        <div className={styles.epList}>
          {draft.upstream.endpoints.map((ep, i) => (
            <div key={i} className={styles.epRow}>
              <Input label={t('settings.name')} value={ep.name} onChange={(e) => updateEndpoint(i, 'name', e.target.value)} />
              <Input label={t('settings.url')} value={ep.url} onChange={(e) => updateEndpoint(i, 'url', e.target.value)} />
              <Input label={t('settings.apiKey')} value={ep.api_key} onChange={(e) => updateEndpoint(i, 'api_key', e.target.value)} type="password" />
              <Button variant="danger" size="sm" onClick={() => setDeleteIdx(i)}>{t('settings.delete')}</Button>
            </div>
          ))}
        </div>
        <div className={styles.fieldRow}>
          <Input
            label="max_failures"
            type="number"
            value={draft.upstream.max_failures}
            onChange={(e) => setDraft({ ...draft, upstream: { ...draft.upstream, max_failures: Number(e.target.value) } })}
          />
          <Input
            label="probe_interval_secs"
            type="number"
            value={draft.upstream.probe_interval_secs}
            onChange={(e) => setDraft({ ...draft, upstream: { ...draft.upstream, probe_interval_secs: Number(e.target.value) } })}
          />
        </div>
      </Card>

      {/* 代理设置 */}
      <Card>
        <h3 className={styles.sectionTitle}>{t('settings.proxy')} <span className={styles.hint}>({t('settings.restartRequired')})</span></h3>
        <div className={styles.fieldRow}>
          <Input label={t('settings.host')} value={draft.proxy.host} onChange={(e) => setDraft({ ...draft, proxy: { ...draft.proxy, host: e.target.value } })} />
          <Input label={t('settings.port')} type="number" value={draft.proxy.port} onChange={(e) => setDraft({ ...draft, proxy: { ...draft.proxy, port: Number(e.target.value) } })} />
        </div>
      </Card>

      {/* 监控设置 */}
      <Card>
        <h3 className={styles.sectionTitle}>{t('settings.monitor')}</h3>
        <div className={styles.fieldRow}>
          <Input label={t('settings.logRetention')} type="number" value={draft.monitor.max_log_entries} onChange={(e) => setDraft({ ...draft, monitor: { ...draft.monitor, max_log_entries: Number(e.target.value) } })} />
          <Input label="auto_refresh_seconds" type="number" value={draft.monitor.auto_refresh_seconds} onChange={(e) => setDraft({ ...draft, monitor: { ...draft.monitor, auto_refresh_seconds: Number(e.target.value) } })} />
        </div>
      </Card>

      {/* 缓存设置 */}
      <Card>
        <h3 className={styles.sectionTitle}>{t('settings.cacheConfig')}</h3>
        <div className={styles.fieldRow}>
          <label className={styles.checkbox}>
            <input type="checkbox" checked={draft.cache.enabled} onChange={(e) => setDraft({ ...draft, cache: { ...draft.cache, enabled: e.target.checked } })} />
            {t('endpoints.enabled')}
          </label>
          <Input label={t('settings.ttl')} type="number" value={draft.cache.ttl_secs} onChange={(e) => setDraft({ ...draft, cache: { ...draft.cache, ttl_secs: Number(e.target.value) } })} />
          <Input label={t('settings.maxEntries')} type="number" value={draft.cache.max_entries} onChange={(e) => setDraft({ ...draft, cache: { ...draft.cache, max_entries: Number(e.target.value) } })} />
          <Input label={t('settings.maxMemory')} type="number" value={draft.cache.max_memory_mb} onChange={(e) => setDraft({ ...draft, cache: { ...draft.cache, max_memory_mb: Number(e.target.value) } })} />
        </div>
      </Card>

      {/* 健康检查 */}
      <Card>
        <h3 className={styles.sectionTitle}>{t('settings.healthCheck')}</h3>
        <div className={styles.fieldRow}>
          <Input label="source_lang" value={draft.health_check.source_lang} onChange={(e) => setDraft({ ...draft, health_check: { ...draft.health_check, source_lang: e.target.value } })} />
          <Input label="target_lang" value={draft.health_check.target_lang} onChange={(e) => setDraft({ ...draft, health_check: { ...draft.health_check, target_lang: e.target.value } })} />
        </div>
      </Card>

      {/* 导出 */}
      <Card>
        <h3 className={styles.sectionTitle}>{t('settings.export')}</h3>
        <Button variant="secondary" onClick={() => exportLogs()}>{t('settings.exportCsv')}</Button>
      </Card>

      {/* 保存按钮 */}
      <div className={styles.saveBar}>
        <Button variant="primary" onClick={save} disabled={saving}>{t('settings.save')}</Button>
      </div>

      <Modal open={deleteIdx !== null} onClose={() => setDeleteIdx(null)} title={t('settings.confirmDelete')}>
        <p style={{ marginBottom: 16 }}>{t('settings.confirmDelete')}</p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={() => setDeleteIdx(null)}>{t('common.cancel')}</Button>
          <Button variant="danger" onClick={() => deleteIdx !== null && removeEndpoint(deleteIdx)}>{t('common.confirm')}</Button>
        </div>
      </Modal>
    </div>
  )
}
```

- [ ] **Step 2: Create Settings.module.scss**

`src/pages/Settings.module.scss`:
```scss
@use '../styles/mixins' as *;

.page {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.sectionHeader {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;

  h3 { font-size: 15px; font-weight: 600; }
}

.sectionTitle {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 16px;
}

.hint {
  font-size: 12px;
  font-weight: 400;
  color: var(--color-warning);
}

.epList {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 16px;
}

.epRow {
  display: grid;
  grid-template-columns: 1fr 2fr 1.5fr auto;
  gap: 10px;
  align-items: flex-end;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);

  @include responsive {
    grid-template-columns: 1fr;
  }
}

.fieldRow {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;

  > * { flex: 1; min-width: 140px; }
}

.checkbox {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  cursor: pointer;

  input { width: 16px; height: 16px; }
}

.saveBar {
  position: sticky;
  bottom: 16px;
  display: flex;
  justify-content: flex-end;
  padding: 12px 0;
}

.toast {
  position: fixed;
  top: 80px;
  right: 16px;
  padding: 10px 20px;
  background: var(--color-success-bg);
  color: var(--color-success);
  border-radius: var(--radius-sm);
  font-size: 14px;
  font-weight: 500;
  z-index: 1000;
  animation: slideIn 200ms ease;
}

@keyframes slideIn {
  from { transform: translateX(100%); opacity: 0; }
  to { transform: translateX(0); opacity: 1; }
}
```

- [ ] **Step 3: Verify build**

```bash
cd frontend && npx tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(frontend): add Settings page with full config CRUD"
```

---

## Task 14: Wire Up App.tsx & Final Integration

**Files:**
- Rewrite: `src/App.tsx`
- Modify: `src/main.tsx` (add theme init)

- [ ] **Step 1: Rewrite App.tsx with tab routing**

`src/App.tsx`:
```tsx
import { useState, useEffect } from 'react'
import { useThemeStore } from '@/stores/useThemeStore'
import { useStatsStore } from '@/stores/useStatsStore'
import { TopBar } from '@/components/layout/TopBar'
import { PageShell } from '@/components/layout/PageShell'
import { ToastContainer } from '@/components/shared/Toast'
import { Overview } from '@/pages/Overview'
import { Analysis } from '@/pages/Analysis'
import { Endpoints } from '@/pages/Endpoints'
import { Logs } from '@/pages/Logs'
import { Settings } from '@/pages/Settings'
import type { Tab } from '@/types'

function getInitialTab(): Tab {
  const hash = window.location.hash.slice(1)
  const valid: Tab[] = ['overview', 'analysis', 'endpoints', 'logs', 'settings']
  return valid.includes(hash as Tab) ? (hash as Tab) : 'overview'
}

function App() {
  const [activeTab, setActiveTab] = useState<Tab>(getInitialTab)
  const { theme } = useThemeStore()
  const { fetchVersion } = useStatsStore()

  useEffect(() => {
    fetchVersion()
  }, [fetchVersion])

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
  }, [theme])

  const onTabChange = (tab: Tab) => {
    setActiveTab(tab)
    window.location.hash = tab
  }

  useEffect(() => {
    const onHash = () => {
      const hash = window.location.hash.slice(1) as Tab
      const valid: Tab[] = ['overview', 'analysis', 'endpoints', 'logs', 'settings']
      if (valid.includes(hash)) setActiveTab(hash)
    }
    window.addEventListener('hashchange', onHash)
    return () => window.removeEventListener('hashchange', onHash)
  }, [])

  const renderPage = () => {
    switch (activeTab) {
      case 'overview': return <Overview />
      case 'analysis': return <Analysis />
      case 'endpoints': return <Endpoints />
      case 'logs': return <Logs />
      case 'settings': return <Settings />
    }
  }

  return (
    <>
      <TopBar activeTab={activeTab} onTabChange={onTabChange} />
      <PageShell>{renderPage()}</PageShell>
      <ToastContainer />
    </>
  )
}

export default App
```

- [ ] **Step 2: Verify full build**

```bash
cd frontend && npm run build
```

Expected: Build succeeds, `../dist/index.html` exists with React app.

- [ ] **Step 3: Start dev server and verify in browser**

```bash
cd frontend && npm run dev
```

Open http://localhost:5173 — verify:
- TopBar renders with tabs and theme/language switchers
- Tab switching works (URL hash updates)
- Theme toggle switches between light/dark
- Language toggle switches between zh/en

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(frontend): wire up App with tab routing, complete React rewrite"
```

---

## Task 15: Update Dockerfile & Documentation

**Files:**
- Modify: `Dockerfile` (Node stage: remove vue-tsc, use tsc)
- Modify: `CLAUDE.md` (update frontend tech stack description)

- [ ] **Step 1: Update Dockerfile Node build stage**

In the Dockerfile, the Node build stage likely has `npm run build` which previously ran `vue-tsc --noEmit && vite build`. The new `package.json` uses `tsc --noEmit && vite build` — same command name, so no Dockerfile change needed unless it references vue-tsc directly.

Verify:
```bash
grep -n "vue-tsc\|npm run build" Dockerfile
```

If Dockerfile only uses `npm run build`, no change needed. If it references `vue-tsc` directly, replace with `tsc`.

- [ ] **Step 2: Update CLAUDE.md frontend description**

Change the Tech Stack section to reflect React:
```
- **Frontend**: React 19 + TypeScript + Vite, Zustand for state, Chart.js + Recharts for charts, SCSS Modules, react-i18next (single-page dashboard with 5 tabs)
```

- [ ] **Step 3: Remove tsconfig.node.json (no longer needed)**

```bash
rm -f frontend/tsconfig.node.json
```

- [ ] **Step 4: Final production build test**

```bash
cd frontend && npm run build
ls -la ../dist/index.html
```

Expected: Build succeeds, dist contains index.html and assets.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: update Dockerfile and docs for React frontend"
```

---

## Self-Review Checklist

- [x] **Spec coverage**: All 5 tabs implemented (Overview, Analysis, Endpoints, Logs, Settings). Theme switching, i18n, auto-refresh, all API endpoints consumed.
- [x] **Placeholder scan**: No TBD/TODO in any task. All code blocks are complete.
- [x] **Type consistency**: `Tab` type used consistently. Store method names match across tasks (`fetchStats`, `fetchUpstreamStatus`, etc.). Component props match their usage in pages.
- [x] **Build output**: `../dist/` path preserved. Vite proxy config preserved. Backend requires zero changes.
