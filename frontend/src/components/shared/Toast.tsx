import { useEffect, useState, useCallback } from 'react'
import { setToastHandler } from '@/hooks/useApi'
import styles from './Toast.module.scss'

interface ToastItem {
  id: number
  message: string
}

let nextId = 0

export function ToastContainer() {
  const [toasts, setToasts] = useState<ToastItem[]>([])

  const addToast = useCallback((message: string) => {
    const id = nextId++
    setToasts((prev) => [...prev, { id, message }])
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id))
    }, 3000)
  }, [])

  useEffect(() => {
    setToastHandler(addToast)
  }, [addToast])

  return (
    <div className={styles.container}>
      {toasts.map((toast) => (
        <div key={toast.id} className={styles.toast}>{toast.message}</div>
      ))}
    </div>
  )
}
