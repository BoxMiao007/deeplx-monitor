import { SelectHTMLAttributes } from 'react'
import styles from './Select.module.scss'

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string
  options: { value: string | number; label: string }[]
}

export function Select({ label, options, className, ...props }: SelectProps) {
  return (
    <label className={styles.wrapper}>
      {label && <span className={styles.label}>{label}</span>}
      <select className={`${styles.select} ${className ?? ''}`} {...props}>
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>{opt.label}</option>
        ))}
      </select>
    </label>
  )
}
