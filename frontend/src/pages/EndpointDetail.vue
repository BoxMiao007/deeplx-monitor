<template>
  <div class="dashboard-page">
    <div class="container dashboard-shell">
      <section class="hero-panel card">
        <div class="hero-copy">
          <div class="hero-logo">
            <img src="@/assets/deepl-logo.svg" width="22" height="22" class="hero-logo-img" alt="DeepLX">
          </div>
          <h1 class="hero-title">DeepLX 监控</h1>
        </div>
        <div class="hero-controls">
          <label class="control-field">
            <span class="control-label">范围</span>
            <select v-model="headerRange" @change="onRangeChange" class="input control-select">
              <option :value="1">今天</option>
              <option :value="7">7 天</option>
              <option :value="30">30 天</option>
              <option :value="90">90 天</option>
              <option :value="0">所有</option>
            </select>
          </label>
        </div>
      </section>

      <TabNav />

      <section class="endpoint-buttons card">
        <button :class="['ep-btn']" @click="$router.push('/endpoints')">全部对比</button>
        <button
          v-for="ep in store.upstreamStatus"
          :key="ep.name"
          :class="['ep-btn', { active: ep.name === endpointName }]"
          @click="$router.push(`/endpoints/${encodeURIComponent(ep.name)}`)"
        >
          <span :class="['dot', ep.healthy ? 'dot-ok' : 'dot-error']"></span>
          {{ ep.name }}
        </button>
      </section>

      <section class="summary-grid">
        <article class="stat-card card emphasis-card">
          <div class="stat-head">
            <span class="stat-label">总调用次数</span>
            <span class="stat-trend">累计总数</span>
          </div>
          <div class="stat-value number">{{ formatNumber(epStats?.total_requests ?? 0) }}</div>
        </article>
        <article class="stat-card card emphasis-card">
          <div class="stat-head">
            <span class="stat-label">总字符数</span>
            <span class="stat-trend">累计总数</span>
          </div>
          <div class="stat-value number">{{ formatNumber(epStats?.total_chars ?? 0) }}</div>
        </article>
      </section>

      <section class="period-grid">
        <article class="period-card card">
          <div class="period-label">今日</div>
          <div class="period-row"><span class="period-key">调用</span><span class="period-value number">{{ formatNumber(epStats?.period?.today_requests ?? 0) }}</span></div>
          <div class="period-row"><span class="period-key">字符</span><span class="period-value number">{{ formatNumber(epStats?.period?.today_chars ?? 0) }}</span></div>
        </article>
        <article class="period-card card">
          <div class="period-label">近 7 天</div>
          <div class="period-row"><span class="period-key">调用</span><span class="period-value number">{{ formatNumber(epStats?.period?.week_requests ?? 0) }}</span></div>
          <div class="period-row"><span class="period-key">字符</span><span class="period-value number">{{ formatNumber(epStats?.period?.week_chars ?? 0) }}</span></div>
        </article>
        <article class="period-card card">
          <div class="period-label">近 30 天</div>
          <div class="period-row"><span class="period-key">调用</span><span class="period-value number">{{ formatNumber(epStats?.period?.month_requests ?? 0) }}</span></div>
          <div class="period-row"><span class="period-key">字符</span><span class="period-value number">{{ formatNumber(epStats?.period?.month_chars ?? 0) }}</span></div>
        </article>
      </section>

      <section class="dashboard-grid">
        <article class="card section-card">
          <h2 class="section-title">语言统计</h2>
          <div class="table-scroll">
            <table class="lang-table-simple">
              <thead>
                <tr><th>语言</th><th>源字符</th><th>目标字符</th></tr>
              </thead>
              <tbody>
                <tr v-if="!langStats.length"><td colspan="3" class="text-center text-muted">暂无数据</td></tr>
                <tr v-for="stat in langStats" :key="stat.lang">
                  <td>{{ stat.lang }}</td>
                  <td class="number">{{ formatNumber(stat.source_chars) }}</td>
                  <td class="number">{{ formatNumber(stat.target_chars) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </article>

        <article class="card section-card">
          <div class="section-topbar">
            <h2 class="section-title">调用趋势</h2>
            <div class="chart-tabs">
              <button :class="['chart-tab', { active: chartMode === 'hourly' }]" @click="chartMode = 'hourly'; fetchChartData()">24h</button>
              <button :class="['chart-tab', { active: chartMode === 'daily' }]" @click="chartMode = 'daily'; fetchChartData()">30天</button>
            </div>
          </div>
          <div class="chart-wrap panel-surface">
            <canvas ref="trendCanvas"></canvas>
          </div>
        </article>
      </section>

      <section class="analytics-grid">
        <article class="card section-card">
          <div class="section-topbar">
            <h2 class="section-title">错误率趋势</h2>
            <div class="chart-tabs">
              <button :class="['chart-tab', { active: errorDays === 1 }]" @click="errorDays = 1; fetchErrorTrend()">24h</button>
              <button :class="['chart-tab', { active: errorDays === 7 }]" @click="errorDays = 7; fetchErrorTrend()">7天</button>
              <button :class="['chart-tab', { active: errorDays === 30 }]" @click="errorDays = 30; fetchErrorTrend()">30天</button>
            </div>
          </div>
          <div class="chart-wrap panel-surface">
            <canvas ref="errorCanvas"></canvas>
          </div>
        </article>

        <article class="card section-card">
          <div class="section-topbar">
            <h2 class="section-title">活动热力图</h2>
            <div class="chart-tabs">
              <button :class="['chart-tab', { active: heatmapView === 'weekday' }]" @click="heatmapView = 'weekday'; fetchHeatmap()">星期</button>
              <button :class="['chart-tab', { active: heatmapView === 'date' }]" @click="heatmapView = 'date'; fetchHeatmap()">日期</button>
            </div>
          </div>
          <div class="heatmap-container">
            <div v-if="!heatmapData.length" class="text-center text-muted" style="padding: 2rem;">暂无数据</div>
            <div v-else class="heatmap-grid" :style="heatmapGridStyle">
              <div
                v-for="(cell, idx) in heatmapData"
                :key="idx"
                class="heatmap-cell"
                :style="heatmapCellStyle(cell.count)"
                :data-tooltip="`${cell.x} ${cell.y}:00 — ${cell.count} 次请求`"
              ></div>
            </div>
          </div>
        </article>
      </section>

      <section class="card section-card">
        <div class="section-topbar">
          <h2 class="section-title">请求日志</h2>
          <div class="request-toolbar">
            <label class="page-size-select">
              <span>每页</span>
              <select v-model="pageSize" @change="currentPage = 1; fetchRequests()" class="input page-size-input">
                <option :value="50">50</option>
                <option :value="100">100</option>
              </select>
              <span>条</span>
            </label>
          </div>
        </div>
        <div class="request-summary">
          <span class="table-count">共 {{ requestsTotal }} 条</span>
          <span class="table-count text-muted">第 {{ currentPage }} / {{ totalPages }} 页</span>
        </div>
        <div class="table-scroll">
          <table class="request-table">
            <thead>
              <tr>
                <th>时间</th>
                <th>源语言</th>
                <th>源字符</th>
                <th>目标语言</th>
                <th>目标字符</th>
                <th>状态</th>
                <th>错误</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="!requestItems.length"><td colspan="7" class="text-center text-muted">暂无数据</td></tr>
              <tr v-for="item in requestItems" :key="item.id">
                <td class="number time-cell">{{ formatTime(item.created_at) }}</td>
                <td>{{ item.source_lang }}</td>
                <td class="number">{{ formatNumber(item.source_chars) }}</td>
                <td>{{ item.target_lang }}</td>
                <td class="number">{{ formatNumber(item.target_chars) }}</td>
                <td><span :class="['badge', item.status === 'success' ? 'badge-success' : 'badge-error']">{{ item.status === 'success' ? '成功' : '失败' }}</span></td>
                <td><span class="error-message">{{ item.error_msg || '-' }}</span></td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="pagination-bar" v-if="totalPages > 1">
          <button class="btn-secondary btn-sm" :disabled="currentPage <= 1" @click="currentPage--; fetchRequests()">上一页</button>
          <button class="btn-secondary btn-sm" :disabled="currentPage >= totalPages" @click="currentPage++; fetchRequests()">下一页</button>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useMonitorStore } from '@/stores/monitor'
import TabNav from '@/components/TabNav.vue'

import type { Chart as ChartType } from 'chart.js'
let Chart: typeof ChartType | null = null
async function ensureChartJs() {
  if (Chart) return
  const mod = await import('chart.js')
  Chart = mod.Chart
  Chart.register(mod.LineController, mod.LineElement, mod.PointElement, mod.LinearScale, mod.CategoryScale, mod.Filler, mod.Tooltip, mod.Legend)
}

const route = useRoute()
const store = useMonitorStore()

const endpointName = computed(() => decodeURIComponent(route.params.name as string))
const headerRange = ref(0)
const HEADER_RANGE_STORAGE_KEY = 'deeplx-monitor.header-range'

// 数据状态
const epStats = ref<any>(null)
const langStats = ref<Array<{ lang: string; source_chars: number; target_chars: number }>>([])
const chartMode = ref<'hourly' | 'daily'>('hourly')
const chartData = ref<any>(null)
const errorDays = ref(7)
const errorTrendData = ref<any[]>([])
const heatmapView = ref('weekday')
const heatmapData = ref<any[]>([])
const requestItems = ref<any[]>([])
const requestsTotal = ref(0)
const currentPage = ref(1)
const pageSize = ref(50)
const totalPages = computed(() => Math.max(1, Math.ceil(requestsTotal.value / pageSize.value)))

const trendCanvas = ref<HTMLCanvasElement | null>(null)
const errorCanvas = ref<HTMLCanvasElement | null>(null)
let trendChart: ChartType | null = null
let errorChart: ChartType | null = null

function onRangeChange() {
  localStorage.setItem(HEADER_RANGE_STORAGE_KEY, String(headerRange.value))
}

const _numFmt = new Intl.NumberFormat()
function formatNumber(n: number) { return _numFmt.format(n) }

function formatTime(ts: string) {
  if (!ts) return '-'
  try {
    const d = new Date(ts)
    return d.toLocaleString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false })
  } catch { return ts }
}

