<template>
  <div class="dashboard-page">
    <div class="manual-refresh-toast-stack">
      <TransitionGroup name="fade">
        <div
          v-for="(toast, index) in manualRefreshToasts"
          :key="toast.id"
          class="manual-refresh-toast"
          :style="manualToastStyle(index)"
        >
          {{ toast.message }}
        </div>
      </TransitionGroup>
    </div>
    <div class="container dashboard-shell">
      <div v-if="store.error" class="error-banner">
        <span class="error-text">{{ store.error }}</span>
        <button class="error-dismiss" @click="store.error = null">×</button>
      </div>

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
            <select v-model="headerRange" @change="onHeaderRangeChange" class="input control-select">
              <option :value="1">今天</option>
              <option :value="7">7 天</option>
              <option :value="30">30 天</option>
              <option :value="90">90 天</option>
              <option :value="0">所有</option>
            </select>
          </label>
          <label class="control-field">
            <span class="control-label">自动刷新</span>
            <div class="refresh-control-group">
              <select v-model="headerRefresh" @change="onHeaderRefreshChange" class="input control-select">
                <option :value="0">关闭</option>
                <option :value="5">5 秒</option>
                <option :value="10">10 秒</option>
                <option :value="30">30 秒</option>
                <option :value="60">60 秒</option>
              </select>
              <button :class="['refresh-icon-btn', { spinning: refreshAnimating }]" type="button" @click="onManualRefresh" title="手动刷新">
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M20 12a8 8 0 1 1-2.34-5.66M20 4v6h-6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </button>
            </div>
          </label>
          <button class="btn-primary btn-sm" @click="openSettings">设置</button>
        </div>
      </section>

      <section class="summary-grid">
        <article class="stat-card card emphasis-card">
          <div class="stat-head">
            <span class="stat-label">总调用次数</span>
            <span class="stat-trend">累计总数</span>
          </div>
          <div ref="statRequestsEl" class="stat-value number clickable" @click="toggleStatDisplay" :title="showFullStat ? '点击切换缩写' : '点击显示完整数值'">{{ showFullStat ? formatNumber(stats?.total_requests ?? 0) : formatCompactNumber(stats?.total_requests ?? 0) }}</div>
          <div class="mini-chart-caption">{{ miniChartCaption }}</div>
          <div class="mini-chart-wrap"><canvas ref="countMiniChartCanvas"></canvas></div>
        </article>

        <article class="stat-card card emphasis-card">
          <div class="stat-head">
            <span class="stat-label">总字符数</span>
            <span class="stat-trend">累计总数</span>
          </div>
          <div ref="statCharsEl" class="stat-value number clickable" @click="toggleStatDisplay" :title="showFullStat ? '点击切换缩写' : '点击显示完整数值'">{{ showFullStat ? formatNumber(stats?.total_chars ?? 0) : formatCompactNumber(stats?.total_chars ?? 0) }}</div>
          <div class="mini-chart-caption">{{ miniChartCaption }}</div>
          <div class="mini-chart-wrap"><canvas ref="charsMiniChartCanvas"></canvas></div>
        </article>

        <article class="stat-card card emphasis-card overview-card">
          <div class="stat-head">
            <span class="stat-label">服务概览</span>
            <span class="stat-trend">运行状态</span>
          </div>
          <div class="overview-metrics">
            <div class="overview-metric-item">
              <span class="overview-metric-label">成功率</span>
              <span :class="['overview-metric-value', 'number', successRateColor(Number(overviewSuccessRate))]">{{ overviewSuccessRate }}<span class="overview-metric-unit">%</span></span>
            </div>
            <div class="overview-metric-item">
              <span class="overview-metric-label">平均延迟</span>
              <span :class="['overview-metric-value', 'number', latencyColor(Number(overviewAvgLatency))]">{{ overviewAvgLatency }}<span class="overview-metric-unit">ms</span></span>
            </div>
            <div class="overview-metric-item">
              <span class="overview-metric-label">健康端点</span>
              <span class="overview-metric-value number">{{ overviewHealthyEndpoints }}</span>
            </div>
          </div>
        </article>
      </section>

      <section class="period-grid">
        <article v-for="card in periodCards" :key="card.label" class="period-card card">
          <div class="period-label">{{ card.label }}</div>
          <div class="period-row">
            <span class="period-key">调用</span>
            <span class="period-value number">{{ formatNumber(card.requests) }}</span>
          </div>
          <div class="period-row">
            <span class="period-key">字符</span>
            <span class="period-value number">{{ formatNumber(card.chars) }}</span>
          </div>
        </article>
      </section>

      <section class="infra-grid">
        <article v-if="store.upstreamStatus.length > 0" class="card section-card">
          <div class="section-topbar">
            <div>
              <h2 class="section-title">上游端点</h2>
              <p class="section-subtitle">负载均衡状态与各端点健康情况</p>
            </div>
            <button class="btn-secondary btn-sm" @click="onHealthCheck" :disabled="checking">
              {{ checking ? '检测中...' : '健康检查' }}
            </button>
          </div>
          <div class="chart-tabs upstream-tabs-row">
            <button
              :class="['chart-tab', { active: activeUpstream === '__all__' }]"
              @click="activeUpstream = '__all__'"
            >全部</button>
            <button
              v-for="ep in store.upstreamStatus"
              :key="ep.name"
              :class="['chart-tab', { active: activeUpstream === ep.name }]"
              @click="activeUpstream = ep.name"
            >
              <span :class="['dot', ep.healthy ? 'dot-ok' : 'dot-error']"></span>
              {{ ep.name }}
            </button>
          </div>
          <div class="upstream-panel" v-if="activeUpstream === '__all__'">
            <div class="upstream-grid">
              <div class="upstream-grid-item">
                <span class="upstream-grid-label">端点数</span>
                <span class="upstream-grid-value number">{{ store.upstreamStatus.length }}</span>
              </div>
              <div class="upstream-grid-item">
                <span class="upstream-grid-label">健康</span>
                <span class="upstream-grid-value number">{{ store.upstreamStatus.filter(e => e.healthy).length }}</span>
              </div>
              <div class="upstream-grid-item">
                <span class="upstream-grid-label">不健康</span>
                <span class="upstream-grid-value number">{{ store.upstreamStatus.filter(e => !e.healthy).length }}</span>
              </div>
              <div class="upstream-grid-item">
                <span class="upstream-grid-label">总请求</span>
                <span class="upstream-grid-value number">{{ store.upstreamStatus.reduce((s, e) => s + e.total_requests, 0) }}</span>
              </div>
              <div class="upstream-grid-item">
                <span class="upstream-grid-label">总成功率</span>
                <span :class="['upstream-grid-value', 'number', successRateColor(Number(upstreamOverallSuccessRate))]">{{ upstreamOverallSuccessRate }}%</span>
              </div>
            </div>
            <div class="health-check-result">
              <span class="health-check-result-label">上次健康检查</span>
              <template v-if="store.healthStatus.checked_at">
                <span class="health-check-result-time">{{ formatCheckedAt(store.healthStatus.checked_at) }}</span>
                <span :class="['badge', 'badge-sm', store.healthStatus.status === 'ok' ? 'badge-success' : 'badge-error']">{{ store.healthStatus.status === 'ok' ? '正常' : '异常' }}</span>
                <span v-if="store.healthStatus.status === 'ok'" :class="['health-check-result-latency', 'number', latencyColor(store.healthStatus.latency_ms ?? 0)]">{{ store.healthStatus.latency_ms ?? '-' }}ms</span>
              </template>
              <span v-else class="health-check-result-time">暂无记录</span>
            </div>
          </div>
          <div class="upstream-panel" v-else-if="activeUpstreamData">
            <div class="upstream-grid">
              <div class="upstream-grid-item">
                <span class="upstream-grid-label">状态</span>
                <span :class="['badge', 'badge-with-tooltip', activeUpstreamData.healthy ? 'badge-success' : 'badge-error']" :data-tooltip="activeUpstreamData.last_error || '正常'">{{ activeUpstreamData.healthy ? '健康' : '不健康' }}</span>
              </div>
              <div class="upstream-grid-item">
                <span class="upstream-grid-label">平均延迟</span>
                <span :class="['upstream-grid-value', 'number', latencyColor(activeUpstreamData.avg_latency_ms)]">{{ activeUpstreamData.avg_latency_ms }}<span class="upstream-grid-unit">ms</span></span>
              </div>
              <div class="upstream-grid-item">
                <span class="upstream-grid-label">成功率</span>
                <span :class="['upstream-grid-value', 'number', successRateColor(activeUpstreamData.total_requests > 0 ? (activeUpstreamData.total_successes / activeUpstreamData.total_requests) * 100 : 0)]">{{ activeUpstreamData.total_requests > 0 ? ((activeUpstreamData.total_successes / activeUpstreamData.total_requests) * 100).toFixed(1) : '0' }}<span class="upstream-grid-unit">%</span></span>
              </div>
              <div class="upstream-grid-item">
                <span class="upstream-grid-label">总请求</span>
                <span class="upstream-grid-value number">{{ activeUpstreamData.total_requests }}</span>
              </div>
              <div class="upstream-grid-item">
                <span class="upstream-grid-label">连续失败</span>
                <span class="upstream-grid-value number">{{ activeUpstreamData.consecutive_failures }}</span>
              </div>
            </div>
            <div class="upstream-url-row" @click="upstreamUrlRevealed = !upstreamUrlRevealed">
              {{ upstreamUrlRevealed ? activeUpstreamData.url : maskUrl(activeUpstreamData.url) }}
            </div>
          </div>
        </article>

        <article class="card section-card clip-card">
          <div class="section-topbar">
            <div>
              <h2 class="section-title">翻译缓存 <span class="mode-badge">{{ (store.cacheStats?.max_memory_mb ?? 0) > 0 ? '内存模式' : '条目模式' }}</span></h2>
              <p class="section-subtitle">缓存命中率与容量使用情况</p>
            </div>
            <div class="chart-tabs">
              <button :class="['chart-tab', { active: cacheViewMode === 'today' }]" @click="cacheViewMode = 'today'">今天</button>
              <button :class="['chart-tab', { active: cacheViewMode === 'total' }]" @click="cacheViewMode = 'total'">总计</button>
            </div>
          </div>
          <div class="overview-metrics" v-if="store.cacheStats?.enabled && cacheViewMode === 'total'">
            <div class="overview-metric-item">
              <span class="overview-metric-label">命中率</span>
              <span :class="['overview-metric-value', 'number', successRateColor(store.cacheStats.hit_rate * 100)]">{{ (store.cacheStats.hit_rate * 100).toFixed(1) }}<span class="overview-metric-unit">%</span></span>
            </div>
            <div class="overview-metric-item">
              <span class="overview-metric-label">命中</span>
              <span class="overview-metric-value number">{{ formatCompactNumber(store.cacheStats.hits) }}</span>
            </div>
            <div class="overview-metric-item">
              <span class="overview-metric-label">未命中</span>
              <span class="overview-metric-value number">{{ formatCompactNumber(store.cacheStats.misses) }}</span>
            </div>
          </div>
          <div class="cache-stats-grid" v-if="store.cacheStats?.enabled && cacheViewMode === 'today'">
            <div class="cache-stat">
              <span class="cache-stat-label">命中率</span>
              <span :class="['cache-stat-value', 'number', successRateColor(store.cacheStats.today_hit_rate * 100)]">{{ (store.cacheStats.today_hit_rate * 100).toFixed(1) }}%</span>
            </div>
            <div class="cache-stat">
              <span class="cache-stat-label">命中</span>
              <span class="cache-stat-value number">{{ formatNumber(store.cacheStats.today_hits) }}</span>
            </div>
            <div class="cache-stat">
              <span class="cache-stat-label">未命中</span>
              <span class="cache-stat-value number">{{ formatNumber(store.cacheStats.today_misses) }}</span>
            </div>
            <div class="cache-stat">
              <span class="cache-stat-label">缓存条目</span>
              <span class="cache-stat-value number">{{ store.cacheStats.max_memory_mb > 0 ? formatNumber(store.cacheStats.size) : `${formatNumber(store.cacheStats.size)} / ${formatNumber(store.cacheStats.max_entries)}` }}</span>
            </div>
            <div class="cache-stat">
              <span class="cache-stat-label">内存占用</span>
              <span class="cache-stat-value number">{{ formatBytes(store.cacheStats.estimated_memory_bytes) }}<template v-if="store.cacheStats.max_memory_mb > 0"> / {{ store.cacheStats.max_memory_mb }} MB</template></span>
            </div>
            <div class="cache-stat">
              <span class="cache-stat-label">缓存时长</span>
              <span class="cache-stat-value number">{{ formatTtl(store.cacheStats.ttl_secs) }}</span>
            </div>
          </div>
          <div v-if="!store.cacheStats?.enabled" class="text-muted" style="padding: 1rem;">缓存未启用</div>
        </article>
      </section>

      <section class="dashboard-grid">
        <article class="card section-card lang-card">
          <div class="section-topbar">
            <div>
              <h2 class="section-title">语言统计</h2>
              <p class="section-subtitle">按字符数聚合，便于观察常用语言方向</p>
            </div>
            <div class="section-actions lang-section-actions">
              <button class="btn-secondary btn-sm" @click="showLangMetricDetail = !showLangMetricDetail">
                {{ showLangMetricDetail ? '收起详细' : '显示详细' }}
              </button>
              <label class="control-field lang-range-field">
                <span class="control-label">范围</span>
                <select v-model="langRange" @change="onLangRangeChange" class="input control-select">
                  <option value="all">所有</option>
                  <option value="today">今天</option>
                  <option value="week">7 天</option>
                  <option value="month">30 天</option>
                </select>
              </label>
            </div>
          </div>
          <div class="lang-list-shell">
            <table class="lang-table">
              <thead>
                <tr>
                  <th>语言</th>
                  <th class="sortable-th" @click="toggleLangSort('source_chars')">
                    源语言字符数<span class="sort-indicator">{{ sortIndicator('source_chars') }}</span>
                  </th>
                  <th class="sortable-th" @click="toggleLangSort('target_chars')">
                    目标语言字符数<span class="sort-indicator">{{ sortIndicator('target_chars') }}</span>
                  </th>
                  <th class="sortable-th" @click="toggleLangSort('total_chars')">
                    总字符数<span class="sort-indicator">{{ sortIndicator('total_chars') }}</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr v-if="!langPairStats.length">
                  <td colspan="4" class="text-center text-muted">暂无数据</td>
                </tr>
                <tr v-for="stat in langPairStats" :key="stat.lang">
                  <td class="lang-entry-cell">
                    <div class="lang-entry-card">
                      <div class="lang-cell">
                        <img v-if="langFlagUrl(stat.lang)" :src="langFlagUrl(stat.lang)" width="20" height="14" class="flag-img">
                        <span>{{ langFullName(stat.lang) }}</span>
                      </div>
                    </div>
                  </td>
                  <td class="number lang-entry-cell">
                    <div class="lang-bar-cell">
                      <div class="lang-bar-track">
                        <div class="lang-bar-fill" :style="barStyle(stat.source_chars, stat.lang, 'source')"></div>
                        <span class="lang-bar-label" :style="barLabelStyle(stat.source_chars)">{{ showLangMetricDetail ? formatNumber(stat.source_chars) : formatCompactNumber(stat.source_chars) }}</span>
                      </div>
                    </div>
                  </td>
                  <td class="number lang-entry-cell">
                    <div class="lang-bar-cell">
                      <div class="lang-bar-track">
                        <div class="lang-bar-fill" :style="barStyle(stat.target_chars, stat.lang, 'target')"></div>
                        <span class="lang-bar-label" :style="barLabelStyle(stat.target_chars)">{{ showLangMetricDetail ? formatNumber(stat.target_chars) : formatCompactNumber(stat.target_chars) }}</span>
                      </div>
                    </div>
                  </td>
                  <td class="number lang-entry-cell">
                    <div class="lang-bar-cell">
                      <div class="lang-bar-track">
                        <div class="lang-bar-fill" :style="barStyle(stat.total_chars, stat.lang, 'total')"></div>
                        <span class="lang-bar-label" :style="barLabelStyle(stat.total_chars)">{{ showLangMetricDetail ? formatNumber(stat.total_chars) : formatCompactNumber(stat.total_chars) }}</span>
                      </div>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </article>

        <article class="card section-card trend-card">
          <div class="section-topbar">
            <div>
              <h2 class="section-title">调用趋势</h2>
              <p class="section-subtitle">支持查看 24 小时与近 30 天走势</p>
            </div>
            <div class="chart-tabs">
              <button
                v-for="tab in chartTabs"
                :key="tab.value"
                :class="['chart-tab', { active: activeChartTab === tab.value }]"
                @click="activeChartTab = tab.value"
              >{{ tab.label }}</button>
            </div>
          </div>
          <div class="charts-grid">
            <div class="chart-wrap panel-surface lang-usage-wrap">
              <div class="chart-label">语言使用趋势</div>
              <canvas ref="langUsageTrendCanvas"></canvas>
            </div>
            <div class="chart-wrap panel-surface">
              <div class="chart-label">调用次数</div>
              <canvas ref="requestsChartCanvas"></canvas>
            </div>
            <div class="chart-wrap panel-surface">
              <div class="chart-label">字符数</div>
              <canvas ref="charsChartCanvas"></canvas>
            </div>
          </div>
        </article>
      </section>

      <section class="analytics-grid">
        <article class="card section-card">
          <div class="section-topbar">
            <div>
              <h2 class="section-title">错误率趋势</h2>
              <p class="section-subtitle">按时间段展示错误率变化</p>
            </div>
            <div class="chart-tabs">
              <button :class="['chart-tab', { active: errorTrendDays === 1 }]" @click="onErrorTrendChange(1)">24h</button>
              <button :class="['chart-tab', { active: errorTrendDays === 7 }]" @click="onErrorTrendChange(7)">7天</button>
              <button :class="['chart-tab', { active: errorTrendDays === 30 }]" @click="onErrorTrendChange(30)">30天</button>
            </div>
          </div>
          <div class="chart-wrap panel-surface">
            <canvas ref="errorTrendCanvas"></canvas>
          </div>
        </article>

        <article class="card section-card">
          <div class="section-topbar">
            <div>
              <h2 class="section-title">活动热力图</h2>
              <p class="section-subtitle">按时段展示请求活跃度分布</p>
            </div>
            <div class="chart-tabs">
              <button :class="['chart-tab', { active: heatmapView === 'weekday' }]" @click="onHeatmapViewChange('weekday')">星期</button>
              <button :class="['chart-tab', { active: heatmapView === 'date' }]" @click="onHeatmapViewChange('date')">日期</button>
            </div>
          </div>
          <div class="heatmap-container">
            <div v-if="!store.heatmapData?.data?.length" class="text-center text-muted" style="padding: 2rem;">暂无数据</div>
            <div v-else class="heatmap-grid" :style="heatmapGridStyle">
              <div
                v-for="(cell, idx) in store.heatmapData.data"
                :key="idx"
                class="heatmap-cell"
                :style="heatmapCellStyle(cell.count)"
                :data-tooltip="`${cell.x} ${cell.y}:00 — ${cell.count} 次请求`"
              ></div>
            </div>
            <div v-if="store.heatmapData?.data?.length" class="heatmap-legend">
              <span class="text-muted">少</span>
              <div class="heatmap-legend-bar"></div>
              <span class="text-muted">多</span>
            </div>
          </div>
        </article>
      </section>

      <section class="card section-card request-card">
        <div class="section-topbar request-topbar">
          <div>
            <div class="log-tabs">
              <button :class="['log-tab', { active: logTab === 'requests' }]" @click="logTab = 'requests'">请求日志</button>
              <button :class="['log-tab', { active: logTab === 'cache-hits' }]" @click="switchToCacheHits">缓存命中</button>
            </div>
            <p class="section-subtitle" v-if="logTab === 'requests'">检查失败请求的错误信息展示</p>
            <p class="section-subtitle" v-else>最近 100 条缓存命中记录（仅保留在内存中）</p>
          </div>
          <div class="request-toolbar" v-if="logTab === 'requests'">
            <label class="page-size-select">
              <span>每页</span>
              <select v-model="pageSize" @change="onPageSizeChange" class="input page-size-input">
                <option :value="50">50</option>
                <option :value="100">100</option>
                <option :value="200">200</option>
              </select>
              <span>条</span>
            </label>
            <div class="export-btns">
              <button class="btn-secondary btn-sm" @click="store.exportLogs('csv')">导出 CSV</button>
              <button class="btn-secondary btn-sm" @click="store.exportLogs('json')">导出 JSON</button>
            </div>
          </div>
        </div>

        <template v-if="logTab === 'requests'">
          <div class="request-summary">
            <span class="table-count">共 {{ store.requests?.total ?? 0 }} 条</span>
            <span class="table-count text-muted">当前第 {{ currentPage }} / {{ totalPages }} 页</span>
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
                <tr v-if="!items.length">
                  <td colspan="7" class="text-center text-muted">暂无数据</td>
                </tr>
                <tr v-for="item in items" :key="item.id">
                  <td class="number time-cell">{{ formatTime(item.created_at) }}</td>
                  <td>
                    <div class="lang-cell">
                      <img v-if="langFlagUrl(item.source_lang)" :src="langFlagUrl(item.source_lang)" width="20" height="14" class="flag-img">
                      <span>{{ langFullName(item.source_lang) }}</span>
                    </div>
                  </td>
                  <td class="number">{{ formatNumber(item.source_chars) }}</td>
                  <td>
                    <div class="lang-cell">
                      <img v-if="langFlagUrl(item.target_lang)" :src="langFlagUrl(item.target_lang)" width="20" height="14" class="flag-img">
                      <span>{{ langFullName(item.target_lang) }}</span>
                    </div>
                  </td>
                  <td class="number">{{ formatNumber(item.target_chars) }}</td>
                  <td>
                    <span :class="['badge', 'status-badge', item.status === 'success' ? 'badge-success' : 'badge-error']">
                      {{ item.status === 'success' ? '成功' : '失败' }}
                    </span>
                  </td>
                  <td><span class="error-message" :title="item.error_msg || '-'">{{ item.error_msg || '-' }}</span></td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="pagination-bar">
            <button class="btn-secondary btn-sm" :disabled="currentPage <= 1" @click="changePage(1)">首页</button>
            <button class="btn-secondary btn-sm" :disabled="currentPage <= 1" @click="changePage(currentPage - 1)">上一页</button>
            <div class="page-numbers compact-scroll">
              <button
                v-for="p in pageNumbers"
                :key="p"
                :class="['btn-page', { active: p === currentPage }]"
                @click="changePage(p)"
              >{{ p }}</button>
            </div>
            <button class="btn-secondary btn-sm" :disabled="currentPage >= totalPages" @click="changePage(currentPage + 1)">下一页</button>
            <button class="btn-secondary btn-sm" :disabled="currentPage >= totalPages" @click="changePage(totalPages)">末页</button>
          </div>
        </template>

        <template v-else>
          <div class="request-summary">
            <span class="table-count">共 {{ store.cacheHitLogs.length }} 条</span>
          </div>

          <div class="table-scroll">
            <table class="request-table">
              <thead>
                <tr>
                  <th>时间</th>
                  <th>源语言</th>
                  <th>目标语言</th>
                  <th>文本预览</th>
                </tr>
              </thead>
              <tbody>
                <tr v-if="!store.cacheHitLogs.length">
                  <td colspan="4" class="text-center text-muted">暂无缓存命中记录</td>
                </tr>
                <tr v-for="(hit, idx) in store.cacheHitLogs" :key="idx">
                  <td class="number time-cell">{{ formatTime(hit.timestamp) }}</td>
                  <td>
                    <div class="lang-cell">
                      <img v-if="langFlagUrl(hit.source_lang)" :src="langFlagUrl(hit.source_lang)" width="20" height="14" class="flag-img">
                      <span>{{ langFullName(hit.source_lang) }}</span>
                    </div>
                  </td>
                  <td>
                    <div class="lang-cell">
                      <img v-if="langFlagUrl(hit.target_lang)" :src="langFlagUrl(hit.target_lang)" width="20" height="14" class="flag-img">
                      <span>{{ langFullName(hit.target_lang) }}</span>
                    </div>
                  </td>
                  <td><span class="text-preview" :title="hit.text_preview">{{ hit.text_preview }}</span></td>
                </tr>
              </tbody>
            </table>
          </div>
        </template>
      </section>
    </div>

    <Transition name="fade">
      <div v-if="showSettings" class="settings-overlay" @click="closeSettings"></div>
    </Transition>

    <Transition name="drawer">
      <aside v-if="showSettings" class="settings-drawer" aria-label="设置抽屉">
        <div class="drawer-header">
          <div>
            <h3>设置 <span class="version-tag" v-if="store.appVersion">v{{ store.appVersion }}</span></h3>
            <p class="drawer-subtitle">管理所有配置项，保存后立即生效</p>
          </div>
          <a href="https://github.com/BoxMiao007/deeplx-monitor" target="_blank" rel="noopener" class="btn-github" title="GitHub">
            <svg viewBox="0 0 16 16" width="22" height="22" fill="currentColor">
              <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>
            </svg>
          </a>
        </div>
        <div class="drawer-body">
          <!-- 上游配置 -->
          <div class="settings-section">
            <h4 class="settings-section-title">上游配置</h4>
            <div class="form-group">
              <label class="form-label">端点列表</label>
              <div class="endpoint-list">
                <div v-for="(ep, idx) in settingsFullForm.upstream.endpoints" :key="idx" class="endpoint-item">
                  <div class="endpoint-name-row">
                    <input v-model="ep.name" type="text" class="input endpoint-input endpoint-name" placeholder="名称">
                    <button class="endpoint-remove-btn" @click="removeEndpoint(idx)" title="移除">×</button>
                    <span class="endpoint-test-group">
                      <span :class="['endpoint-latency', endpointTestResults[idx] ? latencyColor(endpointTestResults[idx].latency) : '']">{{ endpointTestResults[idx] ? (endpointTestResults[idx].ok ? endpointTestResults[idx].latency + 'ms' : '失败') : '-ms' }}</span>
                      <button class="btn-secondary btn-sm endpoint-test-btn" @click="testEndpoint(idx)" :disabled="endpointTesting[idx]">{{ endpointTesting[idx] ? '...' : '测试' }}</button>
                    </span>
                  </div>
                  <input v-model="ep.url" type="text" class="input endpoint-input mono-input" placeholder="URL">
                  <input v-model="ep.api_key" type="password" class="input endpoint-input mono-input" placeholder="API Key（可选）">
                </div>
              </div>
              <button class="btn-secondary btn-sm add-endpoint-btn" @click="addEndpoint">+ 添加端点</button>
            </div>
            <div class="form-row">
              <div class="form-group compact">
                <label class="form-label">最大失败次数</label>
                <input v-model.number="settingsFullForm.upstream.max_failures" type="number" class="input" min="1">
              </div>
              <div class="form-group compact">
                <label class="form-label">探活间隔 (秒)</label>
                <input v-model.number="settingsFullForm.upstream.probe_interval_secs" type="number" class="input" min="5">
              </div>
            </div>
          </div>

          <!-- 代理配置 -->
          <div class="settings-section">
            <h4 class="settings-section-title">
              代理配置
              <span class="settings-badge-warn">需重启生效</span>
            </h4>
            <div class="form-row">
              <div class="form-group compact">
                <label class="form-label">监听地址</label>
                <input v-model="settingsFullForm.proxy.host" type="text" class="input mono-input" placeholder="127.0.0.1">
              </div>
              <div class="form-group compact">
                <label class="form-label">端口</label>
                <input v-model.number="settingsFullForm.proxy.port" type="number" class="input" min="1" max="65535">
              </div>
            </div>
          </div>

          <!-- 监控配置 -->
          <div class="settings-section">
            <h4 class="settings-section-title">监控配置</h4>
            <div class="form-row">
              <div class="form-group compact">
                <label class="form-label">自动刷新间隔 (秒)</label>
                <input v-model.number="settingsFullForm.monitor.auto_refresh_seconds" type="number" class="input" min="0">
              </div>
              <div class="form-group compact">
                <label class="form-label">最大日志条数</label>
                <input v-model.number="settingsFullForm.monitor.max_log_entries" type="number" class="input" min="1">
              </div>
            </div>
          </div>

          <!-- 健康检查 -->
          <div class="settings-section">
            <h4 class="settings-section-title">健康检查</h4>
            <div class="form-row">
              <div class="form-group compact">
                <label class="form-label">源语言</label>
                <input v-model="settingsFullForm.health_check.source_lang" type="text" class="input" placeholder="EN">
              </div>
              <div class="form-group compact">
                <label class="form-label">目标语言</label>
                <input v-model="settingsFullForm.health_check.target_lang" type="text" class="input" placeholder="ZH">
              </div>
            </div>
          </div>

          <!-- 缓存配置 -->
          <div class="settings-section">
            <h4 class="settings-section-title">缓存配置</h4>
            <div class="form-group compact">
              <label class="form-label toggle-label">
                <input v-model="settingsFullForm.cache.enabled" type="checkbox" class="toggle-input">
                <span class="toggle-switch"></span>
                <span>启用缓存</span>
              </label>
            </div>
            <div class="form-row">
              <div class="form-group compact">
                <label class="form-label">缓存时长</label>
                <div class="input-with-unit">
                  <input v-model.number="cacheTtlDisplay" type="number" class="input" min="1">
                  <select v-model="cacheTtlUnit" class="unit-select">
                    <option value="s">秒</option>
                    <option value="m">分</option>
                    <option value="h">时</option>
                  </select>
                </div>
              </div>
              <div :class="['form-group', 'compact', { 'field-disabled': settingsFullForm.cache.max_memory_mb > 0 }]">
                <label class="form-label">最大条目数</label>
                <input v-model.number="settingsFullForm.cache.max_entries" type="number" class="input" min="1" :disabled="settingsFullForm.cache.max_memory_mb > 0">
              </div>
            </div>
            <div class="form-row">
              <div class="form-group compact">
                <label class="form-label">最大内存 (MB)</label>
                <input v-model.number="settingsFullForm.cache.max_memory_mb" type="number" class="input" min="0">
                <span class="form-hint">0 表示不限制内存，使用条目数限制</span>
              </div>
            </div>
            <div class="form-group compact">
              <button class="btn-danger-sm" @click="handleClearCache" :disabled="clearingCache">{{ clearingCache ? '清除中...' : '清除缓存' }}</button>
              <span v-if="clearCacheMsg" class="cache-clear-msg">{{ clearCacheMsg }}</span>
            </div>
          </div>

          <!-- 演示模式 -->
          <div class="settings-section">
            <h4 class="settings-section-title">演示模式</h4>
            <div class="form-group compact">
              <label class="form-label toggle-label">
                <input v-model="settingsFullForm.demo.enabled" type="checkbox" class="toggle-input">
                <span class="toggle-switch"></span>
                <span>启用演示模式</span>
              </label>
            </div>
            <div class="form-group compact">
              <label class="form-label">种子</label>
              <input v-model.number="settingsFullForm.demo.seed" type="number" class="input" min="0">
            </div>
          </div>

        </div>
        <div class="drawer-actions">
          <button :class="['btn-primary', { 'btn-saved': saveSuccess && saveMsg, 'btn-dirty': settingsDirty && !saveMsg }]" @click="saveSettings" :disabled="saving">{{ saving ? '保存中...' : (saveSuccess && saveMsg ? '设置已保存！' : '保存设置') }}</button>
        </div>
      </aside>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { useMonitorStore } from '@/stores/monitor'
