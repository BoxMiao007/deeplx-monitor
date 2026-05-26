import { useRef, useEffect } from 'react'
import { Chart, registerables } from 'chart.js'
import { useThemeStore } from '@/stores/useThemeStore'
import styles from './DonutChart.module.scss'

Chart.register(...registerables)

const COLORS: { base: string; light: string }[] = [
  { base: '#1d4ed8', light: '#60a5fa' },
  { base: '#ca8a04', light: '#facc15' },
  { base: '#15803d', light: '#22c55e' },
  { base: '#7e22ce', light: '#c084fc' },
  { base: '#b91c1c', light: '#ef4444' },
]

interface DonutItem {
  label: string
  value: number
}

interface DonutChartProps {
  title: string
  data: DonutItem[]
  unit?: string
}

export function DonutChart({ title, data, unit }: DonutChartProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const chartRef = useRef<Chart | null>(null)
  const { theme } = useThemeStore()
  const isDark = theme === 'dark'

  const total = data.reduce((s, d) => s + d.value, 0)

  useEffect(() => {
    if (!canvasRef.current || data.length === 0) return
    if (chartRef.current) chartRef.current.destroy()

    const tooltipBg = isDark ? 'rgba(17, 24, 39, 0.92)' : 'rgba(255, 255, 255, 0.98)'
    const tooltipTitle = isDark ? '#ffffff' : '#111827'
    const tooltipBody = isDark ? 'rgba(255, 255, 255, 0.86)' : '#374151'
    const tooltipBorder = isDark ? 'rgba(255, 255, 255, 0.10)' : 'rgba(17, 24, 39, 0.10)'

    const ctx = canvasRef.current.getContext('2d')!

    const gradientPlugin = {
      id: 'donutGradient',
      beforeDraw(chart: Chart) {
        const { ctx: c, chartArea } = chart
        if (!chartArea) return
        const ds = chart.data.datasets[0]
        if (!ds) return
        ds.backgroundColor = data.map((_, i) => {
          const pair = COLORS[i % COLORS.length]
          const gradient = c.createLinearGradient(0, chartArea.top, 0, chartArea.bottom)
          gradient.addColorStop(0, pair.light)
          gradient.addColorStop(1, pair.base)
          return gradient
        })
      },
    }

    chartRef.current = new Chart(ctx, {
      type: 'doughnut',
      data: {
        labels: data.map(d => d.label),
        datasets: [{
          data: data.map(d => d.value),
          backgroundColor: data.map((_, i) => COLORS[i % COLORS.length].base),
          borderColor: 'transparent',
          borderWidth: 0,
          hoverOffset: 8,
        }],
      },
      plugins: [gradientPlugin],
      options: {
        responsive: true,
        maintainAspectRatio: false,
        ...({ cutout: '58%' } as object),
        layout: { padding: 8 },
        plugins: {
          legend: { display: false },
          tooltip: {
            backgroundColor: tooltipBg,
            titleColor: tooltipTitle,
            bodyColor: tooltipBody,
            borderColor: tooltipBorder,
            borderWidth: 1,
            padding: 10,
            callbacks: {
              label: (item) => {
                const pct = total > 0 ? ((item.raw as number) / total * 100).toFixed(1) : '0'
                const suffix = unit ? ` ${unit}` : ''
                return `${item.label}: ${(item.raw as number).toLocaleString()}${suffix} (${pct}%)`
              },
            },
          },
        },
      },
    })

    return () => { chartRef.current?.destroy() }
  }, [data, isDark, total, unit])

  return (
    <div className={styles.wrapper}>
      <h4 className={styles.title}>{title}</h4>
      <div className={styles.surface}>
        <div className={styles.layout}>
          <div className={styles.chartFrame}>
            <canvas ref={canvasRef} />
          </div>
          <div className={styles.legend}>
            {data.map((item, i) => (
              <div key={item.label} className={styles.legendItem}>
                <span className={styles.legendDot} style={{ backgroundColor: COLORS[i % COLORS.length].base }} />
                <span className={styles.legendName}>{item.label}</span>
                <span className={styles.legendPct}>
                  {unit
                    ? `${item.value.toLocaleString()} ${unit}`
                    : `${total > 0 ? ((item.value / total) * 100).toFixed(1) : '0'}%`
                  }
                </span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
