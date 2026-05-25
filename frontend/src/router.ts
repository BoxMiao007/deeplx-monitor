import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'overview',
      component: () => import('@/pages/Dashboard.vue'),
    },
    {
      path: '/endpoints',
      name: 'endpoints',
      component: () => import('@/pages/EndpointComparison.vue'),
    },
    {
      path: '/endpoints/:name',
      name: 'endpoint-detail',
      component: () => import('@/pages/EndpointDetail.vue'),
    },
  ],
})

export default router