import type { FullConfig } from '@/stores/monitor'

import type { Chart as ChartType } from 'chart.js'

let Chart: typeof ChartType | null = null

async function ensureChartJs() {
  if (Chart) return
  const mod = await import('chart.js')
  Chart = mod.Chart
  Chart.register(
    mod.LineController,
    mod.BarController,
    mod.LineElement,
    mod.BarElement,
    mod.PointElement,
    mod.LinearScale,
    mod.CategoryScale,
    mod.Filler,
    mod.Tooltip,
    mod.Legend,
  )
}

const store = useMonitorStore()
const checking = ref(false)
const logTab = ref<'requests' | 'cache-hits'>('requests')
const cacheViewMode = ref<'today' | 'total'>('today')
const showFullStat = ref(false)
const currentPage = ref(1)
const pageSize = ref(50)
const activeChartTab = ref('hourly')
const langRange = ref('all')
const langSortKey = ref<'source_chars' | 'target_chars' | 'total_chars'>('total_chars')
const langSortOrder = ref<'asc' | 'desc'>('desc')
const showLangMetricDetail = ref(false)
const showSettings = ref(false)
const headerRefresh = ref(0)
const headerRange = ref(0)
const refreshing = ref(false)
const refreshAnimating = ref(false)
const manualRefreshToasts = ref<Array<{ id: number; message: string }>>([])
const saving = ref(false)
const saveMsg = ref('')
const saveSuccess = ref(false)
const clearingCache = ref(false)
const clearCacheMsg = ref('')
const settingsSnapshot = ref('')
const settingsDirty = computed(() => {
  if (!settingsSnapshot.value) return false
  return JSON.stringify(settingsFullForm) !== settingsSnapshot.value
})
const settingsFullForm = reactive<FullConfig>({
  upstream: { endpoints: [], max_failures: 3, probe_interval_secs: 60 },
  proxy: { host: '127.0.0.1', port: 5555 },
  monitor: { auto_refresh_seconds: 0, max_log_entries: 10000 },
  health_check: { source_lang: 'EN', target_lang: 'ZH' },
  cache: { enabled: false, ttl_secs: 3600, max_entries: 10000, max_memory_mb: 0 },
  demo: { enabled: false, seed: 20260511 },
})
const cacheTtlUnit = ref<'s' | 'm' | 'h'>('s')
const cacheTtlDisplay = computed({
  get: () => {
    const secs = settingsFullForm.cache.ttl_secs
    if (cacheTtlUnit.value === 'h') return secs / 3600
    if (cacheTtlUnit.value === 'm') return secs / 60
    return secs
  },
  set: (val: number) => {
    if (cacheTtlUnit.value === 'h') settingsFullForm.cache.ttl_secs = val * 3600
    else if (cacheTtlUnit.value === 'm') settingsFullForm.cache.ttl_secs = val * 60
    else settingsFullForm.cache.ttl_secs = val
  },
})
const HEADER_RANGE_STORAGE_KEY = 'deeplx-monitor.header-range'
let manualRefreshToastSeq = 0