const heatmapGridStyle = computed(() => {
  if (!heatmapData.value.length) return {}
  const xValues = [...new Set(heatmapData.value.map((d: any) => d.x))]
  return { display: 'grid', gridTemplateColumns: `repeat(${xValues.length}, 1fr)`, gridTemplateRows: 'repeat(24, 1fr)', gap: '2px' }
})

function heatmapCellStyle(count: number) {
  if (!heatmapData.value.length) return {}
  const max = Math.max(...heatmapData.value.map((d: any) => d.count), 1)
  const intensity = count / max
  return { backgroundColor: `rgba(59, 130, 246, ${0.1 + intensity * 0.8})`, borderRadius: '2px', minHeight: '12px' }
}

async function fetchStats() {
  try {
    const res = await fetch(`/api/stats?endpoint=${encodeURIComponent(endpointName.value)}`)
    if (res.ok) epStats.value = await res.json()
  } catch { /* ignore */ }
}

async function fetchLangStats() {
  try {
    const res = await fetch(`/api/lang-stats?endpoint=${encodeURIComponent(endpointName.value)}`)
    if (res.ok) {
      const raw = await res.json()
      // 合并同语言行
      const merged = new Map<string, { lang: string; source_chars: number; target_chars: number }>()
      for (const row of raw) {
        const key = row.lang.toUpperCase()
        const cur = merged.get(key) ?? { lang: key, source_chars: 0, target_chars: 0 }
        cur.source_chars += row.source_chars
        cur.target_chars += row.target_chars
        merged.set(key, cur)
      }
      langStats.value = [...merged.values()].sort((a, b) => (b.source_chars + b.target_chars) - (a.source_chars + a.target_chars))
    }
  } catch { /* ignore */ }
}

