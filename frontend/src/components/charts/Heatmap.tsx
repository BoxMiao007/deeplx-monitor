import { useState, useCallback, useRef, useEffect } from 'react'
import { createPortal } from 'react-dom'
import { apiFetch } from '@/hooks/useApi'
import styles from './Heatmap.module.scss'

const GRID_COLS = 96
const GRID_ROWS = 7
const TOTAL_BLOCKS = GRID_COLS * GRID_ROWS

function intensityToGradient(rate: number): string {
  const t = Math.max(0, Math.min(1, rate))
  let topR: number, topG: number, topB: number
  let botR: number, botG: number, botB: number
  if (t < 0.5) {
    const lt = t * 2
    topR = Math.round(255 + (226 - 255) * lt)
    topG = Math.round(250 + (181 - 250) * lt)
    topB = Math.round(238 + (98 - 238) * lt)
    botR = Math.round(250 + (214 - 250) * lt)
    botG = Math.round(244 + (162 - 244) * lt)
    botB = Math.round(230 + (76 - 230) * lt)
  } else {
    const lt = (t - 0.5) * 2
    topR = Math.round(226 + (214 - 226) * lt)
    topG = Math.round(181 + (118 - 181) * lt)
    topB = Math.round(98 + (96 - 98) * lt)
    botR = Math.round(214 + (198 - 214) * lt)
    botG = Math.round(162 + (87 - 162) * lt)
    botB = Math.round(76 + (70 - 76) * lt)
  }
  return `linear-gradient(180deg, rgb(${topR}, ${topG}, ${topB}) 0%, rgb(${botR}, ${botG}, ${botB}) 100%)`
}

function formatBlockTime(index: number, days: number): string {
  const totalMs = days * 86400 * 1000
  const bucketMs = totalMs / TOTAL_BLOCKS
  const now = Date.now()
  const startMs = now - totalMs + index * bucketMs
  const endMs = startMs + bucketMs
  const start = new Date(startMs)
  const end = new Date(endMs)
  const fmt = (d: Date) => {
    const mm = String(d.getMonth() + 1).padStart(2, '0')
    const dd = String(d.getDate()).padStart(2, '0')
    const hh = String(d.getHours()).padStart(2, '0')
    const mi = String(d.getMinutes()).padStart(2, '0')
    return `${mm}/${dd} ${hh}:${mi}`
  }
  return `${fmt(start)} – ${fmt(end)}`
}

interface TimelineBlock {
  index: number
  count: number
}

interface TooltipState {
  idx: number
  left: number
  top: number
  transform: string
}

interface HeatmapProps {
  days: number
}

export function Heatmap({ days }: HeatmapProps) {
  const [blocks, setBlocks] = useState<number[]>(() => new Array(TOTAL_BLOCKS).fill(0))
  const [activeTooltip, setActiveTooltip] = useState<TooltipState | null>(null)
  const gridRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    apiFetch<TimelineBlock[]>(`/api/analytics/timeline?days=${days}`).then(data => {
      const arr = new Array(TOTAL_BLOCKS).fill(0)
      data.forEach(b => {
        if (b.index >= 0 && b.index < TOTAL_BLOCKS) arr[b.index] = b.count
      })
      setBlocks(arr)
    }).catch(() => {})
  }, [days])

  const max = Math.max(...blocks, 1)

  useEffect(() => {
    if (!activeTooltip) return
    const handler = (e: PointerEvent) => {
      if (gridRef.current && !gridRef.current.contains(e.target as Node)) {
        setActiveTooltip(null)
      }
    }
    document.addEventListener('pointerdown', handler)
    return () => document.removeEventListener('pointerdown', handler)
  }, [activeTooltip])

  const buildTooltip = useCallback((idx: number, el: HTMLDivElement): TooltipState | null => {
    if (!el.isConnected) return null
    const rect = el.getBoundingClientRect()
    const centerX = rect.left + rect.width / 2
    let left = centerX
    let translateX = '-50%'
    if (centerX <= 90) { left = rect.left; translateX = '0' }
    else if (centerX >= window.innerWidth - 90) { left = rect.right; translateX = '-100%' }
    const above = rect.top > 72
    const top = above ? rect.top - 8 : rect.bottom + 8
    const translateY = above ? '-100%' : '0'
    return { idx, left: Math.round(left), top: Math.round(top), transform: `translate(${translateX}, ${translateY})` }
  }, [])

  const handlePointerEnter = useCallback((e: React.PointerEvent<HTMLDivElement>, idx: number) => {
    if (e.pointerType === 'mouse') setActiveTooltip(buildTooltip(idx, e.currentTarget))
  }, [buildTooltip])

  const handlePointerLeave = useCallback((e: React.PointerEvent) => {
    if (e.pointerType === 'mouse') setActiveTooltip(null)
  }, [])

  const handlePointerDown = useCallback((e: React.PointerEvent<HTMLDivElement>, idx: number) => {
    if (e.pointerType === 'touch') {
      e.preventDefault()
      setActiveTooltip(prev => prev?.idx === idx ? null : buildTooltip(idx, e.currentTarget))
    }
  }, [buildTooltip])

  const activeBlock = activeTooltip !== null ? { count: blocks[activeTooltip.idx], index: activeTooltip.idx } : null

  return (
    <div className={styles.card}>
      <div className={styles.gridScroller}>
        <div className={styles.grid} ref={gridRef}>
          {blocks.map((count, idx) => {
            const intensity = count > 0 ? Math.pow(count / max, 0.6) : -1
            const isIdle = intensity === -1
            const isActive = activeTooltip?.idx === idx
            return (
              <div
                key={idx}
                className={`${styles.blockWrapper} ${isActive ? styles.blockActive : ''}`}
                onPointerEnter={(e) => handlePointerEnter(e, idx)}
                onPointerLeave={handlePointerLeave}
                onPointerDown={(e) => handlePointerDown(e, idx)}
              >
                <div
                  className={`${styles.block} ${isIdle ? styles.blockIdle : ''}`}
                  style={isIdle ? undefined : { background: intensityToGradient(intensity) }}
                />
              </div>
            )
          })}
        </div>
      </div>
      <div className={styles.legend}>
        <span className={styles.legendLabel}>少</span>
        <div className={styles.legendColors}>
          <div className={`${styles.legendBlock} ${styles.blockIdle}`} />
          <div className={styles.legendRamp} />
        </div>
        <span className={styles.legendLabel}>多</span>
      </div>
      {activeTooltip && activeBlock && createPortal(
        <div
          className={styles.tooltip}
          style={{ position: 'fixed', left: activeTooltip.left, top: activeTooltip.top, transform: activeTooltip.transform }}
        >
          <span className={styles.tooltipTime}>{formatBlockTime(activeBlock.index, days)}</span>
          <span className={styles.tooltipCount}>{activeBlock.count} 次请求</span>
        </div>,
        document.body
      )}
    </div>
  )
}