const requestsChartCanvas = ref<HTMLCanvasElement | null>(null)
const charsChartCanvas = ref<HTMLCanvasElement | null>(null)
const langUsageTrendCanvas = ref<HTMLCanvasElement | null>(null)
const countMiniChartCanvas = ref<HTMLCanvasElement | null>(null)
const charsMiniChartCanvas = ref<HTMLCanvasElement | null>(null)
const statRequestsEl = ref<HTMLElement | null>(null)
const statCharsEl = ref<HTMLElement | null>(null)
const errorTrendCanvas = ref<HTMLCanvasElement | null>(null)
const errorTrendDays = ref(7)
const heatmapView = ref('weekday')
const activeUpstream = ref('__all__')
const upstreamUrlRevealed = ref(false)

const activeUpstreamData = computed(() => {
  return store.upstreamStatus.find(ep => ep.name === activeUpstream.value) || null
})

const upstreamOverallSuccessRate = computed(() => {
  const total = store.upstreamStatus.reduce((s, e) => s + e.total_requests, 0)
  const succ = store.upstreamStatus.reduce((s, e) => s + e.total_successes, 0)
  return total > 0 ? ((succ / total) * 100).toFixed(1) : '0'
})

const overviewSuccessRate = computed(() => {
  const total = store.upstreamStatus.reduce((s, e) => s + e.total_requests, 0)
  const succ = store.upstreamStatus.reduce((s, e) => s + e.total_successes, 0)
  return total > 0 ? ((succ / total) * 100).toFixed(1) : '0.0'
})

const overviewAvgLatency = computed(() => {
  const endpoints = store.upstreamStatus.filter(e => e.avg_latency_ms > 0)
  if (!endpoints.length) return '-'
  const avg = endpoints.reduce((s, e) => s + e.avg_latency_ms, 0) / endpoints.length
  return Math.round(avg)
})

