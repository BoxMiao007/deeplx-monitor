import { InputHTMLAttributes, forwardRef } from 'react'
import styles from './Input.module.scss'

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string
}

export const Input = forwardRef<HTMLInputElement, InputProps>(({ label, className, ...props }, ref) => (
  <label className={styles.wrapper}>
    {label && <span className={styles.label}>{label}</span>}
    <input ref={ref} className={`${styles.input} ${className ?? ''}`} {...props} />
  </label>
))

Input.displayName = 'Input'