async function fetchChartData() {
  try {
    const res = await fetch(`/api/chart?endpoint=${encodeURIComponent(endpointName.value)}`)
    if (res.ok) {
      chartData.value = await res.json()
      await ensureChartJs()
      renderTrendChart()
    }
  } catch { /* ignore */ }
}

async function fetchErrorTrend() {
  try {
    const res = await fetch(`/api/analytics/error-trend?days=${errorDays.value}&endpoint=${encodeURIComponent(endpointName.value)}`)
    if (res.ok) {
      errorTrendData.value = await res.json()
      await ensureChartJs()
      renderErrorChart()
    }
  } catch { /* ignore */ }
}

async function fetchHeatmap() {
  try {
    const res = await fetch(`/api/analytics/heatmap?view=${heatmapView.value}&days=30&endpoint=${encodeURIComponent(endpointName.value)}`)
    if (res.ok) {
      const data = await res.json()
      heatmapData.value = data.data ?? []
    }
  } catch { /* ignore */ }
}

async function fetchRequests() {
  try {
    const res = await fetch(`/api/requests?page=${currentPage.value}&page_size=${pageSize.value}&endpoint=${encodeURIComponent(endpointName.value)}`)
    if (res.ok) {
      const data = await res.json()
      requestItems.value = data.items
      requestsTotal.value = data.total
    }
  } catch { /* ignore */ }
}

