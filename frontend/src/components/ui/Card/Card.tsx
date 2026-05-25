import { HTMLAttributes } from 'react'
import styles from './Card.module.scss'

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  padding?: 'sm' | 'md' | 'lg'
}

export function Card({ padding = 'md', className, children, ...props }: CardProps) {
  return (
    <div className={`${styles.card} ${styles[padding]} ${className ?? ''}`} {...props}>
      {children}
    </div>
  )
}
