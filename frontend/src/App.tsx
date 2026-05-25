import { useState, useEffect } from 'react'
import { useThemeStore } from '@/stores/useThemeStore'
import { useStatsStore } from '@/stores/useStatsStore'
import { TopBar } from '@/components/layout/TopBar'
import { PageShell } from '@/components/layout/PageShell'
import { ToastContainer } from '@/components/shared/Toast'
import { Overview } from '@/pages/Overview'
import { Analysis } from '@/pages/Analysis'
import { Endpoints } from '@/pages/Endpoints'
import { Logs } from '@/pages/Logs'
import { Settings } from '@/pages/Settings'
import type { Tab } from '@/types'

function getInitialTab(): Tab {
  const hash = window.location.hash.slice(1)
  const valid: Tab[] = ['overview', 'analysis', 'endpoints', 'logs', 'settings']
  return valid.includes(hash as Tab) ? (hash as Tab) : 'overview'
}

function App() {
  const [activeTab, setActiveTab] = useState<Tab>(getInitialTab)
  const { theme } = useThemeStore()
  const { fetchVersion } = useStatsStore()

  useEffect(() => {
    fetchVersion()
  }, [fetchVersion])

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
  }, [theme])

  const onTabChange = (tab: Tab) => {
    setActiveTab(tab)
    window.location.hash = tab
  }

  useEffect(() => {
    const onHash = () => {
      const hash = window.location.hash.slice(1) as Tab
      const valid: Tab[] = ['overview', 'analysis', 'endpoints', 'logs', 'settings']
      if (valid.includes(hash)) setActiveTab(hash)
    }
    window.addEventListener('hashchange', onHash)
    return () => window.removeEventListener('hashchange', onHash)
  }, [])

  const renderPage = () => {
    switch (activeTab) {
      case 'overview': return <Overview />
      case 'analysis': return <Analysis />
      case 'endpoints': return <Endpoints />
      case 'logs': return <Logs />
      case 'settings': return <Settings />
    }
  }

  return (
    <>
      <TopBar activeTab={activeTab} onTabChange={onTabChange} />
      <PageShell>{renderPage()}</PageShell>
      <ToastContainer />
    </>
  )
}

export default App
