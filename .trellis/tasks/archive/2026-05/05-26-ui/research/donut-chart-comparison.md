# Research: Donut Chart Comparison (Reference vs Current)

- **Query**: Compare the reference donut chart in cpa-usage-keeper with the current DonutChart in deeplx-monitor; identify styling/layout/legend differences and what needs to change.
- **Scope**: internal
- **Date**: 2026-05-26

## Reference Implementation (cpa-usage-keeper)

### Files

| File Path | Description |
|---|---|
| `/home/fedora007/mydev/github/cpa-usage-keeper/web/src/components/usage/analysis/AnalysisPanel.tsx` | Contains `CompositionDonutChart` component (lines 419–456) |
| `/home/fedora007/mydev/github/cpa-usage-keeper/web/src/components/usage/analysis/AnalysisPanel.module.scss` | Styles for donut layout, legend, card surface |

### Component Structure (CompositionDonutChart, lines 419–456)

```tsx
<section className={styles.analysisCard}>
  <div className={styles.cardHeader}>
    <div>
      <h2>{title}</h2>
      <p>{subtitle}</p>
    </div>
  </div>
  <div className={styles.analysisChartSurface}>
    <div className={styles.donutLayout}>
      <div className={styles.donutChartFrame}>
        <Doughnut data={chartData} options={chartOptions} />
      </div>
      <div className={styles.compositionLegend}>
        {items.map((item, index) => (
          <div key={item.key} className={styles.compositionLegendRow}>
            <span className={styles.legendDot} style={{ backgroundColor: CHART_COLORS[index].base }} />
            <span className={styles.legendName}>{item.label}</span>
            <span className={styles.legendValue}>{formatPercent(item.percent)}</span>
          </div>
        ))}
      </div>
    </div>
  </div>
</section>
```

### Key Reference Styles

```scss
// Card container
.analysisCard {
  border: 1px solid var(--border-color);
  border-radius: 24px;
  background: var(--bg-primary);
  box-shadow: 0 18px 46px rgba(0, 0, 0, 0.07);
  padding: 20px;
}

// Inner chart surface (secondary bg with border)
.analysisChartSurface {
  padding: 14px;
  background: var(--bg-secondary);
  border-radius: $radius-lg;
  border: 1px solid var(--border-color);
}

// Donut + legend side-by-side grid
.donutLayout {
  display: grid;
  grid-template-columns: minmax(180px, 0.85fr) minmax(180px, 1fr);
  gap: 18px;
  align-items: center;
}

// Chart frame height
.donutChartFrame {
  height: 220px;
}

// Legend column
.compositionLegend {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

// Legend row: 3-column grid (dot | name | value)
.compositionLegendRow {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 8px;
  align-items: center;
  color: var(--text-secondary);
  font-size: 12px;
}

// Legend dot with outer glow
.legendDot {
  width: 9px;
  height: 9px;
  border-radius: 999px;
  box-shadow: 0 0 0 3px color-mix(in srgb, currentColor 12%, transparent);
}

// Legend name: bold, truncated
.legendName {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-primary);
  font-weight: 700;
}

// Legend value: extra bold, tabular nums
.legendValue {
  font-variant-numeric: tabular-nums;
  font-weight: 800;
  color: var(--text-primary);
}
```

### Reference Chart Options

- `cutout: '58%'`
- Custom external tooltip (DOM-based, positioned with `position: fixed`)
- Gradient fills (top-to-bottom linear gradient from `light` to `base` color)
- `legend: { display: false }` (custom legend rendered in JSX)
- Uses `react-chartjs-2` `<Doughnut>` component (not raw Chart.js)

### Reference Colors (same palette)

```ts
const CHART_COLORS = [
  { base: '#1d4ed8', light: '#60a5fa' },
  { base: '#ca8a04', light: '#facc15' },
  { base: '#15803d', light: '#22c55e' },
  { base: '#7e22ce', light: '#c084fc' },
  { base: '#b91c1c', light: '#ef4444' },
];
```

### Reference Card Header

Has both `<h2>` title (18px, bold) and `<p>` subtitle (13px, secondary color).

---

## Current Implementation (deeplx-monitor)

### Files

| File Path | Description |
|---|---|
| `/home/fedora007/mydev/github/deeplx-monitor/frontend/src/components/charts/DonutChart.tsx` | Current DonutChart component |
| `/home/fedora007/mydev/github/deeplx-monitor/frontend/src/components/charts/DonutChart.module.scss` | Current styles |
| `/home/fedora007/mydev/github/deeplx-monitor/frontend/src/pages/Analysis.tsx` | Page using DonutChart |
| `/home/fedora007/mydev/github/deeplx-monitor/frontend/src/pages/Analysis.module.scss` | Page-level styles |

### Current Component Structure

