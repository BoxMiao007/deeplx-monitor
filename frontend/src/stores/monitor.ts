import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

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

export interface ChartData {
  hourly: HourlyStat[]
  daily: DailyStat[]
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
  size: number
  max_entries: number
  ttl_secs: number
  hit_rate: number
  max_memory_mb: number
  estimated_memory_bytes: number
}

export interface CacheHitEntry {
  source_lang: string
  target_lang: string
  text_preview: string
  timestamp: string
}

// 完整配置接口
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

export const useMonitorStore = defineStore('monitor', () => {
  const appVersion = ref('')
  const stats = ref<StatsResponse | null>(null)
  const chartData = ref<ChartData | null>(null)
  const langStats = ref<LangStat[]>([])
  const langHourlyStats = ref<LangHourlyUsage[]>([])
  const requests = ref<RequestsResponse | null>(null)
  const upstreamStatus = ref<EndpointStatus[]>([])
  const cacheStats = ref<CacheStats | null>(null)
  const cacheHitLogs = ref<CacheHitEntry[]>([])
  const heatmapData = ref<HeatmapResponse | null>(null)
  const errorTrend = ref<ErrorTrendPoint[]>([])
  const fullConfig = ref<FullConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const autoRefreshInterval = ref<ReturnType<typeof setInterval> | null>(null)

  const healthStatus = computed(() => stats.value?.health ?? { status: 'unknown', latency_ms: null, checked_at: null, error: null })
  const config = computed(() => stats.value?.config ?? { upstream_url: '', api_key: '', auto_refresh_seconds: 0 })
  const period = computed(() => stats.value?.period ?? { today_requests: 0, today_chars: 0, week_requests: 0, week_chars: 0, month_requests: 0, month_chars: 0 })

  function rangeToDays(range: string): number | null {
    if (range === 'today') return 1
    if (range === 'week') return 7
    if (range === 'month') return 30
    return null
  }

  async function fetchLangStats(range = 'all') {
    try {
      error.value = null
      const params = new URLSearchParams()
      const days = rangeToDays(range)
      if (days !== null) params.set('days', String(days))
      const query = params.toString()
      const res = await fetch(query ? `/api/lang-stats?${query}` : '/api/lang-stats')
      if (!res.ok) throw new Error('获取语言统计失败')
      langStats.value = await res.json()
    } catch (e) {
      error.value = (e as Error).message
    }
  }

  async function fetchLangHourlyStats(days = 1) {
    try {
      error.value = null
      const res = await fetch(days > 1 ? `/api/lang-hourly-stats?days=${days}` : '/api/lang-hourly-stats')
      if (!res.ok) throw new Error('获取语言趋势失败')
      langHourlyStats.value = await res.json()
    } catch (e) {
      error.value = (e as Error).message
    }
  }

  async function fetchStats(range = 'all') {
    try {
      error.value = null
      const params = new URLSearchParams()
      const days = rangeToDays(range)
      if (days !== null) params.set('days', String(days))
      const query = params.toString()
      const res = await fetch(query ? `/api/stats?${query}` : '/api/stats')
      if (!res.ok) throw new Error('获取统计失败')
      stats.value = await res.json()
    } catch (e) {
      error.value = (e as Error).message
    }
  }

  async function fetchChart() {
    try {
      error.value = null
      const res = await fetch('/api/chart')
      if (!res.ok) throw new Error('获取图表数据失败')
      chartData.value = await res.json()
    } catch (e) {
      error.value = (e as Error).message
    }
  }

  async function fetchRequests(page = 1, pageSize = 50) {
    loading.value = true
    try {
      error.value = null
      const params = new URLSearchParams({ page: String(page), page_size: String(pageSize) })
      const res = await fetch(`/api/requests?${params}`)
      if (!res.ok) throw new Error('获取请求列表失败')
      requests.value = await res.json()
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  async function triggerHealthCheck() {
    try {
      error.value = null
      const res = await fetch('/api/health/check', { method: 'POST' })
      if (!res.ok) throw new Error('健康检查失败')
      const health = await res.json()
      if (stats.value) {
        stats.value.health = health
      }
      // 刷新端点状态卡片，使 UI 立即反映探测结果
      await fetchUpstreamStatus()
    } catch (e) {
      error.value = (e as Error).message
    }
  }

  async function fetchConfig() {
    try {
      error.value = null
      const res = await fetch('/api/config')
      if (!res.ok) throw new Error('获取配置失败')
      const cfg: FullConfig = await res.json()
      fullConfig.value = cfg
      // 同步到 stats.config 以保持 header 控件兼容
      if (stats.value) {
        stats.value.config = {
          upstream_url: cfg.upstream.endpoints[0]?.url ?? '',
          api_key: cfg.upstream.endpoints[0]?.api_key ?? '',
          auto_refresh_seconds: cfg.monitor.auto_refresh_seconds,
        }
      }
      return cfg
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function fetchFullConfig() {
    return fetchConfig()
  }

  async function updateConfig(payload: Partial<ConfigInfo> & { api_key?: string }) {
    try {
      error.value = null
      // 将旧格式转换为新的完整配置格式
      const body: Record<string, unknown> = {}
      if (payload.upstream_url !== undefined || payload.api_key !== undefined) {
        const currentEndpoints = fullConfig.value?.upstream.endpoints ?? []
        const firstEp = currentEndpoints[0] ?? { name: 'default', url: '', api_key: '' }
        body.upstream = {
          endpoints: [{
            name: firstEp.name || 'default',
            url: payload.upstream_url ?? firstEp.url,
            api_key: payload.api_key ?? firstEp.api_key,
          }, ...currentEndpoints.slice(1)],
          max_failures: fullConfig.value?.upstream.max_failures ?? 3,
          probe_interval_secs: fullConfig.value?.upstream.probe_interval_secs ?? 60,
        }
      }
      if (payload.auto_refresh_seconds !== undefined) {
        body.monitor = {
          auto_refresh_seconds: payload.auto_refresh_seconds,
          max_log_entries: fullConfig.value?.monitor.max_log_entries ?? 10000,
        }
      }
      const res = await fetch('/api/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      })
      if (!res.ok) throw new Error('保存配置失败')
      const cfg: FullConfig = await res.json()
      fullConfig.value = cfg
      if (stats.value) {
        stats.value.config = {
          upstream_url: cfg.upstream.endpoints[0]?.url ?? '',
          api_key: cfg.upstream.endpoints[0]?.api_key ?? '',
          auto_refresh_seconds: cfg.monitor.auto_refresh_seconds,
        }
      }
      return true
    } catch (e) {
      error.value = (e as Error).message
      return false
    }
  }

  async function updateFullConfig(payload: FullConfig) {
    try {
      error.value = null
      const res = await fetch('/api/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      })
      if (!res.ok) throw new Error('保存配置失败')
      const cfg: FullConfig = await res.json()
      fullConfig.value = cfg
      if (stats.value) {
        stats.value.config = {
          upstream_url: cfg.upstream.endpoints[0]?.url ?? '',
          api_key: cfg.upstream.endpoints[0]?.api_key ?? '',
          auto_refresh_seconds: cfg.monitor.auto_refresh_seconds,
        }
      }
      return true
    } catch (e) {
      error.value = (e as Error).message
      return false
    }
  }


  async function fetchUpstreamStatus() {
    try {
      const res = await fetch('/api/upstream/status')
      if (!res.ok) throw new Error('获取上游状态失败')
      upstreamStatus.value = await res.json()
    } catch (e) {
      error.value = (e as Error).message
    }
  }

  async function fetchCacheStats() {
    try {
      const res = await fetch('/api/cache/stats')
      if (!res.ok) throw new Error('获取缓存统计失败')
      cacheStats.value = await res.json()
    } catch (e) {
      error.value = (e as Error).message
    }
  }

  async function clearCache() {
    try {
      error.value = null
      const res = await fetch('/api/cache/clear', { method: 'POST' })
      if (!res.ok) throw new Error('清除缓存失败')
      await fetchCacheStats()
      return true
    } catch (e) {
      error.value = (e as Error).message
      return false
    }
  }

  async function fetchCacheHitLogs() {
    try {
      const res = await fetch('/api/cache/hits')
      if (!res.ok) throw new Error('获取缓存命中日志失败')
      cacheHitLogs.value = await res.json()
    } catch (e) {
      error.value = (e as Error).message
    }
  }

  async function fetchHeatmap(view = 'weekday', days = 30) {
    try {
      const res = await fetch(`/api/analytics/heatmap?view=${view}&days=${days}`)
      if (!res.ok) throw new Error('获取热力图数据失败')
      heatmapData.value = await res.json()
    } catch (e) {
      error.value = (e as Error).message
    }
  }

  async function fetchErrorTrend(days = 7, granularity?: string) {
    try {
      const params = new URLSearchParams({ days: String(days) })
      if (granularity) params.set('granularity', granularity)
      const res = await fetch(`/api/analytics/error-trend?${params}`)
      if (!res.ok) throw new Error('获取错误趋势失败')
      errorTrend.value = await res.json()
    } catch (e) {
      error.value = (e as Error).message
    }
  }

  async function exportLogs(format = 'csv', filters?: { start?: string; end?: string; lang?: string; status?: string }) {
    const params = new URLSearchParams({ format })
    if (filters?.start) params.set('start', filters.start)
    if (filters?.end) params.set('end', filters.end)
    if (filters?.lang) params.set('lang', filters.lang)
    if (filters?.status) params.set('status', filters.status)
    const url = `/api/export?${params}`
    if (format === 'csv') {
      const a = document.createElement('a')
      a.href = url
      a.download = 'export.csv'
      a.click()
    } else {
      window.open(url, '_blank')
    }
  }

  async function fetchVersion() {
    try {
      const res = await fetch('/api/version')
      if (!res.ok) return
      const data = await res.json()
      appVersion.value = data.version ?? ''
    } catch {
      // 版本获取失败不影响功能
    }
  }

  let autoRefreshTimer: ReturnType<typeof setTimeout> | null = null

  function startAutoRefresh(seconds: number, onRefresh?: () => Promise<void>) {
    stopAutoRefresh()
    if (seconds > 0) {
      const refresh = async () => {
        if (onRefresh) {
          await onRefresh()
        } else {
          await Promise.all([fetchStats(), fetchLangHourlyStats(), fetchChart(), fetchRequests(1)])
        }
        autoRefreshTimer = setTimeout(refresh, seconds * 1000)
      }
      autoRefreshTimer = setTimeout(refresh, seconds * 1000)
    }
  }

  function stopAutoRefresh() {
    if (autoRefreshTimer) {
      clearTimeout(autoRefreshTimer)
      autoRefreshTimer = null
    }
    if (autoRefreshInterval.value) {
      clearInterval(autoRefreshInterval.value)
      autoRefreshInterval.value = null
    }
  }

  return {
    appVersion,
    stats,
    chartData,
    langStats,
    langHourlyStats,
    requests,
    upstreamStatus,
    cacheStats,
    heatmapData,
    errorTrend,
    fullConfig,
    loading,
    error,
    healthStatus,
    config,
    period,
    fetchVersion,
    fetchStats,
    fetchLangStats,
    fetchLangHourlyStats,
    fetchChart,
    fetchRequests,
    triggerHealthCheck,
    fetchConfig,
    fetchFullConfig,
    updateConfig,
    updateFullConfig,
    fetchUpstreamStatus,
    fetchCacheStats,
    clearCache,
    cacheHitLogs,
    fetchCacheHitLogs,
    fetchHeatmap,
    fetchErrorTrend,
    exportLogs,
    startAutoRefresh,
    stopAutoRefresh,
  }
})
