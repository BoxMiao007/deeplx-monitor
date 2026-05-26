import type { ReactElement } from 'react'

interface FlagProps {
  size?: number
}

const flags: Record<string, (props: FlagProps) => ReactElement> = {
  zh: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Chinese">
      <rect width="32" height="32" rx="4" fill="#DE2910" />
      <g fill="#FFDE00">
        <polygon points="8,4 9.2,7.6 13,7.6 9.9,9.8 11,13.4 8,11.2 5,13.4 6.1,9.8 3,7.6 6.8,7.6" />
        <polygon points="15,3 15.5,4.5 17,4.5 15.8,5.4 16.2,7 15,6 13.8,7 14.2,5.4 13,4.5 14.5,4.5" />
        <polygon points="18,6 18.5,7.5 20,7.5 18.8,8.4 19.2,10 18,9 16.8,10 17.2,8.4 16,7.5 17.5,7.5" />
        <polygon points="18,11 18.5,12.5 20,12.5 18.8,13.4 19.2,15 18,14 16.8,15 17.2,13.4 16,12.5 17.5,12.5" />
        <polygon points="15,14 15.5,15.5 17,15.5 15.8,16.4 16.2,18 15,17 13.8,18 14.2,16.4 13,15.5 14.5,15.5" />
      </g>
    </svg>
  ),
  en: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="English">
      <rect width="32" height="32" rx="4" fill="#012169" />
      <path d="M0,0 L32,32 M32,0 L0,32" stroke="#fff" strokeWidth="4" />
      <path d="M0,0 L32,32 M32,0 L0,32" stroke="#C8102E" strokeWidth="2" />
      <path d="M16,0 V32 M0,16 H32" stroke="#fff" strokeWidth="6" />
      <path d="M16,0 V32 M0,16 H32" stroke="#C8102E" strokeWidth="3.5" />
    </svg>
  ),
  ja: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Japanese">
      <rect width="32" height="32" rx="4" fill="#fff" />
      <circle cx="16" cy="16" r="8" fill="#BC002D" />
    </svg>
  ),
  ko: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Korean">
      <rect width="32" height="32" rx="4" fill="#fff" />
      <circle cx="16" cy="16" r="7" fill="#C60C30" />
      <path d="M16,9 A7,7 0 0,1 16,23 A3.5,3.5 0 0,1 16,16 A3.5,3.5 0 0,0 16,9" fill="#003478" />
    </svg>
  ),
  de: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="German">
      <rect width="32" height="11" rx="4" ry="4" fill="#000" />
      <rect y="11" width="32" height="10" fill="#DD0000" />
      <rect y="21" width="32" height="11" rx="4" ry="4" fill="#FFCE00" />
    </svg>
  ),
  fr: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="French">
      <rect width="11" height="32" rx="4" ry="4" fill="#002395" />
      <rect x="11" width="10" height="32" fill="#fff" />
      <rect x="21" width="11" height="32" rx="4" ry="4" fill="#ED2939" />
    </svg>
  ),
  es: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Spanish">
      <rect width="32" height="8" rx="4" ry="4" fill="#AA151B" />
      <rect y="8" width="32" height="16" fill="#F1BF00" />
      <rect y="24" width="32" height="8" rx="4" ry="4" fill="#AA151B" />
    </svg>
  ),
  ru: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Russian">
      <rect width="32" height="11" rx="4" ry="4" fill="#fff" />
      <rect y="11" width="32" height="10" fill="#0039A6" />
      <rect y="21" width="32" height="11" rx="4" ry="4" fill="#D52B1E" />
    </svg>
  ),
  pt: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Portuguese">
      <rect width="32" height="32" rx="4" fill="#009B3A" />
      <polygon points="16,6 28,16 16,26 4,16" fill="#FEDF00" />
      <circle cx="16" cy="16" r="5" fill="#002776" />
    </svg>
  ),
  it: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Italian">
      <rect width="11" height="32" rx="4" ry="4" fill="#009246" />
      <rect x="11" width="10" height="32" fill="#fff" />
      <rect x="21" width="11" height="32" rx="4" ry="4" fill="#CE2B37" />
    </svg>
  ),
  nl: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Dutch">
      <rect width="32" height="11" rx="4" ry="4" fill="#AE1C28" />
      <rect y="11" width="32" height="10" fill="#fff" />
      <rect y="21" width="32" height="11" rx="4" ry="4" fill="#21468B" />
    </svg>
  ),
  pl: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Polish">
      <rect width="32" height="16" rx="4" ry="4" fill="#fff" />
      <rect y="16" width="32" height="16" rx="4" ry="4" fill="#DC143C" />
    </svg>
  ),
  ar: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Arabic">
      <rect width="32" height="11" rx="4" ry="4" fill="#CE1126" />
      <rect y="11" width="32" height="10" fill="#fff" />
      <rect y="21" width="32" height="11" rx="4" ry="4" fill="#000" />
      <polygon points="0,0 0,32 8,16" fill="#007A3D" />
    </svg>
  ),
  tr: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Turkish">
      <rect width="32" height="32" rx="4" fill="#E30A17" />
      <circle cx="13" cy="16" r="7" fill="#fff" />
      <circle cx="15" cy="16" r="5.5" fill="#E30A17" />
      <polygon points="20,16 22,14.5 21,16.8 23,15.5 20.8,16 23,16.5 21,15.2 22,17.5" fill="#fff" />
    </svg>
  ),
  th: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Thai">
      <rect width="32" height="5" rx="4" ry="4" fill="#A51931" />
      <rect y="5" width="32" height="5" fill="#F4F5F8" />
      <rect y="10" width="32" height="12" fill="#2D2A4A" />
      <rect y="22" width="32" height="5" fill="#F4F5F8" />
      <rect y="27" width="32" height="5" rx="4" ry="4" fill="#A51931" />
    </svg>
  ),
  vi: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Vietnamese">
      <rect width="32" height="32" rx="4" fill="#DA251D" />
      <polygon points="16,7 18.5,13.5 25,13.5 19.8,17.5 21.8,24 16,20 10.2,24 12.2,17.5 7,13.5 13.5,13.5" fill="#FFFF00" />
    </svg>
  ),
  id: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Indonesian">
      <rect width="32" height="16" rx="4" ry="4" fill="#CE1126" />
      <rect y="16" width="32" height="16" rx="4" ry="4" fill="#fff" />
    </svg>
  ),
  uk: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Ukrainian">
      <rect width="32" height="16" rx="4" ry="4" fill="#005BBB" />
      <rect y="16" width="32" height="16" rx="4" ry="4" fill="#FFD500" />
    </svg>
  ),
  cs: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Czech">
      <rect width="32" height="16" rx="4" ry="4" fill="#fff" />
      <rect y="16" width="32" height="16" rx="4" ry="4" fill="#D7141A" />
      <polygon points="0,0 16,16 0,32" fill="#11457E" />
    </svg>
  ),
  sv: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Swedish">
      <rect width="32" height="32" rx="4" fill="#006AA7" />
      <rect x="9" width="5" height="32" fill="#FECC00" />
      <rect y="13" width="32" height="6" fill="#FECC00" />
    </svg>
  ),
  da: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Danish">
      <rect width="32" height="32" rx="4" fill="#C8102E" />
      <rect x="9" width="4" height="32" fill="#fff" />
      <rect y="13" width="32" height="6" fill="#fff" />
    </svg>
  ),
  fi: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Finnish">
      <rect width="32" height="32" rx="4" fill="#fff" />
      <rect x="8" width="5" height="32" fill="#003580" />
      <rect y="12" width="32" height="7" fill="#003580" />
    </svg>
  ),
  el: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Greek">
      <rect width="32" height="32" rx="4" fill="#0D5EAF" />
      <rect y="3.5" width="32" height="2.8" fill="#fff" />
      <rect y="10" width="32" height="2.8" fill="#fff" />
      <rect y="16.5" width="32" height="2.8" fill="#fff" />
      <rect y="23" width="32" height="2.8" fill="#fff" />
      <rect width="12" height="12" fill="#0D5EAF" />
      <rect x="5" width="2.5" height="12" fill="#fff" />
      <rect y="5" width="12" height="2.5" fill="#fff" />
    </svg>
  ),
  hu: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Hungarian">
      <rect width="32" height="11" rx="4" ry="4" fill="#CE2939" />
      <rect y="11" width="32" height="10" fill="#fff" />
      <rect y="21" width="32" height="11" rx="4" ry="4" fill="#477050" />
    </svg>
  ),
  ro: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Romanian">
      <rect width="11" height="32" rx="4" ry="4" fill="#002B7F" />
      <rect x="11" width="10" height="32" fill="#FCD116" />
      <rect x="21" width="11" height="32" rx="4" ry="4" fill="#CE1126" />
    </svg>
  ),
  bg: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Bulgarian">
      <rect width="32" height="11" rx="4" ry="4" fill="#fff" />
      <rect y="11" width="32" height="10" fill="#00966E" />
      <rect y="21" width="32" height="11" rx="4" ry="4" fill="#D62612" />
    </svg>
  ),
  sk: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Slovak">
      <rect width="32" height="11" rx="4" ry="4" fill="#fff" />
      <rect y="11" width="32" height="10" fill="#0B4EA2" />
      <rect y="21" width="32" height="11" rx="4" ry="4" fill="#EE1C25" />
    </svg>
  ),
  lt: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Lithuanian">
      <rect width="32" height="11" rx="4" ry="4" fill="#FDB913" />
      <rect y="11" width="32" height="10" fill="#006A44" />
      <rect y="21" width="32" height="11" rx="4" ry="4" fill="#C1272D" />
    </svg>
  ),
  lv: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Latvian">
      <rect width="32" height="13" rx="4" ry="4" fill="#9E3039" />
      <rect y="13" width="32" height="6" fill="#fff" />
      <rect y="19" width="32" height="13" rx="4" ry="4" fill="#9E3039" />
    </svg>
  ),
  et: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Estonian">
      <rect width="32" height="11" rx="4" ry="4" fill="#0072CE" />
      <rect y="11" width="32" height="10" fill="#000" />
      <rect y="21" width="32" height="11" rx="4" ry="4" fill="#fff" />
    </svg>
  ),
  sl: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Slovenian">
      <rect width="32" height="11" rx="4" ry="4" fill="#fff" />
      <rect y="11" width="32" height="10" fill="#003DA5" />
      <rect y="21" width="32" height="11" rx="4" ry="4" fill="#ED1C24" />
    </svg>
  ),
  nb: ({ size = 16 }) => (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Norwegian">
      <rect width="32" height="32" rx="4" fill="#EF2B2D" />
      <rect x="8" width="8" height="32" fill="#fff" />
      <rect y="11" width="32" height="10" fill="#fff" />
      <rect x="9.5" width="5" height="32" fill="#002868" />
      <rect y="12.5" width="32" height="7" fill="#002868" />
    </svg>
  ),
}

const autoFlag = ({ size = 16 }: FlagProps) => (
  <svg width={size} height={size} viewBox="0 0 32 32" aria-label="Auto">
    <rect width="32" height="32" rx="4" fill="var(--bg-hover)" />
    <circle cx="16" cy="16" r="9" fill="none" stroke="var(--text-tertiary)" strokeWidth="1.5" />
    <path d="M4,16 H28 M16,4 V28 M6,9 Q16,4 26,9 M6,23 Q16,28 26,23" fill="none" stroke="var(--text-tertiary)" strokeWidth="1" opacity="0.6" />
  </svg>
)

export function LangFlag({ lang, size = 16 }: { lang: string; size?: number }) {
  const code = lang.toLowerCase().trim()
  const FlagComponent = flags[code]
  if (FlagComponent) return <FlagComponent size={size} />
  if (code === 'auto' || code === '') return autoFlag({ size })
  return autoFlag({ size })
}