const overviewHealthyEndpoints = computed(() => {
  const healthy = store.upstreamStatus.filter(e => e.healthy).length
  const total = store.upstreamStatus.length
  return total > 0 ? `${healthy}/${total}` : '0/0'
})

function maskUrl(url: string): string {
  try {
    const u = new URL(url)
    const path = u.pathname
    if (path.length > 10) {
      const masked = path.slice(0, 4) + '****' + path.slice(-4)
      return u.origin + masked
    }
    return u.origin + '/****'
  } catch {
    if (url.length > 20) {
      return url.slice(0, 10) + '****' + url.slice(-6)
    }
    return '****'
  }
}

let requestsChart: ChartType | null = null
let charsChart: ChartType | null = null
let langUsageTrendChart: ChartType | null = null
let errorTrendChart: ChartType | null = null
let countMiniChart: ChartType | null = null
let charsMiniChart: ChartType | null = null

const chartTabs = [
  { label: '24 小时', value: 'hourly' },
  { label: '30 天', value: 'daily' },
]


const stats = computed(() => store.stats)
const period = computed(() => store.period)
const items = computed(() => store.requests?.items ?? [])
const totalPages = computed(() => Math.max(1, Math.ceil((store.requests?.total ?? 0) / pageSize.value)))
const pageNumbers = computed(() => {
  const maxVisible = 7
  const total = totalPages.value
  const start = Math.max(1, Math.min(currentPage.value - 3, Math.max(1, total - maxVisible + 1)))
  const end = Math.min(total, start + maxVisible - 1)
  return Array.from({ length: end - start + 1 }, (_, idx) => start + idx)
})

const periodCards = computed(() => [
  { label: '今日', requests: period.value.today_requests, chars: period.value.today_chars },
  { label: '近 7 天', requests: period.value.week_requests, chars: period.value.week_chars },
  { label: '近 30 天', requests: period.value.month_requests, chars: period.value.month_chars },
])

const last30DailyStats = computed(() => {
  const daily = store.chartData?.daily ?? []
  return daily.slice(-30)
})

const miniChartCaption = computed(() => '近 30 天每日分布')

const headerStatsRange = computed(() => {
  if (headerRange.value === 1) return 'today'
  if (headerRange.value === 7) return 'week'
  if (headerRange.value === 30) return 'month'
  return 'all'
})

const langPairStats = computed(() => {
  const merged = new Map<string, { lang: string; source_chars: number; target_chars: number; total_chars: number }>()

  for (const row of store.langStats) {
    const key = row.lang.toUpperCase()
    const current = merged.get(key) ?? {
      lang: key,
      source_chars: 0,
      target_chars: 0,
      total_chars: 0,
    }
    current.source_chars += row.source_chars
    current.target_chars += row.target_chars
    current.total_chars = current.source_chars + current.target_chars
    merged.set(key, current)
  }

  return [...merged.values()].sort((a, b) => {
    const aValue = a[langSortKey.value]
    const bValue = b[langSortKey.value]
    return langSortOrder.value === 'desc' ? bValue - aValue : aValue - bValue
  })
})

const langBarMaxes = computed(() => ({
  source: Math.max(...langPairStats.value.map(item => item.source_chars), 1),
  target: Math.max(...langPairStats.value.map(item => item.target_chars), 1),
  total: Math.max(...langPairStats.value.map(item => item.total_chars), 1),
}))

function buildHeatColors(values: number[]) {
  if (!values.length) return []

  const indexed = values.map((value, index) => ({ value, index }))
  indexed.sort((a, b) => a.value - b.value || a.index - b.index)

  const lowCount = Math.max(1, Math.floor(values.length / 3))
  const highStart = Math.max(lowCount, values.length - lowCount)

  const palette = Array<string>(values.length)

  for (let rank = 0; rank < indexed.length; rank++) {
    const originalIndex = indexed[rank].index
    if (rank < lowCount) {
      palette[originalIndex] = 'rgba(246, 70, 93, 0.92)'
    } else if (rank >= highStart) {
      palette[originalIndex] = 'rgba(14, 203, 129, 0.95)'
    } else {
      palette[originalIndex] = 'rgba(234, 179, 8, 0.9)'
    }
  }

  return palette
}

function formatChartData() {
  const data = store.chartData
  if (!data) return { labels: [] as string[], requests: [] as number[], chars: [] as number[] }

  if (activeChartTab.value === 'hourly') {
    const hourMap = new Map(data.hourly.map(h => [h.hour.split(' ')[1].split(':')[0], h]))
    const labels: string[] = []
    const requests: number[] = []
    const chars: number[] = []

    for (let i = 0; i < 24; i++) {
      const key = i.toString().padStart(2, '0')
      const entry = hourMap.get(key)
      labels.push(`${key}:00`)
      requests.push(entry?.count ?? 0)
      chars.push(entry?.chars ?? 0)
    }

    return { labels, requests, chars }
  }

  const dayMap = new Map(data.daily.map(d => [d.day, d]))
  const labels: string[] = []
  const requests: number[] = []
  const chars: number[] = []

  const daysToShow = 30

  for (let i = 0; i < daysToShow; i++) {
    const date = new Date()
    date.setDate(date.getDate() - (daysToShow - 1 - i))
    const key = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
    const entry = dayMap.get(key)
    labels.push(`${date.getMonth() + 1}/${date.getDate()}`)
    requests.push(entry?.count ?? 0)
    chars.push(entry?.chars ?? 0)
  }

  return { labels, requests, chars }
}

function createCharts() {
  if (!store.chartData || !Chart) return
  requestsChart?.destroy()
  charsChart?.destroy()
  langUsageTrendChart?.destroy()
  requestsChart = null
  charsChart = null
  langUsageTrendChart = null

  const { labels, requests, chars } = formatChartData()
  const formatYAxis = (value: string | number) => {
    const num = typeof value === 'number' ? value : Number(value)
    if (num >= 10000) return `${(num / 10000).toFixed(num >= 100000 ? 0 : 1)}w`
    if (num >= 1000) return `${(num / 1000).toFixed(num >= 10000 ? 0 : 1)}k`
    return `${num}`
  }
  const buildAreaGradient = (canvas: HTMLCanvasElement, topColor: string, bottomColor: string) => {
    const ctx = canvas.getContext('2d')
    if (!ctx) return topColor
    const gradient = ctx.createLinearGradient(0, 0, 0, canvas.height)
    gradient.addColorStop(0, topColor)
    gradient.addColorStop(1, bottomColor)
    return gradient
  }

  const baseOptions = {
    responsive: true,
    maintainAspectRatio: false,
    animation: false as const,
    interaction: {
      mode: 'index' as const,
      intersect: false,
      axis: 'x' as const,
    },
    plugins: {
      legend: { display: false },
      tooltip: {
        enabled: true,
        mode: 'index' as const,
        intersect: false,
        displayColors: false,
        backgroundColor: 'rgba(17, 24, 39, 0.96)',
        borderColor: 'rgba(71, 85, 105, 0.75)',
        borderWidth: 1,
        titleColor: '#ffffff',
        bodyColor: '#e5e7eb',
        padding: 12,
        cornerRadius: 10,
      },
    },
    layout: { padding: { left: 8, right: 16, top: 8, bottom: 0 } },
    scales: {
      x: { offset: true, grid: { color: 'rgba(255,255,255,0.05)' }, ticks: { color: '#707a8a', maxRotation: 0, autoSkip: true, maxTicksLimit: 12 } },
      y: {
        grid: { color: 'rgba(255,255,255,0.05)' },
        ticks: {
          color: '#94a3b8',
          callback(value: string | number) {
            return formatYAxis(value)
          },
        },
        beginAtZero: true,
      },
    },
  }

  const baseColors = [
    { lang: 'ZH', stroke: '#f6465d', fill: 'rgba(246,70,93,0.18)' },
    { lang: 'EN', stroke: '#60a5fa', fill: 'rgba(96,165,250,0.18)' },
    { lang: 'JA', stroke: '#f59e0b', fill: 'rgba(245,158,11,0.18)' },
    { lang: 'KO', stroke: '#0ecb81', fill: 'rgba(14,203,129,0.18)' },
    { lang: 'FR', stroke: '#8b5cf6', fill: 'rgba(139,92,246,0.18)' },
  ]

  if (langUsageTrendCanvas.value) {
    const timeLabels = activeChartTab.value === 'daily'
      ? Array.from({ length: 30 }, (_, i) => {
          const date = new Date()
          date.setDate(date.getDate() - (29 - i))
          return `${date.getMonth() + 1}/${date.getDate()}`
        })
      : Array.from({ length: 24 }, (_, i) => `${i.toString().padStart(2, '0')}:00`)
    const preferred = ['ZH', 'EN', 'JA', 'KO', 'FR']
    const usageMap = new Map<string, Map<string, number>>()
    for (const row of store.langHourlyStats) {
      const timeKey = activeChartTab.value === 'daily'
        ? (() => {
            const [_, month, day] = row.hour.split('-')
            return `${Number(month)}/${Number(day)}`
          })()
        : row.hour.split(' ')[1]
      if (!usageMap.has(row.lang.toUpperCase())) usageMap.set(row.lang.toUpperCase(), new Map())
      usageMap.get(row.lang.toUpperCase())!.set(timeKey, row.count)
    }
    const langs = preferred.filter(lang => usageMap.has(lang)).slice(0, 5)
    langUsageTrendChart = new Chart(langUsageTrendCanvas.value, {
      type: 'line',
      data: {
        labels: timeLabels,
        datasets: langs.map(lang => {
          const color = baseColors.find(item => item.lang === lang) ?? { stroke: '#94a3b8', fill: 'rgba(148,163,184,0.16)' }
          return {
            label: langFullName(lang),
            data: timeLabels.map(timeKey => usageMap.get(lang)?.get(timeKey) ?? 0),
            borderColor: color.stroke,
            backgroundColor: buildAreaGradient(
              langUsageTrendCanvas.value!,
              color.fill.replace(/0\.16\)$/, '0.26)'),
              color.fill.replace(/0\.16\)$/, '0.02)'),
            ),
            borderWidth: 2,
            fill: true,
            tension: 0.35,
            pointRadius: 0,
            pointHoverRadius: 4,
            pointHitRadius: 12,
            pointHoverBackgroundColor: '#ffffff',
            pointHoverBorderColor: color.stroke,
          }
        }),
      },
      options: {
        ...baseOptions,
        layout: { padding: { left: 8, right: 16, top: 8, bottom: 2 } },
        plugins: {
          ...baseOptions.plugins,
          legend: {
            display: true,
            position: 'bottom',
            align: 'center',
            labels: { color: '#cbd5e1', usePointStyle: true, boxWidth: 6, boxHeight: 6, padding: 8, font: { size: 10 } },
          },
          tooltip: {
            ...baseOptions.plugins.tooltip,
            callbacks: {
              title(items: { label?: string }[]) {
                return items[0]?.label ?? ''
              },
              label(item: { dataset: { label?: string }, parsed: { y: number | null } }) {
                return `${item.dataset.label ?? ''}：${formatNumber(item.parsed.y ?? 0)}`
              },
            },
          },
        },
      },
    })
  }

  if (requestsChartCanvas.value) {
    requestsChart = new Chart(requestsChartCanvas.value, {
      type: 'line',
      data: {
        labels,
        datasets: [{
          data: requests,
          borderColor: '#60a5fa',
          backgroundColor: 'rgba(59,130,246,0.16)',
          borderWidth: 3,
          fill: true,
          tension: 0.35,
          pointRadius: 0,
          pointHoverRadius: 4,
          pointHitRadius: 12,
          pointHoverBorderWidth: 2,
          pointHoverBackgroundColor: '#ffffff',
          pointHoverBorderColor: '#60a5fa',
        }],
      },
      options: {
        ...baseOptions,
        plugins: {
          ...baseOptions.plugins,
          tooltip: {
            ...baseOptions.plugins.tooltip,
            callbacks: {
              title(items: { label?: string }[]) {
                return items[0]?.label ?? ''
              },
              label(item: { parsed: { y: number | null } }) {
                return `调用次数：${formatNumber(item.parsed.y ?? 0)}`
              },
            },
          },
        },
      },
    })
  }

  if (charsChartCanvas.value) {
    charsChart = new Chart(charsChartCanvas.value, {
      type: 'line',
      data: {
        labels,
        datasets: [{
          data: chars,
          borderColor: '#f59e0b',
          backgroundColor: 'rgba(245,158,11,0.16)',
          borderWidth: 3,
          fill: true,
          tension: 0.35,
          pointRadius: 0,
          pointHoverRadius: 4,
          pointHitRadius: 12,
          pointHoverBorderWidth: 2,
          pointHoverBackgroundColor: '#ffffff',
          pointHoverBorderColor: '#f59e0b',
        }],
      },
      options: {
        ...baseOptions,
        plugins: {
          ...baseOptions.plugins,
          tooltip: {
            ...baseOptions.plugins.tooltip,
            callbacks: {
              title(items: { label?: string }[]) {
                return items[0]?.label ?? ''
              },
              label(item: { parsed: { y: number | null } }) {
                return `字符数：${formatNumber(item.parsed.y ?? 0)}`
              },
            },
          },
        },
      },
    })
  }
}

