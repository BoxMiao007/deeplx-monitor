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
        <button :class="['ep-btn', 'active']">全部对比</button>
        <button
          v-for="ep in store.upstreamStatus"
          :key="ep.name"
          :class="['ep-btn']"
          @click="$router.push(`/endpoints/${encodeURIComponent(ep.name)}`)"
        >
          <span :class="['dot', ep.healthy ? 'dot-ok' : 'dot-error']"></span>
          {{ ep.name }}
        </button>
      </section>

      <section class="card section-card">
        <h2 class="section-title">对比表格</h2>
        <div class="table-scroll">
          <table class="comparison-table">
            <thead>
              <tr>
                <th>指标</th>
                <th v-for="ep in store.upstreamStatus" :key="ep.name">{{ ep.name }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in comparisonRows" :key="row.label">
                <td class="metric-label">{{ row.label }}</td>
                <td v-for="(val, idx) in row.values" :key="idx" class="number">{{ val }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useMonitorStore } from '@/stores/monitor'
import TabNav from '@/components/TabNav.vue'

const store = useMonitorStore()
const headerRange = ref(0)
const endpointStats = ref<Record<string, { total_requests: number; total_chars: number; today_requests: number; today_chars: number; week_requests: number; week_chars: number; month_requests: number; month_chars: number }>>({})

const HEADER_RANGE_STORAGE_KEY = 'deeplx-monitor.header-range'

function onRangeChange() {
  localStorage.setItem(HEADER_RANGE_STORAGE_KEY, String(headerRange.value))
}

async function fetchEndpointStats() {
  await store.fetchUpstreamStatus()
  const results: typeof endpointStats.value = {}
  await Promise.all(
    store.upstreamStatus.map(async (ep) => {
      try {
        const res = await fetch(`/api/stats?endpoint=${encodeURIComponent(ep.name)}`)
        if (res.ok) {
          const data = await res.json()
          results[ep.name] = {
            total_requests: data.total_requests,
            total_chars: data.total_chars,
            today_requests: data.period.today_requests,
            today_chars: data.period.today_chars,
            week_requests: data.period.week_requests,
            week_chars: data.period.week_chars,
            month_requests: data.period.month_requests,
            month_chars: data.period.month_chars,
          }
        }
      } catch { /* ignore */ }
    })
  )
  endpointStats.value = results
}

const comparisonRows = computed(() => {
  const eps = store.upstreamStatus
  return [
    { label: '总调用', values: eps.map(ep => formatNumber(endpointStats.value[ep.name]?.total_requests ?? 0)) },
    { label: '总字符', values: eps.map(ep => formatNumber(endpointStats.value[ep.name]?.total_chars ?? 0)) },
    { label: '成功率', values: eps.map(ep => ep.total_requests > 0 ? ((ep.total_successes / ep.total_requests) * 100).toFixed(1) + '%' : '-') },
    { label: '平均延迟', values: eps.map(ep => ep.avg_latency_ms + 'ms') },
    { label: '健康状态', values: eps.map(ep => ep.healthy ? '健康' : '不健康') },
    { label: '今日调用', values: eps.map(ep => formatNumber(endpointStats.value[ep.name]?.today_requests ?? 0)) },
    { label: '今日字符', values: eps.map(ep => formatNumber(endpointStats.value[ep.name]?.today_chars ?? 0)) },
    { label: '近7天调用', values: eps.map(ep => formatNumber(endpointStats.value[ep.name]?.week_requests ?? 0)) },
    { label: '近7天字符', values: eps.map(ep => formatNumber(endpointStats.value[ep.name]?.week_chars ?? 0)) },
    { label: '近30天调用', values: eps.map(ep => formatNumber(endpointStats.value[ep.name]?.month_requests ?? 0)) },
    { label: '近30天字符', values: eps.map(ep => formatNumber(endpointStats.value[ep.name]?.month_chars ?? 0)) },
  ]
})

const _numFmt = new Intl.NumberFormat()
function formatNumber(n: number) { return _numFmt.format(n) }

onMounted(async () => {
  const saved = localStorage.getItem(HEADER_RANGE_STORAGE_KEY)
  if (saved !== null) {
    const parsed = Number(saved)
    if ([0, 1, 7, 30, 90].includes(parsed)) headerRange.value = parsed
  }
  await fetchEndpointStats()
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

.comparison-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.comparison-table th,
.comparison-table td {
  padding: 10px 14px;
  text-align: right;
  border-bottom: 1px solid rgba(148, 163, 184, 0.08);
}

.comparison-table th {
  color: #94a3b8;
  font-weight: 500;
  font-size: 12px;
}

.comparison-table td.metric-label {
  text-align: left;
  color: #cbd5e1;
  font-weight: 500;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}

.dot-ok { background: #0ecb81; }
.dot-error { background: #f6465d; }
</style>
