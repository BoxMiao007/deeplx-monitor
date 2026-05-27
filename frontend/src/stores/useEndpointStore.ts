import { create } from 'zustand'
import { apiFetch, apiPost } from '@/hooks/useApi'
import type { EndpointStatus, CacheStats, CacheHitEntry, HealthStatus, StatsResponse } from '@/types'

interface EndpointState {
  endpoints: EndpointStatus[]
  epChars: { label: string; value: number }[]
  cacheStats: CacheStats | null
  cacheHitLogs: CacheHitEntry[]
  loading: boolean

  fetchUpstreamStatus: () => Promise<void>
  fetchEpChars: () => Promise<void>
  fetchCacheStats: () => Promise<void>
  fetchCacheHitLogs: () => Promise<void>
  clearCache: () => Promise<boolean>
  triggerHealthCheck: () => Promise<HealthStatus | null>
}

export const useEndpointStore = create<EndpointState>((set, get) => ({
  endpoints: [],
  epChars: [],
  cacheStats: null,
  cacheHitLogs: [],
  loading: false,

  fetchUpstreamStatus: async () => {
    try {
      const data = await apiFetch<EndpointStatus[]>('/api/upstream/status')
      set({ endpoints: data })
    } catch {}
  },

  fetchEpChars: async () => {
    const { endpoints } = get()
    if (endpoints.length < 2) return
    try {
      const results = await Promise.all(
        endpoints.map(ep => apiFetch<StatsResponse>(`/api/stats?endpoint=${encodeURIComponent(ep.name)}`))
      )
      set({
        epChars: endpoints.map((ep, i) => ({
          label: ep.name,
          value: results[i]?.total_chars ?? 0,
        }))
      })
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