```tsx
<div className={styles.wrapper}>
  <h4 className={styles.title}>{title}</h4>
  <div className={styles.layout}>
    <div className={styles.chartFrame} style={{ height }}>
      <canvas ref={canvasRef} />
    </div>
    <div className={styles.legend}>
      {data.map((item, i) => (
        <div key={item.label} className={styles.legendItem}>
          <span className={styles.legendDot} style={{ backgroundColor: COLORS[i].base }} />
          <span className={styles.legendName}>{item.label}</span>
          <span className={styles.legendPct}>{pct}%</span>
        </div>
      ))}
    </div>
  </div>
</div>
```

### Current Styles

```scss
.wrapper {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.layout {
  display: grid;
  grid-template-columns: minmax(160px, 0.8fr) minmax(140px, 1fr);
  gap: 16px;
  align-items: center;
}

.chartFrame {
  position: relative;
  width: 100%;
}

.legend {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.legendItem {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.legendDot {
  width: 10px;
  height: 10px;
  border-radius: 3px;  // square-ish
  flex-shrink: 0;
}

.legendName {
  flex: 1;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.legendPct {
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  color: var(--text-primary);
}
```

---

## Differences Summary

| Aspect | Reference (cpa-usage-keeper) | Current (deeplx-monitor) |
|--------|------------------------------|--------------------------|
| **Outer container** | `.analysisCard` with `border-radius: 24px`, `box-shadow: 0 18px 46px`, `padding: 20px` | No dedicated card — sits inside `.statCardLarge` from Analysis page |
| **Inner surface** | `.analysisChartSurface` — secondary bg, border, `border-radius: $radius-lg`, `padding: 14px` | None — chart renders directly |
| **Title** | `<h2>` 18px + `<p>` subtitle 13px | `<h4>` 14px, no subtitle |
| **Layout grid** | `minmax(180px, 0.85fr) minmax(180px, 1fr)`, gap 18px | `minmax(160px, 0.8fr) minmax(140px, 1fr)`, gap 16px |
| **Chart height** | 220px | 180px (passed as prop) |
| **Legend gap** | 10px between rows | 8px between rows |
| **Legend row layout** | CSS Grid: `auto minmax(0,1fr) auto` | Flexbox with gap 8px |
| **Legend dot** | 9px, `border-radius: 999px` (circle), has `box-shadow: 0 0 0 3px color-mix(...)` glow | 10px, `border-radius: 3px` (rounded square), no shadow |
| **Legend name** | `font-weight: 700`, `color: var(--text-primary)` | No bold, `color: var(--text-secondary)` |
| **Legend value** | `font-weight: 800` | `font-weight: 600` |
| **Legend font size** | 12px | 13px |
| **Tooltip** | Custom external DOM tooltip with fixed positioning | Standard Chart.js tooltip (canvas-based) |
| **Chart library** | `react-chartjs-2` `<Doughnut>` component | Raw `Chart.js` via `useRef` + `useEffect` |

---

## Analysis Page Context (where DonutChart is used)

In `Analysis.tsx` (lines 72–118), when `endpoints.length > 1`, a `.statsGrid` is rendered containing:

1. **Two `.statCardLarge` (span 6 each)** — each contains a `<DonutChart>`:
   - "Request Share" donut
   - "Character Share" donut

2. **Three `.statCardSmall` (span 4 each)** — stat cards below the donuts:
   - Total Requests (accent: `#8b5cf6`)
   - Success Rate (accent: `#06b6d4`)
   - Avg Latency (accent: `#f59e0b`)

These three small stat cards are the ones "below the donut charts" that may need to be removed.

---

## Changes Needed to Match Reference

### DonutChart.module.scss changes:

1. Add `.analysisChartSurface` inner wrapper: `padding: 14px; background: var(--bg-secondary); border-radius: var(--radius-lg); border: 1px solid var(--border-color);`
2. `.layout` grid: change to `minmax(180px, 0.85fr) minmax(180px, 1fr)`, gap `18px`
3. `.chartFrame` height: increase to `220px`
4. `.legend` gap: `10px`
5. `.legendItem` → change to CSS Grid: `grid-template-columns: auto minmax(0, 1fr) auto; gap: 8px; font-size: 12px;`
6. `.legendDot`: change to `width: 9px; height: 9px; border-radius: 999px; box-shadow: 0 0 0 3px color-mix(in srgb, currentColor 12%, transparent);`
7. `.legendName`: add `font-weight: 700; color: var(--text-primary);`
8. `.legendPct`: change to `font-weight: 800;`
9. `.title`: increase to `font-size: 18px;` or wrap in card header with subtitle

### DonutChart.tsx changes:

1. Add inner `analysisChartSurface` wrapper div around the layout
2. Optionally add subtitle prop
3. Default height to 220 instead of 200

### Analysis.tsx changes (if removing stat cards):

- Remove the three `.statCardSmall` divs (lines 88–118)
- Adjust `.statsGrid` to only contain the two donut cards (could simplify to `grid-template-columns: repeat(2, 1fr)`)
