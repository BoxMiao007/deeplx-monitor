import { useEffect, useCallback, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useStatsStore } from '@/stores/useStatsStore'
import { useEndpointStore } from '@/stores/useEndpointStore'
import { useAutoRefresh } from '@/hooks/useAutoRefresh'
import { apiFetch, showToast } from '@/hooks/useApi'
import { Card } from '@/components/ui/Card/Card'
import { Select } from '@/components/ui/Select/Select'
import { Button } from '@/components/ui/Button/Button'
import { Modal } from '@/components/ui/Modal/Modal'
import { StatCard } from '@/components/shared/StatCard'
import { LangFlag } from '@/components/shared/LangFlag'
import { TrendChart } from '@/components/charts/TrendChart'
import type { TrendDataset } from '@/components/charts/TrendChart'
import type { ChartData as ChartDataType } from '@/types'
import styles from './Overview.module.scss'

export function Overview() {
  const { t } = useTranslation()
  const {
    stats, days, refreshInterval,
    setDays, setRefreshInterval, fetchStats, fetchAll,
  } = useStatsStore()
  const { endpoints, cacheStats, cacheHitLogs, fetchUpstreamStatus, fetchCacheStats, fetchCacheHitLogs, clearCache, triggerHealthCheck } = useEndpointStore()
  const [confirmClear, setConfirmClear] = useState(false)
  const [checking, setChecking] = useState(false)
  const [chartLabels, setChartLabels] = useState<string[]>([])
  const [totalLine, setTotalLine] = useState<TrendDataset>({ label: '', data: [] })
  const [charsLine, setCharsLine] = useState<TrendDataset>({ label: '', data: [] })
  const [epBars, setEpBars] = useState<TrendDataset[]>([])
  const [epCharBars, setEpCharBars] = useState<TrendDataset[]>([])
  const [sparkRequests, setSparkRequests] = useState<{ value: number }[]>([])
  const [sparkChars, setSparkChars] = useState<{ value: number }[]>([])
  const [sparkSuccessRate, setSparkSuccessRate] = useState<{ value: number }[]>([])
  const [sparkLatency, setSparkLatency] = useState<{ value: number }[]>([])

  const fetchChartData = useCallback(async (epList: { name: string }[], chartDays?: number | null) => {
    try {
      const d = chartDays ?? days
      const daysParam = d !== null && d > 0 ? `days=${d}` : 'days=90'
      const totalRes = await apiFetch<ChartDataType>(`/api/chart?${daysParam}`)
      const useHourly = d === 1
      const labels = useHourly
        ? totalRes.hourly.map(h => h.hour)
        : totalRes.daily.map(d => d.day)
      const totalData = useHourly
        ? totalRes.hourly.map(h => h.count)
        : totalRes.daily.map(d => d.count)
      const totalChars = useHourly
        ? totalRes.hourly.map(h => h.chars)
        : totalRes.daily.map(d => d.chars)

      setSparkRequests(totalData.map(v => ({ value: v })))
      setSparkChars(totalChars.map(v => ({ value: v })))

      const successRateData = useHourly
        ? totalRes.hourly.map(h => h.count > 0 ? (h.successes / h.count) * 100 : 100)
        : totalRes.daily.map(d => d.count > 0 ? (d.successes / d.count) * 100 : 100)
      setSparkSuccessRate(successRateData.map(v => ({ value: v })))

      const latencyData = useHourly
        ? totalRes.hourly.map(h => h.avg_latency_ms)
        : totalRes.daily.map(d => d.avg_latency_ms)
      setSparkLatency(latencyData.map(v => ({ value: v })))

      let bars: TrendDataset[] = []
      let charBars: TrendDataset[] = []
      if (epList.length > 0) {
        const epResults = await Promise.all(
          epList.map(ep => apiFetch<ChartDataType>(`/api/chart?endpoint=${encodeURIComponent(ep.name)}&${daysParam}`))
        )
        bars = epList.map((ep, i) => ({
          label: ep.name,
          data: useHourly
            ? epResults[i]?.hourly.map(h => h.count) ?? []
            : epResults[i]?.daily.map(d => d.count) ?? [],
        }))
        charBars = epList.map((ep, i) => ({
          label: ep.name,
          data: useHourly
            ? epResults[i]?.hourly.map(h => h.chars) ?? []
            : epResults[i]?.daily.map(d => d.chars) ?? [],
        }))
      }

      setChartLabels(labels)
      setTotalLine({ label: t('overview.totalRequests'), data: totalData })
      setCharsLine({ label: t('overview.totalChars'), data: totalChars })
      setEpBars(bars)
      setEpCharBars(charBars)
    } catch {}
  }, [days, t])

  useEffect(() => {
    fetchAll()
    fetchUpstreamStatus()
    fetchCacheStats()
    fetchCacheHitLogs()
  }, [fetchAll, fetchUpstreamStatus, fetchCacheStats, fetchCacheHitLogs])

  useEffect(() => {
    fetchChartData(endpoints)
  }, [endpoints, fetchChartData])

  const refreshData = useCallback(async () => {
    await Promise.all([
      fetchAll(),
      fetchUpstreamStatus(),
      fetchCacheStats(),
      fetchChartData(endpoints),
    ])
  }, [fetchAll, fetchUpstreamStatus, fetchCacheStats, endpoints, fetchChartData])

  const refresh = useCallback(async () => {
    await refreshData()
    showToast(t('overview.refreshDone'))
  }, [refreshData, t])

  useAutoRefresh(refreshData, refreshInterval)

  const onDaysChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const v = Number(e.target.value)
    setDays(v === 0 ? null : v)
    fetchStats(v === 0 ? null : v)
    fetchChartData(endpoints, v === 0 ? null : v)
  }

  const onRefreshChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setRefreshInterval(Number(e.target.value))
  }

  const onHealthCheck = async () => {
    setChecking(true)
    await triggerHealthCheck()
    setChecking(false)
  }

  const successRate = stats
    ? stats.total_requests > 0
      ? ((stats.total_requests - (stats.health.status === 'error' ? 1 : 0)) / stats.total_requests * 100).toFixed(1)
      : '100.0'
    : '-'

  const labelMode = days === 1 ? 'time' as const : days === null ? 'full' as const : 'day' as const

  const formatNum = (n: number) => {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
    return String(n)
  }

  const formatTtl = (secs: number) => {
    if (secs >= 3600 && secs % 3600 === 0) return `${secs / 3600} ${t('settings.ttlUnitHours')}`
    if (secs >= 60 && secs % 60 === 0) return `${secs / 60} ${t('settings.ttlUnitMinutes')}`
    return `${secs} ${t('settings.ttlUnitSeconds')}`
  }

  const formatBytes = (bytes: number) => {
    if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${bytes} B`
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
        <Button variant="secondary" className={styles.refreshBtn} onClick={refresh}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/></svg>
          {t('overview.refresh')}
        </Button>
      </div>

      <div className={styles.statsGrid}>
        <StatCard
          label={t('overview.totalRequests')}
          value={formatNum(stats?.total_requests ?? 0)}
          icon={<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>}
          chartData={sparkRequests}
          chartColor="#8b5cf6"
        />
        <StatCard
          label={t('overview.totalChars')}
          value={formatNum(stats?.total_chars ?? 0)}
          icon={<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M4 7V4h16v3"/><path d="M9 20h6"/><path d="M12 4v16"/></svg>}
          chartData={sparkChars}
          chartColor="#06b6d4"
        />
        <StatCard
          label={t('overview.successRate')}
          value={`${successRate}%`}
          icon={<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><polyline points="9 12 11 14 15 10"/></svg>}
          chartData={sparkSuccessRate}
          chartColor="#22c55e"
        />
        <StatCard
          label={t('overview.avgLatency')}
          value={stats?.health.latency_ms != null ? `${stats.health.latency_ms}ms` : '-'}
          icon={<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>}
          chartData={sparkLatency}
          chartColor="#f97316"
        />
      </div>

      <Card className={styles.chartCard}>
        <TrendChart
          title={t('overview.requestTrend')}
          labels={chartLabels}
          lineDataset={totalLine}
          barDatasets={epBars}
          labelMode={labelMode}
        />
      </Card>

      <Card className={styles.chartCard}>
        <TrendChart
          title={t('overview.charTrend')}
          labels={chartLabels}
          lineDataset={charsLine}
          barDatasets={epCharBars}
          labelMode={labelMode}
        />
      </Card>

      <Card className={styles.healthCard}>
        <div className={styles.healthHeader}>
          <h3 className={styles.sectionTitle}>{t('overview.endpointHealth')}</h3>
          <Button variant="secondary" size="sm" onClick={onHealthCheck} disabled={checking}>
            {t('endpoints.healthCheck')}
          </Button>
        </div>
        <div className={styles.epGrid}>
          {endpoints.map((ep) => (
            <div key={ep.name} className={styles.epCard}>
              <div className={styles.epCardHeader}>
                <span className={`${styles.badge} ${ep.healthy ? styles.online : styles.offline}`}>
                  {ep.healthy ? t('endpoints.online') : t('endpoints.offline')}
                </span>
                <span className={styles.epName}>{ep.name}</span>
              </div>
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
            </div>
          ))}
        </div>
      </Card>

      <Card className={styles.cacheCard}>
        <div className={styles.cacheHeader}>
          <h3 className={styles.sectionTitle}>{t('endpoints.cache')}</h3>
          <Button variant="danger" size="sm" onClick={() => setConfirmClear(true)}>
            {t('endpoints.clearCache')}
          </Button>
        </div>
        {cacheStats && (
          <>
            <div className={styles.cacheGrid}>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>{t('endpoints.hitRate')}</span>
                <span className={styles.cacheValue}>{(cacheStats.hit_rate * 100).toFixed(1)}%</span>
              </div>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>{t('endpoints.hitCount')}</span>
                <span className={styles.cacheValue}>{cacheStats.hits.toLocaleString()}</span>
              </div>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>
                  {cacheStats.max_memory_mb > 0 ? t('settings.maxMemory') : t('endpoints.size')}
                </span>
                <span className={styles.cacheValue}>
                  {cacheStats.max_memory_mb > 0
                    ? `${formatBytes(cacheStats.estimated_memory_bytes)} / ${cacheStats.max_memory_mb} MB`
                    : `${cacheStats.size} / ${cacheStats.max_entries}`}
                </span>
              </div>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>{t('endpoints.cacheTtl')}</span>
                <span className={styles.cacheValue}>{formatTtl(cacheStats.ttl_secs)}</span>
              </div>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>{cacheStats.enabled ? t('endpoints.enabled') : t('endpoints.disabled')}</span>
                <span className={`${styles.cacheValue} ${cacheStats.enabled ? styles.successText : styles.errorText}`}>
                  {cacheStats.enabled ? '●' : '○'}
                </span>
              </div>
            </div>
            {cacheHitLogs.length > 0 && (
              <div className={styles.hitList}>
                <h4 className={styles.subTitle}>{t('endpoints.cacheHits')}</h4>
                <div className={styles.hitTableHead}>
                  <span>{t('endpoints.cacheHitLang')}</span>
                  <span>{t('endpoints.cacheHitText')}</span>
                  <span>{t('endpoints.cacheHitTime')}</span>
                </div>
                {cacheHitLogs.slice(0, 10).map((hit, i) => (
                  <div key={i} className={styles.hitItem}>
                    <span className={styles.hitLang}>
                      <span className={styles.langPair}><LangFlag lang={hit.source_lang} size={14} />{hit.source_lang}</span>
                      <span className={styles.langArrow}>→</span>
                      <span className={styles.langPair}><LangFlag lang={hit.target_lang} size={14} />{hit.target_lang}</span>
                    </span>
                    <span className={styles.hitText}>{hit.text_preview}</span>
                    <span className={styles.hitTime}>{hit.timestamp}</span>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </Card>

      <Modal open={confirmClear} onClose={() => setConfirmClear(false)} title={t('settings.confirmDelete')}>
        <p style={{ marginBottom: 16 }}>{t('endpoints.clearCache')}?</p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={() => setConfirmClear(false)}>{t('common.cancel')}</Button>
          <Button variant="danger" onClick={async () => { await clearCache(); setConfirmClear(false) }}>{t('common.confirm')}</Button>
        </div>
      </Modal>
    </div>
  )
}