function renderTrendChart() {
  if (!trendCanvas.value || !Chart || !chartData.value) return
  trendChart?.destroy()
  const data = chartMode.value === 'hourly' ? chartData.value.hourly : chartData.value.daily
  const labels = data.map((d: any) => chartMode.value === 'hourly' ? d.hour.split(' ')[1] : d.day.slice(5))
  const counts = data.map((d: any) => d.count)

  trendChart = new Chart(trendCanvas.value, {
    type: 'line',
    data: {
      labels,
      datasets: [{ label: '调用次数', data: counts, borderColor: '#60a5fa', backgroundColor: 'rgba(96,165,250,0.1)', fill: true, tension: 0.3, pointRadius: 0 }],
    },
    options: {
      responsive: true, maintainAspectRatio: false, animation: false,
      plugins: { legend: { display: false } },
      scales: { y: { beginAtZero: true } },
    },
  })
}

function renderErrorChart() {
  if (!errorCanvas.value || !Chart) return
  errorChart?.destroy()
  const data = errorTrendData.value
  errorChart = new Chart(errorCanvas.value, {
    type: 'line',
    data: {
      labels: data.map((d: any) => d.time.replace(/^\d{4}-/, '').replace(' ', '\n')),
      datasets: [{ label: '错误率', data: data.map((d: any) => +(d.error_rate * 100).toFixed(1)), borderColor: '#ef4444', backgroundColor: 'rgba(239,68,68,0.1)', fill: true, tension: 0.3 }],
    },
    options: {
      responsive: true, maintainAspectRatio: false, animation: false,
      plugins: { legend: { display: false } },
      scales: { y: { beginAtZero: true, ticks: { callback: (v: any) => v + '%' } } },
    },
  })
}

async function loadAll() {
  await store.fetchUpstreamStatus()
  await Promise.all([fetchStats(), fetchLangStats(), fetchChartData(), fetchErrorTrend(), fetchHeatmap(), fetchRequests()])
}

watch(() => route.params.name, () => {
  currentPage.value = 1
  loadAll()
})

onMounted(async () => {
  const saved = localStorage.getItem(HEADER_RANGE_STORAGE_KEY)
  if (saved !== null) {
    const parsed = Number(saved)
    if ([0, 1, 7, 30, 90].includes(parsed)) headerRange.value = parsed
  }
  await loadAll()
})
</script>

<style scoped>
.endpoint-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 12px 16px;
}

.ep-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  font-size: 13px;
  font-weight: 500;
  color: #94a3b8;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(148, 163, 184, 0.12);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.15s ease;
}

.ep-btn:hover {
  color: #e2e8f0;
  background: rgba(255, 255, 255, 0.08);
}

.ep-btn.active {
  color: #f8fafc;
  background: rgba(96, 165, 250, 0.15);
  border-color: rgba(96, 165, 250, 0.3);
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}

.dot-ok { background: #0ecb81; }
.dot-error { background: #f6465d; }

.lang-table-simple {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.lang-table-simple th,
.lang-table-simple td {
  padding: 8px 12px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.08);
}

.lang-table-simple th {
  color: #94a3b8;
  font-weight: 500;
  text-align: left;
}

.heatmap-container {
  padding: 12px;
}

.heatmap-grid {
  min-height: 200px;
}

.heatmap-cell {
  min-height: 12px;
}
</style>
