import { type ReactNode, useState, useRef, useEffect } from 'react'
import { MiniChart } from '@/components/charts/MiniChart'
import styles from './StatCard.module.scss'

interface StatCardProps {
  label: string
  value: string
  detailValue?: string
  icon?: ReactNode
  chartData?: { value: number }[]
  chartColor?: string
}

export function StatCard({ label, value, detailValue, icon, chartData, chartColor = '#8b5cf6' }: StatCardProps) {
  const [showDetail, setShowDetail] = useState(false)
  const valueRef = useRef<HTMLDivElement>(null)
  const [fontSize, setFontSize] = useState(24)

  useEffect(() => {
    if (!showDetail || !valueRef.current) {
      setFontSize(24)
      return
    }
    const el = valueRef.current
    const parent = el.parentElement
    if (!parent) return
    let size = 24
    el.style.fontSize = `${size}px`
    while (el.scrollWidth > parent.clientWidth && size > 12) {
      size -= 1
      el.style.fontSize = `${size}px`
    }
    setFontSize(size)
  }, [showDetail, detailValue])

  const displayValue = showDetail && detailValue ? detailValue : value
  const clickable = !!detailValue

  return (
    <article
      className={`${styles.card} ${clickable ? styles.clickable : ''}`}
      style={{ '--accent': chartColor } as React.CSSProperties}
      onClick={clickable ? () => setShowDetail(v => !v) : undefined}
    >
      <div className={styles.head}>
        <span className={styles.label}>{label}</span>
        {icon && <span className={styles.iconBadge}>{icon}</span>}
      </div>
      <div className={styles.valueWrap}>
        <div
          ref={valueRef}
          className={styles.value}
          style={{ fontSize: showDetail ? `${fontSize}px` : undefined }}
        >
          {displayValue}
        </div>
      </div>
      {chartData && chartData.length > 0 && (
        <div className={styles.chart}>
          <MiniChart data={chartData} color={chartColor} />
        </div>
      )}
    </article>
  )
}
