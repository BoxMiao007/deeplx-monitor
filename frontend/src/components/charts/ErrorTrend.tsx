import { useRef, useEffect } from 'react'
import { Chart, registerables } from 'chart.js'

Chart.register(...registerables)

interface ErrorTrendProps {
  labels: string[]
  errorRates: number[]
  height?: number
}

export function ErrorTrend({ labels, errorRates, height = 220 }: ErrorTrendProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const chartRef = useRef<Chart | null>(null)

  useEffect(() => {
    if (!canvasRef.current) return
    if (chartRef.current) chartRef.current.destroy()

    const ctx = canvasRef.current.getContext('2d')!
    chartRef.current = new Chart(ctx, {
      type: 'line',
      data: {
        labels,
        datasets: [{
          label: 'Error Rate %',
          data: errorRates,
          borderColor: getComputedStyle(document.documentElement).getPropertyValue('--color-error').trim(),
          backgroundColor: 'rgba(198, 87, 70, 0.1)',
          borderWidth: 2,
          pointRadius: 0,
          tension: 0.3,
          fill: true,
        }],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { display: false },
          tooltip: { backgroundColor: 'rgba(0,0,0,0.8)', padding: 10, cornerRadius: 8 },
        },
        scales: {
          x: { grid: { display: false }, ticks: { maxTicksLimit: 8, font: { size: 11 } } },
          y: { beginAtZero: true, max: 100, ticks: { callback: (v) => `${v}%`, font: { size: 11 } } },
        },
      },
    })

    return () => { chartRef.current?.destroy() }
  }, [labels, errorRates])

  return (
    <div style={{ position: 'relative', height }}>
      <canvas ref={canvasRef} />
    </div>
  )
}
