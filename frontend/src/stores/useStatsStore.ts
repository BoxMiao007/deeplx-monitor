import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { apiFetch } from '@/hooks/useApi'
import { timeRangeQuery, type TimeRange } from '@/utils/timeRange'
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
  range: TimeRange
  refreshInterval: number
  appVersion: string

  setRange: (range: TimeRange) => void
  setRefreshInterval: (seconds: number) => void
  fetchStats: (range?: TimeRange, endpoint?: string) => Promise<void>
  fetchChart: (range?: TimeRange, endpoint?: string) => Promise<void>
  fetchLangStats: (range?: TimeRange) => Promise<void>
  fetchLangHourlyStats: (days?: number) => Promise<void>
  fetchHeatmap: (view?: string, days?: number) => Promise<void>
  fetchErrorTrend: (days?: number, granularity?: string) => Promise<void>
  fetchVersion: () => Promise<void>
  fetchAll: (range?: TimeRange) => Promise<void>
}

export const useStatsStore = create<StatsState>()(
  persist(
    (set, get) => ({
  stats: null,
  chartData: null,
  langStats: [],
  langHourlyStats: [],
  heatmapData: null,
  errorTrend: [],
  loading: false,
  error: null,
  range: 'today',
  refreshInterval: 0,
  appVersion: '',

  setRange: (range) => set({ range }),
  setRefreshInterval: (seconds) => set({ refreshInterval: seconds }),

  fetchStats: async (range, endpoint) => {
    try {
      const params = new URLSearchParams()
      const r = range ?? get().range
      new URLSearchParams(timeRangeQuery(r)).forEach((value, key) => params.set(key, value))
      if (endpoint) params.set('endpoint', endpoint)
      const query = params.toString()
      const data = await apiFetch<StatsResponse>(query ? `/api/stats?${query}` : '/api/stats')
      set({ stats: data, error: null })
    } catch (e) {
      set({ error: (e as Error).message })
    }
  },

  fetchChart: async (range, endpoint) => {
    try {
      const params = new URLSearchParams(timeRangeQuery(range ?? get().range))
      if (endpoint) params.set('endpoint', endpoint)
      const data = await apiFetch<ChartData>(`/api/chart?${params.toString()}`)
      set({ chartData: data })
    } catch (e) {
      set({ error: (e as Error).message })
    }
  },

  fetchLangStats: async (range) => {
    try {
      const params = timeRangeQuery(range ?? get().range)
      const data = await apiFetch<LangStat[]>(`/api/lang-stats?${params}`)
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

  fetchAll: async (range) => {
    set({ loading: true })
    const r = range ?? get().range
    await Promise.all([
      get().fetchStats(r),
      get().fetchChart(r),
    ])
    set({ loading: false })
  },
    }),
    {
      name: 'stats-preferences',
      partialize: (state) => ({ refreshInterval: state.refreshInterval, range: state.range }),
    }
  )
)
