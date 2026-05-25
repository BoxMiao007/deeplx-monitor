import { useEffect, useRef } from 'react'

export function useAutoRefresh(callback: () => void, intervalSeconds: number) {
  const callbackRef = useRef(callback)
  callbackRef.current = callback

  useEffect(() => {
    if (intervalSeconds <= 0) return

    let timer: ReturnType<typeof setTimeout>
    let paused = false

    const tick = () => {
      if (!paused) callbackRef.current()
      timer = setTimeout(tick, intervalSeconds * 1000)
    }

    timer = setTimeout(tick, intervalSeconds * 1000)

    const onVisibility = () => {
      paused = document.hidden
      if (!paused) callbackRef.current()
    }

    document.addEventListener('visibilitychange', onVisibility)

    return () => {
      clearTimeout(timer)
      document.removeEventListener('visibilitychange', onVisibility)
    }
  }, [intervalSeconds])
}
