import { useTranslation } from 'react-i18next'
import { ThemeSwitcher } from '@/components/shared/ThemeSwitcher'
import { LanguageSwitcher } from '@/components/shared/LanguageSwitcher'
import type { Tab } from '@/types'
import styles from './TopBar.module.scss'

interface TopBarProps {
  activeTab: Tab
  onTabChange: (tab: Tab) => void
}

const TABS: Tab[] = ['overview', 'analysis', 'logs', 'settings']

export function TopBar({ activeTab, onTabChange }: TopBarProps) {
  const { t } = useTranslation()

  return (
    <header className={styles.topbar}>
      <div className={styles.brand}>
        <img src="/favicon.svg" alt="" width={20} height={20} />
        <span className={styles.brandText}>{t('brand')}</span>
      </div>

      <nav className={styles.tabs}>
        {TABS.map((tab) => (
          <button
            key={tab}
            className={`${styles.tab} ${activeTab === tab ? styles.active : ''}`}
            onClick={() => onTabChange(tab)}
          >
            {t(`tabs.${tab}`)}
          </button>
        ))}
      </nav>

      <div className={styles.actions}>
        <ThemeSwitcher />
        <LanguageSwitcher />
      </div>
    </header>
  )
}
