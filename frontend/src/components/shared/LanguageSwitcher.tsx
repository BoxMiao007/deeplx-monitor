import { useTranslation } from 'react-i18next'
import styles from './Switcher.module.scss'

export function LanguageSwitcher() {
  const { i18n } = useTranslation()

  const toggle = () => {
    const next = i18n.language === 'zh' ? 'en' : 'zh'
    i18n.changeLanguage(next)
    localStorage.setItem('language', next)
  }

  return (
    <button className={styles.switcher} onClick={toggle} aria-label="Switch language">
      <span style={{ fontSize: 13, fontWeight: 600 }}>{i18n.language === 'zh' ? 'EN' : '中'}</span>
    </button>
  )
}
