import { useRef, useEffect } from 'react'
import { Chart, registerables } from 'chart.js'
import styles from './TrendChart.module.scss'

Chart.register(...registerables)

interface TrendChartProps {
  labels: string[]
  datasets: {
    label: string
    data: number[]
    borderColor?: string
    backgroundColor?: string
  }[]
  height?: number
}

export function TrendChart({ labels, datasets, height = 280 }: TrendChartProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const chartRef = useRef<Chart | null>(null)

  useEffect(() => {
    if (!canvasRef.current) return

    if (chartRef.current) {
      chartRef.current.destroy()
    }

    const ctx = canvasRef.current.getContext('2d')!
    chartRef.current = new Chart(ctx, {
      type: 'line',
      data: {
        labels,
        datasets: datasets.map((ds) => ({
          ...ds,
          borderColor: ds.borderColor ?? getComputedStyle(document.documentElement).getPropertyValue('--color-primary').trim(),
          backgroundColor: ds.backgroundColor ?? 'transparent',
          borderWidth: 2,
          pointRadius: 0,
          pointHoverRadius: 4,
          tension: 0.3,
          fill: !!ds.backgroundColor,
        })),
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        interaction: { mode: 'index', intersect: false },
        plugins: {
          legend: { display: datasets.length > 1, position: 'top', labels: { boxWidth: 12, padding: 16 } },
          tooltip: { backgroundColor: 'rgba(0,0,0,0.8)', padding: 10, cornerRadius: 8 },
        },
        scales: {
          x: { grid: { display: false }, ticks: { maxTicksLimit: 8, font: { size: 11 } } },
          y: { beginAtZero: true, grid: { color: 'var(--border-color)' }, ticks: { font: { size: 11 } } },
        },
      },
    })

    return () => { chartRef.current?.destroy() }
  }, [labels, datasets])

  return (
    <div className={styles.wrapper} style={{ height }}>
      <canvas ref={canvasRef} />
    </div>
  )
}
