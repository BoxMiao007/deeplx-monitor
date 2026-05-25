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
