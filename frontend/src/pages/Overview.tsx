import { useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useStatsStore } from '@/stores/useStatsStore'
import { useEndpointStore } from '@/stores/useEndpointStore'
import { useAutoRefresh } from '@/hooks/useAutoRefresh'
import { Card } from '@/components/ui/Card/Card'
import { Select } from '@/components/ui/Select/Select'
import { Button } from '@/components/ui/Button/Button'
import { StatCard } from '@/components/shared/StatCard'
import { TrendChart } from '@/components/charts/TrendChart'
import styles from './Overview.module.scss'

export function Overview() {
  const { t } = useTranslation()
  const {
    stats, chartData, days, refreshInterval,
    setDays, setRefreshInterval, fetchStats, fetchChart, fetchAll,
  } = useStatsStore()
  const { endpoints, fetchUpstreamStatus } = useEndpointStore()

  useEffect(() => {
    fetchAll()
    fetchUpstreamStatus()
  }, [fetchAll, fetchUpstreamStatus])

  const refresh = useCallback(() => {
    fetchAll()
    fetchUpstreamStatus()
  }, [fetchAll, fetchUpstreamStatus])

  useAutoRefresh(refresh, refreshInterval)

  const onDaysChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const v = Number(e.target.value)
    setDays(v === 0 ? null : v)
    fetchStats(v === 0 ? null : v)
    fetchChart()
  }

  const onRefreshChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setRefreshInterval(Number(e.target.value))
  }

  const successRate = stats
    ? stats.total_requests > 0
      ? ((stats.total_requests - (stats.health.status === 'error' ? 1 : 0)) / stats.total_requests * 100).toFixed(1)
      : '100.0'
    : '-'

  const hourlyData = chartData?.hourly.map((h) => ({ value: h.count })) ?? []
  const chartLabels = chartData?.daily.map((d) => d.day) ?? chartData?.hourly.map((h) => h.hour) ?? []
  const chartValues = chartData?.daily.map((d) => d.count) ?? chartData?.hourly.map((h) => h.count) ?? []

  const formatNum = (n: number) => {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
    return String(n)
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
        <Button variant="ghost" size="sm" onClick={refresh}>{t('overview.refresh')}</Button>
      </div>

      <div className={styles.statsGrid}>
        <StatCard
          label={t('overview.totalRequests')}
          value={formatNum(stats?.total_requests ?? 0)}
          subtitle={t('overview.cumulative')}
          chartData={hourlyData}
        />
        <StatCard
          label={t('overview.totalChars')}
          value={formatNum(stats?.total_chars ?? 0)}
          subtitle={t('overview.cumulative')}
          chartData={hourlyData}
          chartColor="var(--color-success)"
        />
        <StatCard
          label={t('overview.successRate')}
          value={`${successRate}%`}
          subtitle={t('overview.trend')}
        />
        <StatCard
          label={t('overview.avgLatency')}
          value={stats?.health.latency_ms != null ? `${stats.health.latency_ms}ms` : '-'}
          subtitle={t('overview.trend')}
        />
      </div>

      <Card className={styles.chartCard}>
        <TrendChart
          labels={chartLabels}
          datasets={[{ label: t('overview.totalRequests'), data: chartValues }]}
        />
      </Card>

      <Card className={styles.healthCard}>
        <h3 className={styles.sectionTitle}>{t('overview.endpointHealth')}</h3>
        <div className={styles.healthGrid}>
          {endpoints.map((ep) => (
            <div key={ep.name} className={styles.healthItem}>
              <span className={`${styles.dot} ${ep.healthy ? styles.online : styles.offline}`} />
              <span className={styles.epName}>{ep.name}</span>
              <span className={styles.epLatency}>{ep.avg_latency_ms.toFixed(0)}ms</span>
            </div>
          ))}
        </div>
      </Card>
    </div>
  )
}
