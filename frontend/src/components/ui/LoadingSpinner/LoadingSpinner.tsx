import styles from './LoadingSpinner.module.scss'

export function LoadingSpinner({ size = 24 }: { size?: number }) {
  return <div className={styles.spinner} style={{ width: size, height: size }} />
}
