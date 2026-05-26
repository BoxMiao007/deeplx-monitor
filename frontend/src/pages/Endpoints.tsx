import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useEndpointStore } from '@/stores/useEndpointStore'
import { Card } from '@/components/ui/Card/Card'
import { Button } from '@/components/ui/Button/Button'
import { EmptyState } from '@/components/ui/EmptyState/EmptyState'
import styles from './Endpoints.module.scss'

export function Endpoints() {
  const { t } = useTranslation()
  const {
    endpoints,
    fetchUpstreamStatus, triggerHealthCheck,
  } = useEndpointStore()

  const [checking, setChecking] = useState(false)

  useEffect(() => {
    fetchUpstreamStatus()
  }, [fetchUpstreamStatus])

  const onHealthCheck = async () => {
    setChecking(true)
    await triggerHealthCheck()
    setChecking(false)
  }

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h3>{t('endpoints.title')}</h3>
        <Button variant="secondary" size="sm" onClick={onHealthCheck} disabled={checking}>
          {t('endpoints.healthCheck')}
        </Button>
      </div>

      {endpoints.length > 0 ? (
        <div className={styles.epGrid}>
          {endpoints.map((ep) => (
            <Card key={ep.name} className={styles.epCard}>
              <div className={styles.epHeader}>
                <span className={`${styles.badge} ${ep.healthy ? styles.online : styles.offline}`}>
                  {ep.healthy ? t('endpoints.online') : t('endpoints.offline')}
                </span>
                <span className={styles.epName}>{ep.name}</span>
              </div>
              <div className={styles.epUrl}>{ep.url}</div>
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
            </Card>
          ))}
        </div>
      ) : (
        <EmptyState />
      )}
    </div>
  )
}
