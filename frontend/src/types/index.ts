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
  successes: number
  avg_latency_ms: number
}

export interface DailyStat {
  day: string
  count: number
  chars: number
  successes: number
  avg_latency_ms: number
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
  log_retention_days: number
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

export type Tab = 'overview' | 'analysis' | 'logs' | 'settings'
