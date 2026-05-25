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
