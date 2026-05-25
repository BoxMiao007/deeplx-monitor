<template>
  <nav class="tab-nav">
    <router-link to="/" :class="['tab-link', { active: route.path === '/' }]">概述</router-link>
    <router-link to="/endpoints" :class="['tab-link', { active: route.path.startsWith('/endpoints') }]">端点详情</router-link>
  </nav>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
const route = useRoute()
</script>

<style scoped>
.tab-nav {
  display: flex;
  gap: 4px;
  padding: 6px 8px;
  background: rgba(15, 23, 42, 0.6);
  border: 1px solid rgba(148, 163, 184, 0.08);
  border-radius: var(--radius-md);
  width: fit-content;
}

.tab-link {
  padding: 6px 16px;
  font-size: 13px;
  font-weight: 500;
  color: #94a3b8;
  text-decoration: none;
  border-radius: var(--radius-sm);
  transition: all 0.15s ease;
}

.tab-link:hover {
  color: #e2e8f0;
  background: rgba(255, 255, 255, 0.04);
}

.tab-link.active {
  color: #f8fafc;
  background: rgba(96, 165, 250, 0.15);
  border: 1px solid rgba(96, 165, 250, 0.3);
}
</style>
