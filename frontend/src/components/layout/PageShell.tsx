import { type ReactNode } from 'react'
import styles from './PageShell.module.scss'

export function PageShell({ children }: { children: ReactNode }) {
  return (
    <main className={styles.shell}>
      <div className={styles.content}>{children}</div>
    </main>
  )
}
