import type { HeatmapCell } from '@/types'
import styles from './Heatmap.module.scss'

interface HeatmapProps {
  data: HeatmapCell[]
  xLabels: string[]
  yLabels: string[]
}

function getIntensity(count: number, max: number): number {
  if (max === 0) return 0
  return Math.min(count / max, 1)
}

export function Heatmap({ data, xLabels, yLabels }: HeatmapProps) {
  const max = Math.max(...data.map((d) => d.count), 1)

  const cellMap = new Map<string, number>()
  data.forEach((d) => cellMap.set(`${d.x}-${d.y}`, d.count))

  return (
    <div className={styles.container}>
      <div className={styles.grid} style={{ gridTemplateColumns: `auto repeat(${xLabels.length}, 1fr)` }}>
        <div />
        {xLabels.map((x) => (
          <div key={x} className={styles.xLabel}>{x}</div>
        ))}
        {yLabels.map((y) => (
          <div key={`row-${y}`} className={styles.row}>
            <div className={styles.yLabel}>{y}</div>
            {xLabels.map((x) => {
              const count = cellMap.get(`${x}-${y}`) ?? 0
              const intensity = getIntensity(count, max)
              return (
                <div
                  key={`${x}-${y}`}
                  className={styles.cell}
                  style={{ opacity: 0.1 + intensity * 0.9, backgroundColor: `var(--color-primary)` }}
                  title={`${x}, ${y}: ${count}`}
                />
              )
            })}
          </div>
        ))}
      </div>
    </div>
  )
}