function createMiniCharts() {
  const daily = last30DailyStats.value
  countMiniChart?.destroy()
  charsMiniChart?.destroy()
  countMiniChart = null
  charsMiniChart = null
  if (!daily.length || !Chart) return

  const labels = daily.map(d => d.day)
  const counts = daily.map(d => d.count)
  const chars = daily.map(d => d.chars)
  const maxCount = Math.max(...counts, 1)
  const maxChars = Math.max(...chars, 1)

  const countBg = buildHeatColors(counts)
  const charsBg = buildHeatColors(chars)

  function buildMiniBarChart(
    canvas: HTMLCanvasElement,
    chartLabels: string[],
    data: number[],
    bg: string[],
    maxVal: number,
    tooltipLabel: string,
  ): ChartType {
    canvas.width = canvas.offsetWidth
    canvas.height = 72

    const chart = new Chart!(canvas, {
      type: 'bar',
      data: {
        labels: chartLabels,
        datasets: [{
          data,
          backgroundColor: [...bg],
          borderRadius: 3,
          borderSkipped: false,
          borderWidth: 0,
          barPercentage: 0.72,
          categoryPercentage: 0.9,
          maxBarThickness: 14,
        }],
      },
      options: {
        responsive: false,
        maintainAspectRatio: false,
        animation: false as const,
        events: ['mousemove', 'mouseout'],
        interaction: { mode: 'index', intersect: false, axis: 'x' },
        layout: { padding: { left: 6, right: 6, top: 8, bottom: 0 } },
        plugins: {
          legend: { display: false },
          tooltip: {
            enabled: true,
            displayColors: false,
            backgroundColor: 'rgba(17, 24, 39, 0.96)',
            titleColor: '#ffffff',
            bodyColor: '#e5e7eb',
            padding: 8,
            cornerRadius: 6,
            callbacks: {
              title(items: any[]) { return items[0]?.label ?? '' },
              label(item: any) { return `${tooltipLabel}：${formatNumber(item.parsed.y ?? 0)}` },
            },
          },
        },
        scales: {
          x: { display: false, offset: true },
          y: { display: false, suggestedMax: maxVal * 1.2 },
        },
      },
      plugins: [{
        id: 'barScaleHighlight',
        beforeDraw(ch: any) {
          const hoverIdx = ch._hoverIdx ?? -1
          if (hoverIdx === -1) return
          const meta = ch.getDatasetMeta(0)
          const radius = 3
          for (let i = 0; i < meta.data.length; i++) {
            const dist = Math.abs(i - hoverIdx)
            if (dist > radius) continue
            const bar = meta.data[i] as any
            const scale = 1 + 0.6 * Math.cos((dist / radius) * (Math.PI / 2))
            const lift = 4 * Math.cos((dist / radius) * (Math.PI / 2))
            bar._saved = { width: bar.width, y: bar.y }
            bar.width = bar.width * scale
            bar.y = bar.y - lift
          }
        },
        afterDraw(ch: any) {
          const hoverIdx = ch._hoverIdx ?? -1
          if (hoverIdx === -1) return
          const meta = ch.getDatasetMeta(0)
          for (let i = 0; i < meta.data.length; i++) {
            const bar = meta.data[i] as any
            if (bar._saved) {
              bar.width = bar._saved.width
              bar.y = bar._saved.y
              delete bar._saved
            }
          }
        },
      }],
    })

    canvas.onmousemove = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect()
      const x = e.clientX - rect.left
      const area = (chart as any).chartArea
      if (!area) return
      const relX = x - area.left
      const usable = area.right - area.left
      const idx = Math.max(0, Math.min(bg.length - 1, Math.floor(relX / (usable / bg.length))))
      if ((chart as any)._hoverIdx === idx) return
      ;(chart as any)._hoverIdx = idx
      chart.data.datasets[0].backgroundColor = bg.map((c: string, i: number) =>
        i === idx ? c.replace(/[\d.]+\)$/, '1)') : c.replace(/[\d.]+\)$/, '0.18)')
      )
      chart.update('none')
    }

    canvas.onmouseleave = () => {
      ;(chart as any)._hoverIdx = -1
      chart.data.datasets[0].backgroundColor = [...bg]
      chart.update('none')
    }

    return chart
  }

  if (countMiniChartCanvas.value) {
    countMiniChart = buildMiniBarChart(countMiniChartCanvas.value, labels, counts, countBg, maxCount, '调用次数')
  }

  if (charsMiniChartCanvas.value) {
    charsMiniChart = buildMiniBarChart(charsMiniChartCanvas.value, labels, chars, charsBg, maxChars, '字符数')
  }
}

function latencyColor(ms: number) {
  if (ms <= 500) return 'color-good'
  if (ms <= 1500) return 'color-warn'
  return 'color-bad'
}

function successRateColor(rate: number) {
  if (rate >= 95) return 'color-good'
  if (rate >= 80) return 'color-warn'
  return 'color-bad'
}

