import { ButtonHTMLAttributes } from 'react'
import styles from './Button.module.scss'

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost'
  size?: 'sm' | 'md' | 'lg'
}

export function Button({ variant = 'primary', size = 'md', className, children, ...props }: ButtonProps) {
  return (
    <button className={`${styles.btn} ${styles[variant]} ${styles[size]} ${className ?? ''}`} {...props}>
      {children}
    </button>
  )
}
