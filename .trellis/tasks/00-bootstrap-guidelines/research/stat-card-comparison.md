# Research: Stat Card Top-Right Corner Comparison

- **Query**: Compare stat card top-right corner content between cpa-usage-keeper and deeplx-monitor
- **Scope**: internal
- **Date**: 2026-05-26

## Findings

### Reference: cpa-usage-keeper StatCards

**File**: `/home/fedora007/mydev/github/cpa-usage-keeper/web/src/components/usage/StatCards.tsx`

#### Top-Right Corner Content: Icon Badge

Each card has a **colored icon badge** in the top-right corner, rendered via:

```tsx
<span className={styles.statIconBadge}>{card.icon}</span>
```

The badge is positioned inside a flex header with `justify-content: space-between`:

```tsx
<div className={styles.statCardHeader}>
  <div className={styles.statLabelGroup}>
    <span className={styles.statLabel}>{card.label}</span>
  </div>
  <span className={styles.statIconBadge}>{card.icon}</span>
</div>
```

#### Icon Badge Styling (UsagePage.module.scss:1189)

```scss
.statIconBadge {
  width: 34px;
  height: 34px;
  border-radius: $radius-md;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 13px;
  background: var(--accent);          // uses card's accent color
  border: 1px solid rgba(255, 255, 255, 0.08);
  box-shadow: 0 10px 22px rgba(0, 0, 0, 0.25);
  flex-shrink: 0;

  svg {
    display: block;
  }
}
```

#### Icons per Card

| Card | Icon Component | Description |
|------|---------------|-------------|
| Total Requests | `<IconSatellite size={16} />` | Satellite/signal icon (Lucide) |
| Total Tokens | `<IconDiamond size={16} />` | Diamond shape icon |
| RPM | `<IconTimer size={16} />` | Timer/clock icon |
| TPM | `<IconTrendingUp size={16} />` | Trending up arrow icon |
| Total Cost | `<IconDollarSign size={16} />` | Dollar sign icon |

All icons are 16px Lucide SVG icons, rendered white on the accent-colored background.

---

### Current: deeplx-monitor StatCard

**File**: `/home/fedora007/mydev/github/deeplx-monitor/frontend/src/components/shared/StatCard.tsx`

#### Top-Right Corner Content: Subtitle Text

The current card has NO icon badge. The top-right corner shows a **text subtitle** (e.g., "cumulative" or "trend"):

```tsx
<div className={styles.head}>
  <span className={styles.label}>{label}</span>
  {subtitle && <span className={styles.subtitle}>{subtitle}</span>}
</div>
```

The `.head` uses `justify-content: space-between`, so the subtitle appears at the right side.

#### Subtitle Styling (StatCard.module.scss:46-49)

```scss
.subtitle {
  font-size: 12px;
  color: var(--text-tertiary);
}
```

It's just plain gray text, no background, no icon, no badge.

---

### Accent Colors for deeplx-monitor Overview Cards

| Card | Label (i18n key) | chartColor / --accent |
|------|-------------------|----------------------|
| 总调用次数 (Total Requests) | `overview.totalRequests` | `#8b5cf6` (purple) |
| 总字符数 (Total Chars) | `overview.totalChars` | `#22c55e` (green) |
| 成功率 (Success Rate) | `overview.successRate` | `#06b6d4` (cyan) |
| 平均延迟 (Avg Latency) | `overview.avgLatency` | `#f97316` (orange) |

The `--accent` CSS variable is used for:
1. Top border gradient: `linear-gradient(90deg, var(--accent), transparent)` (3px height)
2. Background radial gradient: `color-mix(in srgb, var(--accent) 18%, transparent)`
3. MiniChart sparkline color

---

## Summary of Differences

| Aspect | cpa-usage-keeper | deeplx-monitor |
|--------|-----------------|----------------|
| Top-right element | 34x34px icon badge (rounded square, accent bg, white SVG icon, heavy shadow) | Plain text subtitle ("累计" / "趋势") |
| Icon | Lucide SVG 16px, white on accent | None |
| Badge background | `var(--accent)` solid color | N/A |
| Badge shadow | `0 10px 22px rgba(0,0,0,0.25)` | N/A |
| Badge border | `1px solid rgba(255,255,255,0.08)` | N/A |

## Caveats / Not Found

- The cpa-usage-keeper icons are all inline SVG components defined in `icons.tsx`, not from an icon library package. If deeplx-monitor wants to replicate this pattern, it would need to either add similar inline SVG icons or use a library like `lucide-react`.