function formatTtl(secs: number): string {
  if (secs >= 3600 && secs % 3600 === 0) return `${secs / 3600}h`
  if (secs >= 60 && secs % 60 === 0) return `${secs / 60}m`
  return `${secs}s`
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`
}

function barStyle(value: number, lang: string, metric: 'source' | 'target' | 'total') {
  const max = langBarMaxes.value[metric]
  const color = `${LANG_COLOR[lang.toUpperCase()] || '#3b82f6'}cc`
  return {
    width: `${Math.max(6, Math.round((value / max) * 100))}%`,
    background: color,
  }
}

function barLabelStyle(value: number) {
  const text = showLangMetricDetail.value ? formatNumber(value) : formatCompactNumber(value)
  const len = text.length
  if (len <= 8) return {}
  if (len <= 10) return { fontSize: '10px' }
  if (len <= 12) return { fontSize: '9px' }
  return { fontSize: '8px' }
}

function toggleLangSort(key: 'source_chars' | 'target_chars' | 'total_chars') {
  if (langSortKey.value === key) {
    langSortOrder.value = langSortOrder.value === 'desc' ? 'asc' : 'desc'
  } else {
    langSortKey.value = key
    langSortOrder.value = 'desc'
  }
}

function sortIndicator(key: 'source_chars' | 'target_chars' | 'total_chars') {
  if (langSortKey.value !== key) return ''
  return langSortOrder.value === 'desc' ? '↓' : '↑'
}

function formatCompactNumber(value: number) {
  const abs = Math.abs(value)
  if (abs >= 1_000_000_000_000) return `${(value / 1_000_000_000_000).toFixed(1).replace(/\.0$/, '')}T`
  if (abs >= 1_000_000_000) return `${(value / 1_000_000_000).toFixed(1).replace(/\.0$/, '')}B`
  if (abs >= 1_000_000) return `${(value / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`
  if (abs >= 1_000) return `${(value / 1_000).toFixed(1).replace(/\.0$/, '')}K`
  return `${value}`
}

function toggleStatDisplay() {
  showFullStat.value = !showFullStat.value
  for (const el of [statRequestsEl.value, statCharsEl.value]) {
    if (el) el.style.fontSize = ''
  }
  nextTick(() => {
    requestAnimationFrame(fitStatFontSize)
  })
}

function fitStatFontSize() {
  for (const el of [statRequestsEl.value, statCharsEl.value]) {
    if (!el) continue
    if (el.scrollWidth <= el.clientWidth) continue
    const currentSize = parseInt(getComputedStyle(el).fontSize) || 44
    // 用比例估算起点，然后向上微调找到刚好不溢出的最大字号
    const ratio = el.clientWidth / el.scrollWidth
    let size = Math.max(16, Math.round(currentSize * ratio))
    el.style.fontSize = `${size}px`
    // 向上逐步放大直到溢出
    while (el.scrollWidth <= el.clientWidth && size < currentSize) {
      size++
      el.style.fontSize = `${size}px`
    }
    // 如果溢出了，回退一步
    if (el.scrollWidth > el.clientWidth) {
      size--
      el.style.fontSize = `${size}px`
    }
  }
}

let _prevStatLen = 0
watch(() => store.stats, () => {
  if (!showFullStat.value) return
  const s = store.stats
  const len = formatNumber(s?.total_requests ?? 0).length + formatNumber(s?.total_chars ?? 0).length
  if (len !== _prevStatLen) {
    _prevStatLen = len
    nextTick(() => requestAnimationFrame(fitStatFontSize))
  }
})

async function refreshMainPanels() {
  await Promise.all([
    store.fetchStats(headerStatsRange.value),
    store.fetchLangHourlyStats(activeChartTab.value === 'daily' ? 30 : 1),
    store.fetchChart(),
    store.fetchUpstreamStatus(),
    store.fetchCacheStats(),
  ])
  await nextTick()
  await ensureChartJs()
  createCharts()
  createMiniCharts()
}

async function refreshLangPanels() {
  await store.fetchLangStats(langRange.value)
}

async function refreshRequestsPanel(page = currentPage.value) {
  await store.fetchRequests(page, pageSize.value)
}

async function refreshAll() {
  await Promise.all([
    refreshMainPanels(),
    refreshLangPanels(),
    refreshRequestsPanel(),
    store.fetchHeatmap(heatmapView.value),
    store.fetchErrorTrend(errorTrendDays.value),
  ])
  renderErrorTrendChart()
}

async function openSettings() {
  saveMsg.value = ''
  showSettings.value = true
  store.fetchVersion()
  const cfg = await store.fetchFullConfig()
  if (cfg) {
    settingsFullForm.upstream.endpoints = cfg.upstream.endpoints.map(ep => ({ ...ep }))
    settingsFullForm.upstream.max_failures = cfg.upstream.max_failures
    settingsFullForm.upstream.probe_interval_secs = cfg.upstream.probe_interval_secs
    settingsFullForm.proxy.host = cfg.proxy.host
    settingsFullForm.proxy.port = cfg.proxy.port
    settingsFullForm.monitor.auto_refresh_seconds = cfg.monitor.auto_refresh_seconds
    settingsFullForm.monitor.max_log_entries = cfg.monitor.max_log_entries
    settingsFullForm.health_check.source_lang = cfg.health_check.source_lang
    settingsFullForm.health_check.target_lang = cfg.health_check.target_lang
    settingsFullForm.cache.enabled = cfg.cache.enabled
    settingsFullForm.cache.ttl_secs = cfg.cache.ttl_secs
    settingsFullForm.cache.max_entries = cfg.cache.max_entries
    settingsFullForm.cache.max_memory_mb = cfg.cache.max_memory_mb ?? 0
    // 自动选择最合适的时间单位
    const secs = cfg.cache.ttl_secs
    if (secs >= 3600 && secs % 3600 === 0) cacheTtlUnit.value = 'h'
    else if (secs >= 60 && secs % 60 === 0) cacheTtlUnit.value = 'm'
    else cacheTtlUnit.value = 's'
    settingsFullForm.demo.enabled = cfg.demo.enabled
    settingsFullForm.demo.seed = cfg.demo.seed
    settingsSnapshot.value = JSON.stringify(settingsFullForm)
  }
}

function addEndpoint() {
  settingsFullForm.upstream.endpoints.push({ name: '', url: '', api_key: '' })
}

function removeEndpoint(idx: number) {
  settingsFullForm.upstream.endpoints.splice(idx, 1)
}

const endpointTestResults = ref<Record<number, { ok: boolean; latency: number }>>({})
const endpointTesting = ref<Record<number, boolean>>({})

async function testEndpoint(idx: number) {
  const ep = settingsFullForm.upstream.endpoints[idx]
  if (!ep?.url) return
  endpointTesting.value = { ...endpointTesting.value, [idx]: true }
  const start = performance.now()
  try {
    const body = JSON.stringify({ text: 'hi', source_lang: 'EN', target_lang: 'ZH' })
    const headers: Record<string, string> = { 'Content-Type': 'application/json' }
    if (ep.api_key) headers['Authorization'] = `Bearer ${ep.api_key}`
    const res = await fetch(ep.url, { method: 'POST', headers, body, signal: AbortSignal.timeout(10000) })
    const latency = Math.round(performance.now() - start)
    endpointTestResults.value = { ...endpointTestResults.value, [idx]: { ok: res.ok, latency } }
  } catch {
    const latency = Math.round(performance.now() - start)
    endpointTestResults.value = { ...endpointTestResults.value, [idx]: { ok: false, latency } }
  }
  endpointTesting.value = { ...endpointTesting.value, [idx]: false }
}

function closeSettings() {
  showSettings.value = false
}

async function onHealthCheck() {
  checking.value = true
  await store.triggerHealthCheck()
  await store.fetchUpstreamStatus()
  checking.value = false
}

async function onErrorTrendChange(days: number) {
  errorTrendDays.value = days
  await store.fetchErrorTrend(days)
  renderErrorTrendChart()
}

async function onHeatmapViewChange(view: string) {
  heatmapView.value = view
  await store.fetchHeatmap(view)
}

function renderErrorTrendChart() {
  if (!errorTrendCanvas.value || !Chart) return
  errorTrendChart?.destroy()
  const data = store.errorTrend
  errorTrendChart = new Chart(errorTrendCanvas.value, {
    type: 'line',
    data: {
      labels: data.map(d => d.time.replace(/^\d{4}-/, '').replace(' ', '\n')),
      datasets: [{
        label: '错误率',
        data: data.map(d => +(d.error_rate * 100).toFixed(1)),
        borderColor: '#ef4444',
        backgroundColor: 'rgba(239, 68, 68, 0.1)',
        fill: true,
        tension: 0.3,
      }],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: { legend: { display: false } },
      scales: {
        y: { beginAtZero: true, ticks: { callback: (v) => v + '%' } },
      },
    },
  })
}

const heatmapGridStyle = computed(() => {
  if (!store.heatmapData?.data?.length) return {}
  const xValues = [...new Set(store.heatmapData.data.map(d => d.x))]
  return {
    display: 'grid',
    gridTemplateColumns: `repeat(${xValues.length}, 1fr)`,
    gridTemplateRows: 'repeat(24, 1fr)',
    gap: '2px',
  }
})

function heatmapCellStyle(count: number) {
  if (!store.heatmapData?.data?.length) return {}
  const max = Math.max(...store.heatmapData.data.map(d => d.count), 1)
  const intensity = count / max
  return {
    backgroundColor: `rgba(59, 130, 246, ${0.1 + intensity * 0.8})`,
    borderRadius: '2px',
    minHeight: '12px',
  }
}

async function onHeaderRefreshChange() {
  await store.updateConfig({ auto_refresh_seconds: headerRefresh.value })
  if (headerRefresh.value > 0) {
    store.startAutoRefresh(headerRefresh.value, refreshAll)
  } else {
    store.stopAutoRefresh()
  }
  await store.fetchStats(headerStatsRange.value)
}

async function onHeaderRangeChange() {
  localStorage.setItem(HEADER_RANGE_STORAGE_KEY, String(headerRange.value))
  await refreshMainPanels()
}

async function onManualRefresh() {
  if (refreshing.value) return
  const animStartedAt = performance.now()
  refreshAnimating.value = true
  refreshing.value = true
  await Promise.all([
    refreshMainPanels(),
    refreshLangPanels(),
    refreshRequestsPanel(),
  ])
  refreshing.value = false
  const animElapsed = performance.now() - animStartedAt
  const minVisibleSpin = 400
  if (animElapsed < minVisibleSpin) {
    await new Promise(resolve => setTimeout(resolve, minVisibleSpin - animElapsed))
  }
  refreshAnimating.value = false
  const id = ++manualRefreshToastSeq
  manualRefreshToasts.value.unshift({ id, message: '刷新成功' })
  setTimeout(() => {
    manualRefreshToasts.value = manualRefreshToasts.value.filter(item => item.id !== id)
  }, 1500)
}

function manualToastStyle(index: number) {
  const offset = Math.min(index, 4)
  return {
    transform: `translateX(-50%) translateY(${-offset * 12}px) scale(${1 - offset * 0.04})`,
    opacity: `${Math.max(0, 1 - offset * 0.32)}`,
    zIndex: `${120 - offset}`,
  }
}

async function onLangRangeChange() {
  await refreshLangPanels()
}

function switchToCacheHits() {
  logTab.value = 'cache-hits'
  store.fetchCacheHitLogs()
}


async function onPageSizeChange() {
  currentPage.value = 1
  await refreshRequestsPanel(1)
}

async function changePage(page: number) {
  const nextPage = Math.min(Math.max(page, 1), totalPages.value)
  currentPage.value = nextPage
  await refreshRequestsPanel(nextPage)
}

async function handleClearCache() {
  clearingCache.value = true
  clearCacheMsg.value = ''
  const ok = await store.clearCache()
  clearingCache.value = false
  clearCacheMsg.value = ok ? '已清除' : '清除失败'
  setTimeout(() => { clearCacheMsg.value = '' }, 2000)
}

async function saveSettings() {
  saving.value = true
  saveMsg.value = ''

  const ok = await store.updateFullConfig({
    upstream: {
      endpoints: settingsFullForm.upstream.endpoints.map(ep => ({ ...ep })),
      max_failures: settingsFullForm.upstream.max_failures,
      probe_interval_secs: settingsFullForm.upstream.probe_interval_secs,
    },
    proxy: { ...settingsFullForm.proxy },
    monitor: { ...settingsFullForm.monitor },
    health_check: { ...settingsFullForm.health_check },
    cache: { ...settingsFullForm.cache },
    demo: { ...settingsFullForm.demo },
  })

  saving.value = false
  saveSuccess.value = ok
  saveMsg.value = ok ? '设置已保存！' : '保存失败'

  if (ok) {
    settingsSnapshot.value = JSON.stringify(settingsFullForm)
    // 同步 header 刷新控件
    headerRefresh.value = settingsFullForm.monitor.auto_refresh_seconds
    if (headerRefresh.value > 0) {
      store.startAutoRefresh(headerRefresh.value, refreshAll)
    } else {
      store.stopAutoRefresh()
    }
    await refreshAll()
  }

  setTimeout(() => {
    saveMsg.value = ''
  }, 2000)
}

const _numFmt = new Intl.NumberFormat()
function formatNumber(n: number) {
  return _numFmt.format(n)
}

function formatTime(ts: string) {
  if (!ts) return '-'
  try {
    const d = new Date(ts)
    return d.toLocaleString('zh-CN', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false,
    })
  } catch {
    return ts
  }
}

function formatCheckedAt(ts: string) {
  if (!ts) return '-'
  try {
    const d = new Date(ts)
    const pad = (n: number) => n.toString().padStart(2, '0')
    return `${d.getFullYear()}/${d.getMonth() + 1}/${d.getDate()} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  } catch {
    return ts
  }
}

function langFlagUrl(code: string) {
  return FLAG_URL[code.toUpperCase()] || ''
}

function langFullName(code: string) {
  const normalized = code.toUpperCase()
  const name = LANG_MAP[normalized] || normalized
  return `${normalized}（${name}）`
}

watch(activeUpstream, () => { upstreamUrlRevealed.value = false })

watch(activeChartTab, async () => {
  await store.fetchLangHourlyStats(activeChartTab.value === 'daily' ? 30 : 1)
  await nextTick()
  await ensureChartJs()
  createCharts()
})

watch(langPairStats, async () => {
  await nextTick()
})

onMounted(async () => {
  const savedHeaderRange = localStorage.getItem(HEADER_RANGE_STORAGE_KEY)
  if (savedHeaderRange !== null) {
    const parsed = Number(savedHeaderRange)
    if ([0, 1, 7, 30, 90].includes(parsed)) {
      headerRange.value = parsed
    }
  }
  await refreshAll()
  headerRefresh.value = store.config.auto_refresh_seconds
  if (headerRefresh.value > 0) {
    store.startAutoRefresh(headerRefresh.value, refreshAll)
  }
})

onUnmounted(() => {
  requestsChart?.destroy()
  charsChart?.destroy()
  langUsageTrendChart?.destroy()
  errorTrendChart?.destroy()
  countMiniChart?.destroy()
  charsMiniChart?.destroy()
  store.stopAutoRefresh()
})

const LANG_MAP: Record<string, string> = {
  AUTO: '自动检测', ZH: '中文', EN: '英语', JA: '日语', KO: '韩语',
  FR: '法语', DE: '德语', ES: '西班牙语', IT: '意大利语',
  PT: '葡萄牙语', RU: '俄语', NL: '荷兰语', PL: '波兰语',
  TR: '土耳其语', UK: '乌克兰语', DA: '丹麦语', SV: '瑞典语',
  FI: '芬兰语', CS: '捷克语', RO: '罗马尼亚语', HU: '匈牙利语',
  TH: '泰语', VI: '越南语', ID: '印尼语', MS: '马来语',
  HE: '希伯来语', AR: '阿拉伯语', HI: '印地语', BN: '孟加拉语',
  'ZH-HANS': '简体中文', 'ZH-HANT': '繁体中文', EL: '希腊语',
}

const LANG_COLOR: Record<string, string> = {
  ZH: '#f6465d', EN: '#3b82f6', JA: '#f59e0b', KO: '#0ecb81',
  FR: '#8b5cf6', DE: '#ec4899', ES: '#06b6d4', IT: '#84cc16',
  PT: '#f97316', RU: '#14b8a6', NL: '#a855f7', PL: '#f43f5e',
  TR: '#eab308', UK: '#3b82f6', DA: '#ef4444', SV: '#22c55e',
  FI: '#06b6d4', CS: '#f97316', RO: '#8b5cf6', HU: '#ec4899',
  TH: '#f59e0b', VI: '#14b8a6', ID: '#f43f5e', MS: '#22c55e',
  HE: '#3b82f6', AR: '#14b8a6', HI: '#f97316', BN: '#22c55e',
  'ZH-HANS': '#f6465d', 'ZH-HANT': '#f43f5e', EL: '#3b82f6',
}

const FLAG_URL: Record<string, string> = {
  AUTO: '', ZH: 'https://flagcdn.com/w20/cn.png', EN: 'https://flagcdn.com/w20/gb.png', JA: 'https://flagcdn.com/w20/jp.png', KO: 'https://flagcdn.com/w20/kr.png',
  FR: 'https://flagcdn.com/w20/fr.png', DE: 'https://flagcdn.com/w20/de.png', ES: 'https://flagcdn.com/w20/es.png', IT: 'https://flagcdn.com/w20/it.png',
  PT: 'https://flagcdn.com/w20/pt.png', RU: 'https://flagcdn.com/w20/ru.png', NL: 'https://flagcdn.com/w20/nl.png', PL: 'https://flagcdn.com/w20/pl.png',
  TR: 'https://flagcdn.com/w20/tr.png', UK: 'https://flagcdn.com/w20/ua.png', DA: 'https://flagcdn.com/w20/dk.png', SV: 'https://flagcdn.com/w20/se.png',
  FI: 'https://flagcdn.com/w20/fi.png', CS: 'https://flagcdn.com/w20/cz.png', RO: 'https://flagcdn.com/w20/ro.png', HU: 'https://flagcdn.com/w20/hu.png',
  TH: 'https://flagcdn.com/w20/th.png', VI: 'https://flagcdn.com/w20/vn.png', ID: 'https://flagcdn.com/w20/id.png', MS: 'https://flagcdn.com/w20/my.png',
  HE: 'https://flagcdn.com/w20/il.png', AR: 'https://flagcdn.com/w20/sa.png', HI: 'https://flagcdn.com/w20/in.png', BN: 'https://flagcdn.com/w20/bd.png',
  'ZH-HANS': 'https://flagcdn.com/w20/cn.png', 'ZH-HANT': 'https://flagcdn.com/w20/tw.png', EL: 'https://flagcdn.com/w20/gr.png',
}
</script>

<style scoped>
.dashboard-page {
  min-height: 100vh;
}

.manual-refresh-toast-stack {
  position: fixed;
  top: 28px;
  left: 50%;
  z-index: 120;
  width: 0;
  height: 0;
  pointer-events: none;
}

.manual-refresh-toast {
  position: fixed;
  left: 50%;
  padding: 10px 18px;
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.96);
  border: 1px solid rgba(96, 165, 250, 0.24);
  color: #f8fafc;
  font-size: 13px;
  font-weight: 600;
  box-shadow: 0 10px 24px rgba(2, 6, 23, 0.28);
}

