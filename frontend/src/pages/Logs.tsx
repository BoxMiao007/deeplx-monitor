import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useLogsStore } from '@/stores/useLogsStore'
import { useEndpointStore } from '@/stores/useEndpointStore'
import { Card } from '@/components/ui/Card/Card'
import { Select } from '@/components/ui/Select/Select'
import { Button } from '@/components/ui/Button/Button'
import { EmptyState } from '@/components/ui/EmptyState/EmptyState'
import { LoadingSpinner } from '@/components/ui/LoadingSpinner/LoadingSpinner'
import styles from './Logs.module.scss'

export function Logs() {
  const { t } = useTranslation()
  const {
    logs, total, page, pageSize, loading,
    filterStatus, filterEndpoint,
    setFilter, fetchLogs, exportLogs,
  } = useLogsStore()
  const { endpoints, fetchUpstreamStatus } = useEndpointStore()

  useEffect(() => {
    fetchLogs()
    fetchUpstreamStatus()
  }, [fetchLogs, fetchUpstreamStatus])

  const totalPages = Math.ceil(total / pageSize)

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h3>{t('logs.title')}</h3>
        <Button variant="secondary" size="sm" onClick={() => exportLogs()}>
          {t('settings.exportCsv')}
        </Button>
      </div>

      <div className={styles.filters}>
        <Select
          label={t('logs.status')}
          value={filterStatus}
          onChange={(e) => setFilter('filterStatus', e.target.value)}
          options={[
            { value: '', label: t('logs.all') },
            { value: 'success', label: t('logs.success') },
            { value: 'error', label: t('logs.failed') },
          ]}
        />
        <Select
          label={t('logs.endpoint')}
          value={filterEndpoint}
          onChange={(e) => setFilter('filterEndpoint', e.target.value)}
          options={[
            { value: '', label: t('logs.all') },
            ...endpoints.map((ep) => ({ value: ep.name, label: ep.name })),
          ]}
        />
      </div>

      <Card padding="sm">
        {loading ? (
          <div className={styles.loadingWrap}><LoadingSpinner /></div>
        ) : logs.length > 0 ? (
          <div className={styles.tableWrap}>
            <table className={styles.table}>
              <thead>
                <tr>
                  <th>{t('logs.time')}</th>
                  <th>{t('logs.sourceLang')}</th>
                  <th>{t('logs.targetLang')}</th>
                  <th>{t('logs.endpoint')}</th>
                  <th>{t('logs.latency')}</th>
                  <th>{t('logs.status')}</th>
                </tr>
              </thead>
              <tbody>
                {logs.map((log) => (
                  <tr key={log.id}>
                    <td className={styles.mono}>{log.created_at}</td>
                    <td>{log.source_lang || 'auto'}</td>
                    <td>{log.target_lang}</td>
                    <td>{log.endpoint_name ?? '-'}</td>
                    <td className={styles.mono}>{log.latency_ms != null ? `${log.latency_ms}ms` : '-'}</td>
                    <td>
                      <span className={`${styles.statusBadge} ${log.status === 'success' ? styles.success : styles.error}`}>
                        {log.status === 'success' ? t('logs.success') : t('logs.failed')}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <EmptyState />
        )}
      </Card>

      {totalPages > 1 && (
        <div className={styles.pagination}>
          <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => fetchLogs(page - 1)}>
            ←
          </Button>
          <span className={styles.pageInfo}>{page} / {totalPages}</span>
          <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => fetchLogs(page + 1)}>
            →
          </Button>
        </div>
      )}
    </div>
  )
}
