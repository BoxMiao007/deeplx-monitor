import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useStatsStore } from '@/stores/useStatsStore'
import { useEndpointStore } from '@/stores/useEndpointStore'
import { apiFetch } from '@/hooks/useApi'
import { Card } from '@/components/ui/Card/Card'
import { Select } from '@/components/ui/Select/Select'
import { Heatmap } from '@/components/charts/Heatmap'
import { ErrorTrend } from '@/components/charts/ErrorTrend'
import { DonutChart } from '@/components/charts/DonutChart'
import { EmptyState } from '@/components/ui/EmptyState/EmptyState'
import { LangFlag } from '@/components/shared/LangFlag'
import type { StatsResponse } from '@/types'
import styles from './Analysis.module.scss'

export function Analysis() {
  const { t } = useTranslation()
  const {
    errorTrend, langStats, days,
    fetchErrorTrend, fetchLangStats,
  } = useStatsStore()
  const { endpoints, fetchUpstreamStatus } = useEndpointStore()
  const [epChars, setEpChars] = useState<{ label: string; value: number }[]>([])
  const [heatmapDays, setHeatmapDays] = useState(1)

  useEffect(() => {
    fetchErrorTrend(7)
    fetchLangStats(days)
    fetchUpstreamStatus()
  }, [fetchErrorTrend, fetchLangStats, fetchUpstreamStatus, days])

  useEffect(() => {
    if (endpoints.length < 2) return
    Promise.all(
      endpoints.map(ep => apiFetch<StatsResponse>(`/api/stats?endpoint=${encodeURIComponent(ep.name)}`))
    ).then(results => {
      setEpChars(endpoints.map((ep, i) => ({
        label: ep.name,
        value: results[i]?.total_chars ?? 0,
      })))
    }).catch(() => {})
  }, [endpoints])

  const onHeatmapDaysChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    setHeatmapDays(Number(e.target.value))
  }

  const onErrorDaysChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    fetchErrorTrend(Number(e.target.value))
  }

  const requestDonutData = endpoints.map(ep => ({
    label: ep.name,
    value: ep.total_requests,
  }))

  const latencyDonutData = endpoints.map(ep => ({
    label: ep.name,
    value: Math.round(ep.avg_latency_ms),
  }))

  const sourceLangDonutData = langStats
    .filter(ls => ls.source_chars > 0)
    .map(ls => ({
      label: ls.lang || 'auto',
      value: ls.source_chars,
    }))

  return (
    <div className={styles.page}>
      {endpoints.length > 1 && (
        <div className={styles.statsGrid}>
          <div className={styles.statCardLarge}>
            <DonutChart
              title={t('analysis.requestShare')}
              data={requestDonutData}
            />
          </div>
          <div className={styles.statCardLarge}>
            <DonutChart
              title={t('analysis.charShare')}
              data={epChars.length > 0 ? epChars : requestDonutData}
            />
          </div>
        </div>
      )}

      <div className={styles.statsGrid}>
        {endpoints.length > 1 && (
          <div className={styles.statCardLarge}>
            <DonutChart
              title={t('analysis.latencyShare')}
              data={latencyDonutData}
              unit="ms"
            />
          </div>
        )}
        {sourceLangDonutData.length > 0 && (
          <div className={styles.statCardLarge}>
            <DonutChart
              title={t('analysis.sourceLangShare')}
              data={sourceLangDonutData}
            />
          </div>
        )}
      </div>

      <Card>
        <div className={styles.cardHeader}>
          <h3>{t('analysis.heatmap')}</h3>
          <Select
            value={heatmapDays}
            onChange={onHeatmapDaysChange}
            options={[
              { value: 1, label: t('overview.today') },
              { value: 7, label: t('overview.days7') },
            ]}
          />
        </div>
        <Heatmap days={heatmapDays} />
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
          <ErrorTrend data={errorTrend} />
        ) : (
          <EmptyState />
        )}
      </Card>

      <Card>
        <h3 className={styles.cardTitle}>{t('analysis.langStats')}</h3>
        {langStats.length > 0 ? (
          <div className={styles.langTable}>
            <div className={styles.langTableHead}>
              <span className={styles.langColName}>{t('analysis.language')}</span>
              <span className={styles.langColVal}>{t('analysis.sourceLang')}</span>
              <span className={styles.langColVal}>{t('analysis.targetLang')}</span>
              <span className={styles.langColVal}>{t('analysis.totalChars')}</span>
            </div>
            {langStats.map((ls) => {
              const total = ls.source_chars + ls.target_chars
              const maxTotal = Math.max(...langStats.map(l => l.source_chars + l.target_chars))
              const maxSource = Math.max(...langStats.map(l => l.source_chars))
              const maxTarget = Math.max(...langStats.map(l => l.target_chars))
              return (
                <div key={ls.lang} className={styles.langTableRow}>
                  <span className={styles.langColName}>
                    <LangFlag lang={ls.lang} size={16} />
                    {ls.lang || 'auto'}
                  </span>
                  <span className={styles.langColVal}>
                    <span className={styles.langBarWrap}>
                      <span className={styles.langBarFill} style={{ width: `${maxSource > 0 ? Math.min(ls.source_chars / maxSource * 100, 100) : 0}%` }} />
                    </span>
                    {ls.source_chars.toLocaleString()}
                  </span>
                  <span className={styles.langColVal}>
                    <span className={styles.langBarWrap}>
                      <span className={styles.langBarFill} style={{ width: `${maxTarget > 0 ? Math.min(ls.target_chars / maxTarget * 100, 100) : 0}%` }} />
                    </span>
                    {ls.target_chars.toLocaleString()}
                  </span>
                  <span className={styles.langColVal}>
                    <span className={styles.langBarWrap}>
                      <span className={styles.langBarFill} style={{ width: `${Math.min(total / maxTotal * 100, 100)}%` }} />
                    </span>
                    {total.toLocaleString()}
                  </span>
                </div>
              )
            })}
          </div>
        ) : (
          <EmptyState />
        )}
      </Card>
    </div>
  )
}