.dashboard-shell {
  display: flex;
  flex-direction: column;
  gap: var(--space-lg);
}

.hero-panel {
  position: sticky;
  top: 0;
  z-index: 40;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-lg);
  padding: 14px 20px;
  background:
    linear-gradient(135deg, rgba(23, 37, 56, 0.96), rgba(15, 23, 42, 0.96) 56%, rgba(11, 14, 17, 0.98));
  backdrop-filter: blur(12px);
  border: 1px solid rgba(96, 165, 250, 0.24);
  box-shadow:
    0 10px 30px rgba(2, 6, 23, 0.35),
    inset 0 1px 0 rgba(148, 163, 184, 0.08);
}

.hero-copy {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.hero-logo {
  width: 34px;
  height: 34px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: #ffffff;
  border-radius: 10px;
  box-shadow: 0 2px 10px rgba(15, 23, 42, 0.18);
  flex-shrink: 0;
}

.hero-logo-img {
  display: block;
}

.hero-title {
  font-size: 24px;
  line-height: 1.1;
  color: #f8fafc;
  letter-spacing: 0.01em;
  text-shadow: 0 1px 0 rgba(255,255,255,0.04);
  margin: 0;
}

.hero-controls {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  align-self: flex-start;
  gap: var(--space-md);
  max-width: 100%;
  width: auto;
  min-width: 0;
}

.control-field {
  min-width: 190px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 6px 8px 6px 12px;
  border-radius: var(--radius-md);
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(148, 163, 184, 0.08);
}

.control-label {
  font-size: 12px;
  color: #cbd5e1;
  font-weight: 500;
  white-space: nowrap;
}

.control-select {
  min-width: 112px;
  height: 40px;
  width: auto;
  padding-left: 12px;
  padding-right: 32px;
  background-color: rgba(30, 35, 41, 0.92);
}

.refresh-control-group {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.refresh-icon-btn {
  width: 40px;
  height: 40px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  border: 1px solid rgba(148, 163, 184, 0.14);
  background: rgba(30, 35, 41, 0.92);
  color: #cbd5e1;
  transition: background-color 0.15s, color 0.15s, border-color 0.15s;
}

.refresh-icon-btn:hover:not(:disabled) {
  background: rgba(43, 49, 57, 0.96);
  color: #ffffff;
  border-color: rgba(96, 165, 250, 0.32);
}

.refresh-icon-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.refresh-icon-btn svg {
  width: 16px;
  height: 16px;
}

.refresh-icon-btn.spinning svg {
  animation: refresh-spin 0.55s linear infinite;
  transform-origin: center;
}

@keyframes refresh-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--space-lg);
}

.stat-card {
  display: flex;
  flex-direction: column;
}

.emphasis-card {
  background: linear-gradient(180deg, rgba(43,49,57,1), rgba(30,35,41,1));
}

.stat-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-sm);
  margin-bottom: var(--space-md);
}

.stat-label {
  color: var(--body-muted);
  font-size: 13px;
}

.stat-trend {
  color: var(--muted-strong);
  font-size: 12px;
  text-align: right;
}

.stat-value {
  font-size: 44px;
  font-weight: 600;
  color: var(--on-dark);
  margin-bottom: var(--space-sm);
}

.stat-value.clickable {
  cursor: pointer;
  transition: opacity 0.15s, font-size 0.15s;
  overflow: hidden;
  white-space: nowrap;
}

.stat-value.clickable:hover {
  opacity: 0.7;
}

.mini-chart-caption {
  color: var(--body-muted);
  font-size: 12px;
  margin-top: var(--space-sm);
}

.mini-chart-wrap {
  margin-top: auto;
  border-radius: var(--radius-md);
  overflow: hidden;
}

.mini-chart-wrap canvas {
  display: block;
  width: 100%;
  height: 72px;
}

.overview-card {
  display: flex;
  flex-direction: column;
}

.overview-metrics {
  display: grid;
  grid-template-columns: 1fr;
  gap: 8px;
  flex: 1;
}

.overview-metric-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-sm);
  padding: 10px 14px;
  border-radius: var(--radius-md);
  background: rgba(255,255,255,0.03);
  border: 1px solid rgba(148, 163, 184, 0.08);
}

.overview-metric-label {
  color: var(--body-muted);
  font-size: 12px;
  font-weight: 500;
}

.overview-metric-value {
  color: var(--on-dark);
  font-size: 16px;
  font-weight: 600;
}

.overview-metric-unit {
  color: var(--body-muted);
  font-size: 11px;
  margin-left: 2px;
}

.health-check-result {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  margin-top: var(--space-lg);
  padding: 10px 14px;
  border-radius: var(--radius-md);
  background: rgba(255,255,255,0.03);
  border: 1px solid rgba(148, 163, 184, 0.08);
  font-size: 12px;
}

.health-check-result-label {
  color: var(--body-muted);
  font-weight: 500;
  white-space: nowrap;
}

.health-check-result-time {
  color: var(--body-muted);
  margin-left: auto;
  white-space: nowrap;
}

.health-check-result-latency {
  color: var(--on-dark);
  font-weight: 600;
}

.badge-sm {
  font-size: 11px;
  padding: 2px 6px;
}

.period-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--space-lg);
}

.period-card {
  display: grid;
  gap: var(--space-sm);
}

.period-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--on-dark);
}

.period-row {
  display: flex;
  justify-content: space-between;
  gap: var(--space-sm);
}

.period-key {
  color: var(--body-muted);
  font-size: 12px;
}

.period-value {
  color: var(--on-dark);
  font-size: 18px;
  font-weight: 600;
}

.dashboard-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-lg);
}

.section-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

.clip-card {
  overflow: hidden;
}

.section-topbar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-md);
  flex-wrap: wrap;
}

.section-title {
  color: var(--on-dark);
  font-size: 18px;
  margin-bottom: 4px;
}

.mode-badge {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(59, 130, 246, 0.15);
  color: var(--info);
  font-weight: 500;
  vertical-align: middle;
}

.section-subtitle {
  color: var(--body-muted);
  font-size: 13px;
}

.section-actions {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.lang-section-actions {
  margin-left: auto;
  justify-content: flex-end;
}

.panel-surface {
  background: rgba(255,255,255,0.02);
  border: 1px solid rgba(255,255,255,0.04);
}

.chart-wrap canvas {
  display: block;
  width: 100%;
  flex: 1;
  min-height: 0;
}

.lang-list-shell,
.table-scroll,
.compact-scroll {
  overflow-x: auto;
}

.lang-list-shell {
  overflow-x: hidden;
}

.lang-list-shell--expanded {
  overflow-x: auto;
}

.lang-table {
  width: 100%;
  min-width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
}

.lang-table th:first-child,
.lang-table td:first-child {
  width: 26%;
}

.request-table {
  width: 100%;
  min-width: 100%;
  border-collapse: collapse;
}

.lang-table th,
.lang-table td,
.request-table th,
.request-table td {
  white-space: nowrap;
}

.lang-table th:not(:first-child) {
  text-align: left;
  padding-left: 14px;
  padding-right: 12px;
}

.lang-entry-cell {
  vertical-align: middle;
  padding: 6px 8px;
}

.lang-entry-card {
  display: flex;
  flex-direction: column;
  justify-content: center;
  height: 100%;
  padding: 0;
}

.lang-cell {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.lang-entry-cell:first-child .lang-entry-card {
  min-height: 40px;
}

.lang-bar-cell {
  width: 100%;
  padding-left: 6px;
  padding-right: 4px;
}

.lang-bar-track {
  position: relative;
  height: 24px;
  border-radius: 4px;
  background: rgba(255,255,255,0.06);
  overflow: hidden;
}

.lang-bar-fill {
  position: absolute;
  top: 0;
  left: 0;
  bottom: 0;
  border-radius: inherit;
}

.lang-bar-label {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  height: 100%;
  padding: 0 8px;
  font-size: 11px;
  font-weight: 600;
  color: var(--on-dark);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}

.color-good { color: var(--trading-up) !important; }
.color-warn { color: #f59e0b !important; }
.color-bad { color: var(--trading-down) !important; }

.sortable-th {
  cursor: pointer;
  user-select: none;
}

.sortable-th:hover {
  color: var(--on-dark);
}

.sort-indicator {
  display: inline-block;
  min-width: 12px;
  margin-left: 4px;
  color: var(--muted-strong);
}

.flag-img {
  border-radius: 2px;
  flex-shrink: 0;
}

.trend-card .charts-grid {
  display: grid;
  grid-template-columns: 1fr;
  grid-template-rows: 1fr 1fr 1fr;
  gap: 8px;
  height: 780px;
  max-height: 780px;
  overflow: hidden;
}

.chart-wrap {
  display: flex;
  flex-direction: column;
  padding: 10px;
  border-radius: var(--radius-lg);
  min-height: 0;
  height: 100%;
  overflow: hidden;
}

.lang-usage-wrap {
  padding-bottom: 2px;
}

.chart-label {
  color: var(--body-muted);
  font-size: 12px;
  margin-bottom: 4px;
}

.chart-tabs {
  display: inline-flex;
  flex-wrap: wrap;
  gap: var(--space-xs);
}

.chart-tab {
  background: var(--surface-elevated-dark);
  color: var(--body-muted);
  border: 1px solid var(--hairline-on-dark);
  border-radius: var(--radius-sm);
  padding: 6px 12px;
  font-size: 12px;
  font-weight: 500;
  transition: all 0.15s;
  cursor: pointer;
}

.chart-tab:hover {
  color: var(--on-dark);
}

.chart-tab.active {
  background: var(--primary);
  color: var(--ink);
  border-color: var(--primary);
}

.request-card {
  gap: var(--space-lg);
}

.request-topbar {
  align-items: center;
}

.log-tabs {
  display: inline-flex;
  gap: 4px;
  background: var(--surface-card-dark);
  border-radius: var(--radius-md);
  padding: 3px;
  border: 1px solid var(--hairline-on-dark);
}

.log-tab {
  background: transparent;
  color: var(--body-muted);
  border: none;
  border-radius: calc(var(--radius-md) - 2px);
  padding: 6px 16px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.log-tab:hover {
  color: var(--on-dark);
  background: rgba(255, 255, 255, 0.05);
}

.log-tab.active {
  background: var(--primary);
  color: var(--ink);
  font-weight: 600;
}

.text-preview {
  display: inline-block;
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  color: var(--body-muted);
}

.text-center {
  text-align: center;
}

.request-toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  flex-wrap: wrap;
  justify-content: flex-end;
}

.request-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-sm);
  flex-wrap: wrap;
}

.table-count {
  color: var(--body-muted);
  font-size: 13px;
}

.table-scroll {
  max-height: 520px;
  overflow: auto;
  border: 1px solid rgba(255,255,255,0.05);
  border-radius: var(--radius-lg);
  background: rgba(255,255,255,0.02);
}

.request-table {
  width: 100%;
  min-width: 980px;
  border-collapse: separate;
  border-spacing: 0;
}

.request-table thead th {
  position: sticky;
  top: 0;
  background: var(--surface-card-dark);
  z-index: 2;
}

.request-table tbody tr:nth-child(odd) {
  background: rgba(255,255,255,0.015);
}

.request-table tbody tr:hover {
  background: rgba(255,255,255,0.03);
}

.time-cell {
  color: var(--on-dark);
}

.error-message {
  display: inline-block;
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--body-muted);
  vertical-align: middle;
  white-space: nowrap;
}

