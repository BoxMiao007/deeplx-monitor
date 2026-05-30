export type TimeRange = 'today' | '24h' | '7d' | '30d' | '90d' | 'all'
export type HeatmapRange = 'today' | '24h' | '7d'

export function timeRangeQuery(range: TimeRange | HeatmapRange): string {
  return new URLSearchParams({ range }).toString()
}

export function isHourlyRange(range: TimeRange): boolean {
  return range === 'today' || range === '24h'
}

export function startOfRange(range: HeatmapRange, now: Date): Date {
  if (range === 'today') {
    const start = new Date(now)
    start.setHours(0, 0, 0, 0)
    return start
  }
  if (range === '24h') {
    return new Date(now.getTime() - 24 * 60 * 60 * 1000)
  }
  return new Date(now.getTime() - 7 * 86400 * 1000)
}
