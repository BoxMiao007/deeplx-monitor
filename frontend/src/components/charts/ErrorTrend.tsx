import { useRef, useEffect } from 'react'
import { Chart, registerables } from 'chart.js'
import { useThemeStore } from '@/stores/useThemeStore'
import type { ErrorTrendPoint } from '@/types'

Chart.register(...registerables)

interface ErrorTrendProps {
  data: ErrorTrendPoint[]
  height?: number
}

export function ErrorTrend({ data, height = 220 }: ErrorTrendProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const chartRef = useRef<Chart | null>(null)
  const { theme } = useThemeStore()
  const isDark = theme === 'dark'

  useEffect(() => {
    if (!canvasRef.current) return
    if (chartRef.current) chartRef.current.destroy()

    const labels = data.map(p => p.time)
    const errorRates = data.map(p => p.error_rate * 100)

    const tooltipBg = isDark ? 'rgba(17, 24, 39, 0.92)' : 'rgba(255, 255, 255, 0.98)'
    const tooltipTitle = isDark ? '#ffffff' : '#111827'
    const tooltipBody = isDark ? 'rgba(255, 255, 255, 0.86)' : '#374151'
    const tooltipBorder = isDark ? 'rgba(255, 255, 255, 0.10)' : 'rgba(17, 24, 39, 0.10)'
    const tickColor = isDark ? 'rgba(255, 255, 255, 0.72)' : 'rgba(17, 24, 39, 0.72)'

    const ctx = canvasRef.current.getContext('2d')!
    chartRef.current = new Chart(ctx, {
      type: 'line',
      data: {
        labels,
        datasets: [{
          label: 'Error Rate',
          data: errorRates,
          borderColor: getComputedStyle(document.documentElement).getPropertyValue('--color-error').trim() || '#ef4444',
          backgroundColor: 'rgba(239, 68, 68, 0.08)',
          borderWidth: 2,
          pointRadius: 0,
          pointHoverRadius: 5,
          tension: 0.3,
          fill: true,
        }],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        interaction: { mode: 'index', intersect: false },
        plugins: {
          legend: { display: false },
          tooltip: {
            backgroundColor: tooltipBg,
            titleColor: tooltipTitle,
            bodyColor: tooltipBody,
            borderColor: tooltipBorder,
            borderWidth: 1,
            padding: 10,
            displayColors: false,
            callbacks: {
              title: (items) => items[0]?.label ?? '',
              label: (item) => {
                const point = data[item.dataIndex]
                if (!point) return ''
                return [
                  `错误率: ${(point.error_rate * 100).toFixed(1)}%`,
                  `错误数: ${point.errors}`,
                  `总请求: ${point.total}`,
                ]
              },
            },
          },
        },
        scales: {
          x: { grid: { display: false }, ticks: { color: tickColor, maxTicksLimit: 8, font: { size: 11 } } },
          y: { beginAtZero: true, ticks: { color: tickColor, callback: (v) => `${v}%`, font: { size: 11 } } },
        },
      },
    })

    return () => { chartRef.current?.destroy() }
  }, [data, isDark])

  return (
    <div style={{ position: 'relative', height }}>
      <canvas ref={canvasRef} />
    </div>
  )
}