.status-badge {
  width: auto;
  min-width: 64px;
  padding-left: 8px;
  padding-right: 8px;
}

.page-size-select {
  display: inline-flex;
  align-items: center;
  gap: var(--space-xs);
  color: var(--body-muted);
  font-size: 13px;
}

.page-size-input {
  width: 78px;
  min-width: 78px;
}

.pagination-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-sm);
  flex-wrap: wrap;
}

.page-numbers {
  display: inline-flex;
  gap: 4px;
}

.btn-page {
  min-width: 36px;
  height: 36px;
  background: var(--surface-card-dark);
  color: var(--body-muted);
  border: 1px solid var(--hairline-on-dark);
  border-radius: var(--radius-sm);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s;
}

.btn-page:hover {
  color: var(--on-dark);
  border-color: var(--muted-strong);
}

.btn-page.active {
  background: var(--primary);
  color: var(--ink);
  border-color: var(--primary);
}

.error-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-sm);
  background: rgba(246, 70, 93, 0.15);
  color: var(--trading-down);
  border: 1px solid rgba(246, 70, 93, 0.3);
  border-radius: var(--radius-md);
  padding: var(--space-sm) var(--space-md);
}

.error-text {
  min-width: 0;
}

.error-dismiss {
  background: none;
  border: none;
  color: var(--trading-down);
  font-size: 18px;
  padding: 0 4px;
}

.settings-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.58);
  backdrop-filter: blur(2px);
  z-index: 100;
}

.settings-drawer {
  position: fixed;
  top: 0;
  right: 0;
  width: min(520px, 100vw);
  height: 100vh;
  background: var(--surface-card-dark);
  z-index: 101;
  display: flex;
  flex-direction: column;
  box-shadow: -24px 0 48px rgba(0,0,0,0.35);
}

.drawer-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-lg);
  border-bottom: 1px solid var(--hairline-on-dark);
}

.drawer-header h3 {
  font-size: 20px;
  color: var(--on-dark);
  margin: 0;
}

.drawer-subtitle {
  color: var(--body-muted);
  font-size: 12px;
  margin-top: 4px;
}

.version-tag {
  font-size: 13px;
  font-weight: 400;
  color: var(--body-muted);
  margin-left: 10px;
  vertical-align: baseline;
}

.btn-github {
  color: var(--body-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  margin-right: 4px;
  transition: color 0.2s, background 0.2s;
}

.btn-github:hover {
  color: var(--on-dark);
  background: rgba(255, 255, 255, 0.08);
}

.drawer-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-lg);
}

.form-group {
  margin-bottom: var(--space-lg);
}

.form-label {
  display: block;
  color: var(--body-muted);
  font-size: 13px;
  margin-bottom: var(--space-xs);
}

.mono-input {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
  overflow-x: auto;
}

.input-hint {
  color: var(--body-muted);
  font-size: 12px;
  margin-top: 6px;
}

.drawer-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-md);
  padding: var(--space-sm);
  flex-shrink: 0;
}

.drawer-actions .btn-primary {
  opacity: 0.8;
}

.drawer-actions .btn-saved {
  background-color: var(--trading-up);
  color: #fff;
  opacity: 1;
}

.drawer-actions .btn-dirty {
  background-color: var(--trading-down);
  color: #fff;
  opacity: 1;
}

.save-msg {
  font-size: 13px;
  font-weight: 500;
}

.fade-enter-active,
.fade-leave-active,
.drawer-enter-active,
.drawer-leave-active {
  transition: all 0.22s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.drawer-enter-from,
.drawer-leave-to {
  transform: translateX(100%);
}

@media (max-width: 1080px) {
  .summary-grid,
  .period-grid,
  .dashboard-grid,
  .trend-card .charts-grid {
    grid-template-columns: 1fr;
  }

  .hero-panel {
    flex-direction: column;
    align-items: stretch;
  }

  .hero-controls,
  .request-toolbar {
    justify-content: flex-start;
  }

  .hero-controls {
    align-self: stretch;
    width: 100%;
  }
}

@media (max-width: 768px) {
  .dashboard-shell {
    gap: var(--space-md);
  }

  .hero-panel,
  .section-card,
  .request-card,
  .stat-card,
  .period-card {
    padding: var(--space-lg);
  }

  .hero-title {
    font-size: 24px;
  }

  .stat-value {
    font-size: 36px;
  }

  .hero-controls {
    flex-direction: column;
    align-items: stretch;
    max-width: none;
  }

  .control-field,
  .hero-actions {
    width: 100%;
  }

  .control-field {
    min-width: 0;
  }

  .hero-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
  }

  .section-topbar,
  .request-summary,
  .request-topbar {
    flex-direction: column;
    align-items: stretch;
  }

  .request-toolbar {
    flex-direction: column;
    align-items: stretch;
  }

  .page-numbers,
  .chart-tabs {
    width: 100%;
  }

  .page-numbers {
    padding-bottom: 4px;
  }

  .pagination-bar {
    justify-content: flex-start;
  }

  .error-message {
    max-width: 180px;
  }
}

/* 上游端点 + 缓存统计 */
.infra-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-lg);
}

.upstream-tabs-row {
  padding: 0 var(--space-lg);
  margin-bottom: var(--space-md);
  justify-content: center;
}

.upstream-panel {
  padding: 0 var(--space-lg) var(--space-sm);
}

.upstream-grid {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: var(--space-sm);
}

.upstream-grid-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.upstream-grid-label {
  font-size: 12px;
  color: var(--body-muted);
  height: 14px;
  line-height: 14px;
}

.upstream-grid-value {
  font-size: 24px;
  font-weight: 600;
  color: var(--on-dark);
  height: 30px;
  line-height: 30px;
}

.upstream-grid-unit {
  font-size: 12px;
  font-weight: 400;
  color: var(--body-muted);
  margin-left: 2px;
}

.upstream-grid-item .badge {
  font-size: 14px;
  font-weight: 600;
  padding: 0 10px;
  width: auto;
  height: 30px;
  line-height: 30px;
}

.badge-with-tooltip {
  position: relative;
  cursor: default;
}

.badge-with-tooltip[data-tooltip]:hover::after {
  content: attr(data-tooltip);
  position: absolute;
  bottom: calc(100% + 6px);
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.88);
  color: #fff;
  padding: 6px 10px;
  border-radius: 4px;
  font-size: 0.72rem;
  white-space: nowrap;
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
  z-index: 100;
  pointer-events: none;
}

.upstream-url-row {
  font-size: 12px;
  font-family: var(--font-mono, 'SF Mono', monospace);
  color: var(--body-muted);
  text-align: center;
  cursor: pointer;
  margin-top: var(--space-lg);
  padding: 10px 14px;
  border-radius: var(--radius-md);
  background: rgba(255,255,255,0.03);
  border: 1px solid rgba(148, 163, 184, 0.08);
  word-break: break-all;
  transition: opacity 0.15s;
}

.upstream-url-row:hover {
  opacity: 0.7;
}

.dot-ok { background: #22c55e; }
.dot-error { background: #ef4444; }

.cache-stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 2rem 1rem;
  padding: 2rem 1rem;
  text-align: center;
}

.cache-stat {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.cache-stat-label {
  font-size: 0.75rem;
  color: var(--text-muted, #6b7280);
}

.cache-stat-value {
  font-size: 1.1rem;
  font-weight: 600;
}

.btn-danger-sm {
  padding: 0.3rem 0.75rem;
  font-size: 0.75rem;
  border-radius: 4px;
  border: 1px solid #ef4444;
  background: transparent;
  color: #ef4444;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.btn-danger-sm:hover:not(:disabled) {
  background: #ef4444;
  color: #fff;
}
.btn-danger-sm:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.cache-clear-msg {
  font-size: 0.75rem;
  color: var(--text-muted, #6b7280);
  margin-left: 0.5rem;
}

/* 分析面板 */
.analytics-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-lg);
}

.heatmap-container {
  padding: 1rem;
}

.heatmap-grid {
  min-height: 200px;
}

.heatmap-cell {
  position: relative;
  cursor: pointer;
}

.heatmap-cell[data-tooltip]:hover::after {
  content: attr(data-tooltip);
  position: absolute;
  bottom: calc(100% + 6px);
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.85);
  color: #fff;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 0.72rem;
  white-space: nowrap;
  z-index: 100;
  pointer-events: none;
}

.heatmap-legend {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-top: 0.75rem;
  font-size: 0.75rem;
}

.heatmap-legend-bar {
  width: 100px;
  height: 8px;
  border-radius: 4px;
  background: linear-gradient(to right, rgba(59, 130, 246, 0.1), rgba(59, 130, 246, 0.9));
}

.export-btns {
  display: flex;
  gap: 0.5rem;
}

@media (max-width: 768px) {
  .infra-grid,
  .analytics-grid {
    grid-template-columns: 1fr;
  }
}

/* 设置抽屉 - 完整配置 */
.settings-section {
  margin-bottom: var(--space-xl);
  padding-bottom: var(--space-lg);
  border-bottom: 1px solid var(--hairline-on-dark);
}

.settings-section:last-of-type {
  border-bottom: none;
}

.settings-section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--on-dark);
  margin-bottom: var(--space-md);
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.settings-badge-warn {
  font-size: 11px;
  font-weight: 500;
  color: #f59e0b;
  background: rgba(245, 158, 11, 0.12);
  border: 1px solid rgba(245, 158, 11, 0.3);
  border-radius: var(--radius-sm);
  padding: 2px 8px;
}

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-md);
}

.form-group.compact {
  margin-bottom: var(--space-md);
}

.field-disabled {
  opacity: 0.4;
  pointer-events: none;
}

.form-hint {
  display: block;
  font-size: 0.7rem;
  color: var(--body-muted, #6b7280);
  margin-top: 3px;
}

.input-with-unit {
  display: flex;
  gap: 4px;
}
.input-with-unit .input {
  flex: 1;
  min-width: 0;
}
.unit-select {
  width: 52px;
  padding: 6px 4px;
  border-radius: var(--radius-md);
  border: 1px solid var(--hairline-on-dark);
  background-color: var(--surface-card-dark);
  color: var(--on-dark);
  font-size: 0.82rem;
  cursor: pointer;
}

.endpoint-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.endpoint-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px;
  border-radius: var(--radius-md);
  background: rgba(255, 255, 255, 0.03);
}

.endpoint-item:last-child {
  border-bottom: none;
  margin-bottom: 0;
  padding-bottom: 8px;
}

.endpoint-name-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.endpoint-test-group {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
}

.endpoint-input {
  padding: 6px 10px;
  font-size: 12px;
  height: 32px;
}

.endpoint-name {
  width: 50%;
}

.endpoint-remove-btn {
  width: 24px;
  height: 24px;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  border: 1px solid rgba(246, 70, 93, 0.3);
  background: rgba(246, 70, 93, 0.08);
  color: #f6465d;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.15s;
}

.endpoint-remove-btn:hover {
  background: rgba(246, 70, 93, 0.2);
  border-color: rgba(246, 70, 93, 0.5);
}

.endpoint-latency {
  font-size: 11px;
  font-weight: 600;
  white-space: nowrap;
}

.endpoint-test-btn {
  padding: 4px 10px;
  font-size: 11px;
  flex-shrink: 0;
}

.add-endpoint-btn {
  width: 100%;
  margin-top: var(--space-sm);
}

/* Toggle switch */
.toggle-label {
  display: inline-flex !important;
  align-items: center;
  gap: var(--space-sm);
  cursor: pointer;
  user-select: none;
}

.toggle-input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-switch {
  position: relative;
  width: 36px;
  height: 20px;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid var(--hairline-on-dark);
  border-radius: 999px;
  transition: all 0.2s;
  flex-shrink: 0;
}

.toggle-switch::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  background: var(--body-muted);
  border-radius: 50%;
  transition: all 0.2s;
}

.toggle-input:checked + .toggle-switch {
  background: rgba(14, 203, 129, 0.2);
  border-color: rgba(14, 203, 129, 0.5);
}

.toggle-input:checked + .toggle-switch::after {
  left: 18px;
  background: #0ecb81;
}

@media (max-width: 768px) {
  .settings-drawer {
    width: 100vw;
  }

  .form-row {
    grid-template-columns: 1fr;
  }
}
</style>
