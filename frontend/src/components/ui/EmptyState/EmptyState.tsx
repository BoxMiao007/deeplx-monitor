import { useTranslation } from 'react-i18next'
import styles from './EmptyState.module.scss'

export function EmptyState({ message }: { message?: string }) {
  const { t } = useTranslation()
  return (
    <div className={styles.empty}>
      <p>{message ?? t('common.empty')}</p>
    </div>
  )
}
