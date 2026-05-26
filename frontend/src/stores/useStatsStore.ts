import { create } from 'zustand'
import { persist } from 'zustand/middleware'
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
    }),
    {
      name: 'stats-preferences',
      partialize: (state) => ({ refreshInterval: state.refreshInterval, days: state.days }),
    }
  )
)