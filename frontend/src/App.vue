<template>
  <div id="app">
    <main class="main">
      <Dashboard />
    </main>
  </div>
</template>

<script setup lang="ts">
import Dashboard from '@/pages/Dashboard.vue'
</script>

<style scoped>
.main {
  flex: 1;
  padding: var(--space-lg) 0;
}
</style>
