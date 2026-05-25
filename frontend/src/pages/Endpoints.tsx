import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useEndpointStore } from '@/stores/useEndpointStore'
import { Card } from '@/components/ui/Card/Card'
import { Button } from '@/components/ui/Button/Button'
import { Modal } from '@/components/ui/Modal/Modal'
import { EmptyState } from '@/components/ui/EmptyState/EmptyState'
import styles from './Endpoints.module.scss'

export function Endpoints() {
  const { t } = useTranslation()
  const {
    endpoints, cacheStats, cacheHitLogs,
    fetchUpstreamStatus, fetchCacheStats, fetchCacheHitLogs,
    clearCache, triggerHealthCheck,
  } = useEndpointStore()

  const [confirmClear, setConfirmClear] = useState(false)
  const [checking, setChecking] = useState(false)

  useEffect(() => {
    fetchUpstreamStatus()
    fetchCacheStats()
    fetchCacheHitLogs()
  }, [fetchUpstreamStatus, fetchCacheStats, fetchCacheHitLogs])

  const onHealthCheck = async () => {
    setChecking(true)
    await triggerHealthCheck()
    setChecking(false)
  }

  const onClearCache = async () => {
    await clearCache()
    setConfirmClear(false)
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

      <div className={styles.cacheSection}>
        <div className={styles.header}>
          <h3>{t('endpoints.cache')}</h3>
          <Button variant="danger" size="sm" onClick={() => setConfirmClear(true)}>
            {t('endpoints.clearCache')}
          </Button>
        </div>

        {cacheStats && (
          <Card className={styles.cacheCard}>
            <div className={styles.cacheGrid}>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>{t('endpoints.hitRate')}</span>
                <span className={styles.cacheValue}>{(cacheStats.hit_rate * 100).toFixed(1)}%</span>
              </div>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>{t('endpoints.size')}</span>
                <span className={styles.cacheValue}>{cacheStats.size} / {cacheStats.max_entries}</span>
              </div>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>TTL</span>
                <span className={styles.cacheValue}>{cacheStats.ttl_secs}s</span>
              </div>
              <div className={styles.cacheStat}>
                <span className={styles.cacheLabel}>{cacheStats.enabled ? t('endpoints.enabled') : t('endpoints.disabled')}</span>
                <span className={`${styles.cacheValue} ${cacheStats.enabled ? styles.successText : styles.errorText}`}>
                  {cacheStats.enabled ? '●' : '○'}
                </span>
              </div>
            </div>
          </Card>
        )}

        {cacheHitLogs.length > 0 && (
          <Card>
            <h4 className={styles.subTitle}>{t('endpoints.cacheHits')}</h4>
            <div className={styles.hitList}>
              {cacheHitLogs.slice(0, 20).map((hit, i) => (
                <div key={i} className={styles.hitItem}>
                  <span className={styles.hitLang}>{hit.source_lang}→{hit.target_lang}</span>
                  <span className={styles.hitText}>{hit.text_preview}</span>
                  <span className={styles.hitTime}>{hit.timestamp}</span>
                </div>
              ))}
            </div>
          </Card>
        )}
      </div>

      <Modal open={confirmClear} onClose={() => setConfirmClear(false)} title={t('settings.confirmDelete')}>
        <p style={{ marginBottom: 16 }}>{t('endpoints.clearCache')}?</p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={() => setConfirmClear(false)}>{t('common.cancel')}</Button>
          <Button variant="danger" onClick={onClearCache}>{t('common.confirm')}</Button>
        </div>
      </Modal>
    </div>
  )
}
