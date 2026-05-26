import { type ReactNode } from 'react'
import { MiniChart } from '@/components/charts/MiniChart'
import styles from './StatCard.module.scss'

interface StatCardProps {
  label: string
  value: string
  icon?: ReactNode
  chartData?: { value: number }[]
  chartColor?: string
}

export function StatCard({ label, value, icon, chartData, chartColor = '#8b5cf6' }: StatCardProps) {
  return (
    <article className={styles.card} style={{ '--accent': chartColor } as React.CSSProperties}>
      <div className={styles.head}>
        <span className={styles.label}>{label}</span>
        {icon && <span className={styles.iconBadge}>{icon}</span>}
      </div>
      <div className={styles.value}>{value}</div>
      {chartData && chartData.length > 0 && (
        <div className={styles.chart}>
          <MiniChart data={chartData} color={chartColor} />
        </div>
      )}
    </article>
  )
}
