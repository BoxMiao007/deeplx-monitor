import { useRef, useEffect } from 'react'
import { Chart, registerables } from 'chart.js'
import { useThemeStore } from '@/stores/useThemeStore'
import styles from './TrendChart.module.scss'

Chart.register(...registerables)

const CHART_COLORS: { base: string; light: string }[] = [
  { base: '#8b5cf6', light: '#d8b4fe' },
  { base: '#16a34a', light: '#86efac' },
  { base: '#d97706', light: '#fde68a' },
  { base: '#f59e0b', light: '#fde68a' },
  { base: '#06b6d4', light: '#a5f3fc' },
  { base: '#ef4444', light: '#fca5a5' },
  { base: '#6366f1', light: '#c7d2fe' },
  { base: '#ec4899', light: '#f9a8d4' },
]
const LINE_COLOR = '#ff3e20'

export interface TrendDataset {
  label: string
  data: number[]
  borderColor?: string
}

type LabelMode = 'time' | 'day' | 'full'

interface TrendChartProps {
  title: string
  labels: string[]
  lineDataset: TrendDataset
  barDatasets?: TrendDataset[]
  height?: number
  labelMode?: LabelMode
}

function formatLabel(raw: string, mode: LabelMode): string | string[] {
  if (mode === 'time') {
    const parts = raw.split(' ')
    return parts[1] ?? raw
  }
  if (mode === 'day') {
    const parts = raw.split('-')
    if (parts.length === 3) return `${parts[1]}-${parts[2]}`
    return raw
  }
  return raw
}

export function TrendChart({ title, labels, lineDataset, barDatasets = [], height = 280, labelMode = 'day' }: TrendChartProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const chartRef = useRef<Chart | null>(null)
  const { theme } = useThemeStore()
  const isDark = theme === 'dark'

  useEffect(() => {
    if (!canvasRef.current) return
    if (chartRef.current) chartRef.current.destroy()

    const gridColor = isDark ? 'rgba(255, 255, 255, 0.06)' : 'rgba(17, 24, 39, 0.06)'
    const axisBorderColor = isDark ? 'rgba(255, 255, 255, 0.10)' : 'rgba(17, 24, 39, 0.10)'
    const tickColor = isDark ? 'rgba(255, 255, 255, 0.72)' : 'rgba(17, 24, 39, 0.72)'
    const tooltipBg = isDark ? 'rgba(17, 24, 39, 0.92)' : 'rgba(255, 255, 255, 0.98)'
    const tooltipTitle = isDark ? '#ffffff' : '#111827'
    const tooltipBody = isDark ? 'rgba(255, 255, 255, 0.86)' : '#374151'
    const tooltipBorder = isDark ? 'rgba(255, 255, 255, 0.10)' : 'rgba(17, 24, 39, 0.10)'

    const datasets: object[] = [
      {
        type: 'line' as const,
        label: lineDataset.label,
        data: lineDataset.data,
        borderColor: lineDataset.borderColor ?? LINE_COLOR,
        backgroundColor: 'transparent',
        pointBackgroundColor: lineDataset.borderColor ?? LINE_COLOR,
        pointBorderColor: lineDataset.borderColor ?? LINE_COLOR,
        borderWidth: 2,
        borderDash: [6, 4],
        pointRadius: labels.length > 30 ? 0 : 3,
        pointHoverRadius: 5,
        tension: 0.35,
        fill: false,
        order: 0,
      },
      ...barDatasets.map((ds, i) => {
        const colorPair = CHART_COLORS[i % CHART_COLORS.length]
        return {
          type: 'bar' as const,
          label: ds.label,
          data: ds.data,
          backgroundColor: (context: { chart: Chart }) => {
            const { chart } = context
            const { ctx, chartArea } = chart
            if (!chartArea) return colorPair.base
            const gradient = ctx.createLinearGradient(0, chartArea.top, 0, chartArea.bottom)
            gradient.addColorStop(0, colorPair.light)
            gradient.addColorStop(1, colorPair.base)
            return gradient
          },
          borderColor: ds.borderColor ?? colorPair.base,
          borderWidth: 1,
          borderRadius: 3,
          stack: 'endpoints',
          order: 1,
        }
      }),
    ]

    const maxRotation = labelMode === 'full' ? 45 : 0

    const ctx = canvasRef.current.getContext('2d')!
    chartRef.current = new Chart(ctx, {
      type: 'bar',
      data: { labels, datasets: datasets as never[] },
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
            displayColors: true,
            usePointStyle: true,
          },
        },
        scales: {
          x: {
            stacked: true,
            grid: { color: gridColor, drawTicks: false },
            border: { color: axisBorderColor },
            ticks: {
              color: tickColor,
              font: { size: 11 },
              maxTicksLimit: labelMode === 'full' ? 12 : 10,
              maxRotation,
              minRotation: maxRotation > 0 ? maxRotation : 0,
              callback: (_value, index) => formatLabel(labels[index] ?? '', labelMode),
            },
          },
          y: {
            beginAtZero: true,
            grid: { color: gridColor },
            border: { color: axisBorderColor },
            ticks: { color: tickColor, font: { size: 11 } },
          },
        },
      },
    })

    return () => { chartRef.current?.destroy() }
  }, [labels, lineDataset, barDatasets, isDark, labelMode])

  return (
    <div className={styles.chartWrapper}>
      <div className={styles.chartHeader}>
        <h3 className={styles.chartTitle}>{title}</h3>
        {barDatasets.length > 0 && (
          <div className={styles.chartLegend}>
            <span className={styles.legendItem}>
              <span className={styles.legendLine} style={{ backgroundColor: lineDataset.borderColor ?? LINE_COLOR }} />
              <span className={styles.legendLabel}>{lineDataset.label}</span>
            </span>
            {barDatasets.map((ds, i) => (
              <span key={ds.label} className={styles.legendItem}>
                <span className={styles.legendDot} style={{ backgroundColor: ds.borderColor ?? CHART_COLORS[i % CHART_COLORS.length].base }} />
                <span className={styles.legendLabel}>{ds.label}</span>
              </span>
            ))}
          </div>
        )}
      </div>
      <div className={styles.chartArea} style={{ height }}>
        <canvas ref={canvasRef} />
      </div>
    </div>
  )
}
