import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useConfigStore } from '@/stores/useConfigStore'
import { useLogsStore } from '@/stores/useLogsStore'
import { Card } from '@/components/ui/Card/Card'
import { Button } from '@/components/ui/Button/Button'
import { Input } from '@/components/ui/Input/Input'
import { Modal } from '@/components/ui/Modal/Modal'
import type { FullConfig, FullEndpointInfo } from '@/types'
import styles from './Settings.module.scss'

type TtlUnit = 'seconds' | 'minutes' | 'hours'

function detectTtlUnit(secs: number): TtlUnit {
  if (secs > 0 && secs % 3600 === 0) return 'hours'
  if (secs > 0 && secs % 60 === 0) return 'minutes'
  return 'seconds'
}

function secsToDisplay(secs: number, unit: TtlUnit): number {
  if (unit === 'hours') return secs / 3600
  if (unit === 'minutes') return secs / 60
  return secs
}

function displayToSecs(value: number, unit: TtlUnit): number {
  if (unit === 'hours') return value * 3600
  if (unit === 'minutes') return value * 60
  return value
}

export function Settings() {
  const { t } = useTranslation()
  const { config, fetchConfig, updateConfig } = useConfigStore()
  const { exportLogs } = useLogsStore()
  const [draft, setDraft] = useState<FullConfig | null>(null)
  const [saving, setSaving] = useState(false)
  const [toast, setToast] = useState('')
  const [deleteIdx, setDeleteIdx] = useState<number | null>(null)
  const [ttlUnit, setTtlUnit] = useState<TtlUnit>('seconds')

  useEffect(() => {
    fetchConfig()
  }, [fetchConfig])

  useEffect(() => {
    if (config) {
      setDraft(structuredClone(config))
      setTtlUnit(detectTtlUnit(config.cache.ttl_secs))
    }
  }, [config])

  if (!draft) return null

  const save = async () => {
    setSaving(true)
    const ok = await updateConfig(draft)
    setSaving(false)
    if (ok) {
      setToast(t('settings.saved'))
      setTimeout(() => setToast(''), 2000)
    }
  }

  const updateEndpoint = (idx: number, field: keyof FullEndpointInfo, value: string) => {
    const eps = [...draft.upstream.endpoints]
    eps[idx] = { ...eps[idx], [field]: value }
    setDraft({ ...draft, upstream: { ...draft.upstream, endpoints: eps } })
  }

  const addEndpoint = () => {
    const eps = [...draft.upstream.endpoints, { name: '', url: '', api_key: '' }]
    setDraft({ ...draft, upstream: { ...draft.upstream, endpoints: eps } })
  }

  const removeEndpoint = (idx: number) => {
    const eps = draft.upstream.endpoints.filter((_, i) => i !== idx)
    setDraft({ ...draft, upstream: { ...draft.upstream, endpoints: eps } })
    setDeleteIdx(null)
  }

  return (
    <div className={styles.page}>
      {toast && <div className={styles.toast}>{toast}</div>}

      <Card>
        <div className={styles.sectionHeader}>
          <h3>{t('settings.upstream')}</h3>
          <Button variant="secondary" size="sm" onClick={addEndpoint}>{t('settings.add')}</Button>
        </div>
        <div className={styles.epList}>
          {draft.upstream.endpoints.map((ep, i) => (
            <div key={i} className={styles.epRow}>
              <Input label={t('settings.name')} value={ep.name} onChange={(e) => updateEndpoint(i, 'name', e.target.value)} />
              <Input label={t('settings.url')} value={ep.url} onChange={(e) => updateEndpoint(i, 'url', e.target.value)} />
              <Input label={t('settings.apiKey')} value={ep.api_key} onChange={(e) => updateEndpoint(i, 'api_key', e.target.value)} type="password" />
              <Button variant="danger" size="sm" onClick={() => setDeleteIdx(i)}>{t('settings.delete')}</Button>
            </div>
          ))}
        </div>
        <div className={styles.fieldRow}>
          <Input
            label={t('settings.maxFailures')}
            type="number"
            value={draft.upstream.max_failures}
            onChange={(e) => setDraft({ ...draft, upstream: { ...draft.upstream, max_failures: Number(e.target.value) } })}
          />
          <Input
            label={t('settings.probeInterval')}
            type="number"
            value={draft.upstream.probe_interval_secs}
            onChange={(e) => setDraft({ ...draft, upstream: { ...draft.upstream, probe_interval_secs: Number(e.target.value) } })}
          />
        </div>
      </Card>

      <Card>
        <h3 className={styles.sectionTitle}>{t('settings.proxy')} <span className={styles.hint}>({t('settings.restartRequired')})</span></h3>
        <div className={styles.fieldRow}>
          <Input label={t('settings.host')} value={draft.proxy.host} onChange={(e) => setDraft({ ...draft, proxy: { ...draft.proxy, host: e.target.value } })} />
          <Input label={t('settings.port')} type="number" value={draft.proxy.port} onChange={(e) => setDraft({ ...draft, proxy: { ...draft.proxy, port: Number(e.target.value) } })} />
        </div>
      </Card>

      <Card>
        <h3 className={styles.sectionTitle}>{t('settings.monitor')}</h3>
        <div className={styles.fieldRow}>
          <Input label={t('settings.logRetention')} type="number" value={draft.monitor.log_retention_days} onChange={(e) => setDraft({ ...draft, monitor: { ...draft.monitor, log_retention_days: Number(e.target.value) } })} />
          <Input label={t('settings.autoRefresh')} type="number" value={draft.monitor.auto_refresh_seconds} onChange={(e) => setDraft({ ...draft, monitor: { ...draft.monitor, auto_refresh_seconds: Number(e.target.value) } })} />
        </div>
      </Card>

      <Card>
        <h3 className={styles.sectionTitle}>{t('settings.cacheConfig')}</h3>
        <div className={styles.fieldRow}>
          <label className={styles.checkbox}>
            <input type="checkbox" checked={draft.cache.enabled} onChange={(e) => setDraft({ ...draft, cache: { ...draft.cache, enabled: e.target.checked } })} />
            {t('endpoints.enabled')}
          </label>
          <div className={styles.ttlGroup}>
            <Input
              label={t('settings.ttl')}
              type="number"
              value={secsToDisplay(draft.cache.ttl_secs, ttlUnit)}
              onChange={(e) => setDraft({ ...draft, cache: { ...draft.cache, ttl_secs: displayToSecs(Number(e.target.value), ttlUnit) } })}
            />
            <select
              className={styles.ttlSelect}
              value={ttlUnit}
              onChange={(e) => {
                const newUnit = e.target.value as TtlUnit
                setTtlUnit(newUnit)
              }}
            >
              <option value="seconds">{t('settings.ttlUnitSeconds')}</option>
              <option value="minutes">{t('settings.ttlUnitMinutes')}</option>
              <option value="hours">{t('settings.ttlUnitHours')}</option>
            </select>
          </div>
          <Input label={t('settings.maxEntries')} type="number" value={draft.cache.max_entries} onChange={(e) => setDraft({ ...draft, cache: { ...draft.cache, max_entries: Number(e.target.value) } })} />
          <Input label={t('settings.maxMemory')} type="number" value={draft.cache.max_memory_mb} onChange={(e) => setDraft({ ...draft, cache: { ...draft.cache, max_memory_mb: Number(e.target.value) } })} />
        </div>
      </Card>

      <Card>
        <h3 className={styles.sectionTitle}>{t('settings.healthCheck')}</h3>
        <div className={styles.fieldRow}>
          <Input label={t('settings.sourceLang')} value={draft.health_check.source_lang} onChange={(e) => setDraft({ ...draft, health_check: { ...draft.health_check, source_lang: e.target.value } })} />
          <Input label={t('settings.targetLang')} value={draft.health_check.target_lang} onChange={(e) => setDraft({ ...draft, health_check: { ...draft.health_check, target_lang: e.target.value } })} />
        </div>
      </Card>

      <Card>
        <h3 className={styles.sectionTitle}>{t('settings.export')}</h3>
        <Button variant="secondary" onClick={() => exportLogs()}>{t('settings.exportCsv')}</Button>
      </Card>

      <div className={styles.saveBar}>
        <Button variant="primary" onClick={save} disabled={saving}>{t('settings.save')}</Button>
      </div>

      <Modal open={deleteIdx !== null} onClose={() => setDeleteIdx(null)} title={t('settings.confirmDelete')}>
        <p style={{ marginBottom: 16 }}>{t('settings.confirmDelete')}</p>
        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <Button variant="secondary" onClick={() => setDeleteIdx(null)}>{t('common.cancel')}</Button>
          <Button variant="danger" onClick={() => deleteIdx !== null && removeEndpoint(deleteIdx)}>{t('common.confirm')}</Button>
        </div>
      </Modal>
    </div>
  )
}
