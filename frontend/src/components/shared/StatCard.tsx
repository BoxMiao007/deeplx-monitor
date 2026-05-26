import { MiniChart } from '@/components/charts/MiniChart'
import styles from './StatCard.module.scss'

interface StatCardProps {
  label: string
  value: string
  subtitle?: string
  chartData?: { value: number }[]
  chartColor?: string
}

export function StatCard({ label, value, subtitle, chartData, chartColor = '#8b5cf6' }: StatCardProps) {
  return (
    <article className={styles.card} style={{ '--accent': chartColor } as React.CSSProperties}>
      <div className={styles.head}>
        <span className={styles.label}>{label}</span>
        {subtitle && <span className={styles.subtitle}>{subtitle}</span>}
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
