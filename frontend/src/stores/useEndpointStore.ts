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
