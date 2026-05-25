import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useStatsStore } from '@/stores/useStatsStore'
import { useEndpointStore } from '@/stores/useEndpointStore'
import { Card } from '@/components/ui/Card/Card'
import { Select } from '@/components/ui/Select/Select'
import { Heatmap } from '@/components/charts/Heatmap'
import { ErrorTrend } from '@/components/charts/ErrorTrend'
import { TrendChart } from '@/components/charts/TrendChart'
import { EmptyState } from '@/components/ui/EmptyState/EmptyState'
import styles from './Analysis.module.scss'

const HOURS = Array.from({ length: 24 }, (_, i) => String(i))
const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']

export function Analysis() {
  const { t } = useTranslation()
  const {
    heatmapData, errorTrend, langStats, days,
    fetchHeatmap, fetchErrorTrend, fetchLangStats,
  } = useStatsStore()
  const { endpoints, fetchUpstreamStatus } = useEndpointStore()

  useEffect(() => {
    fetchHeatmap('weekday', 30)
    fetchErrorTrend(7)
    fetchLangStats(days)
    fetchUpstreamStatus()
  }, [fetchHeatmap, fetchErrorTrend, fetchLangStats, fetchUpstreamStatus, days])

  const onHeatmapViewChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    fetchHeatmap(e.target.value, 30)
  }

  const onErrorDaysChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    fetchErrorTrend(Number(e.target.value))
  }

  return (
    <div className={styles.page}>
      <Card>
        <div className={styles.cardHeader}>
          <h3>{t('analysis.heatmap')}</h3>
          <Select
            value={heatmapData?.view ?? 'weekday'}
            onChange={onHeatmapViewChange}
            options={[
              { value: 'weekday', label: t('analysis.weekday') },
              { value: 'hourly', label: t('analysis.hourly') },
            ]}
          />
        </div>
        {heatmapData && heatmapData.data.length > 0 ? (
          <Heatmap
            data={heatmapData.data}
            xLabels={HOURS}
            yLabels={heatmapData.view === 'weekday' ? WEEKDAYS : HOURS}
          />
        ) : (
          <EmptyState />
        )}
      </Card>

      <Card>
        <div className={styles.cardHeader}>
          <h3>{t('analysis.errorTrend')}</h3>
          <Select
            value="7"
            onChange={onErrorDaysChange}
            options={[
              { value: 7, label: '7d' },
              { value: 14, label: '14d' },
              { value: 30, label: '30d' },
            ]}
          />
        </div>
        {errorTrend.length > 0 ? (
          <ErrorTrend
            labels={errorTrend.map((p) => p.time)}
            errorRates={errorTrend.map((p) => p.error_rate * 100)}
          />
        ) : (
          <EmptyState />
        )}
      </Card>

      <Card>
        <h3 className={styles.cardTitle}>{t('analysis.langStats')}</h3>
        {langStats.length > 0 ? (
          <div className={styles.langGrid}>
            {langStats.map((ls) => (
              <div key={ls.lang} className={styles.langItem}>
                <span className={styles.langName}>{ls.lang || 'auto'}</span>
                <div className={styles.langBar}>
                  <div className={styles.langBarFill} style={{ width: `${Math.min((ls.source_chars + ls.target_chars) / Math.max(...langStats.map(l => l.source_chars + l.target_chars)) * 100, 100)}%` }} />
                </div>
                <span className={styles.langCount}>{(ls.source_chars + ls.target_chars).toLocaleString()}</span>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState />
        )}
      </Card>

      <Card>
        <h3 className={styles.cardTitle}>{t('analysis.endpointCompare')}</h3>
        {endpoints.length > 1 ? (
          <TrendChart
            labels={endpoints.map((ep) => ep.name)}
            datasets={[{
              label: t('endpoints.latency'),
              data: endpoints.map((ep) => ep.avg_latency_ms),
              borderColor: 'var(--color-primary)',
            }]}
            height={200}
          />
        ) : (
          <EmptyState />
        )}
      </Card>
    </div>
  )
}
