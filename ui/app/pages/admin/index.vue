<script setup lang="ts">
definePageMeta({ middleware: 'auth' })
interface Overview { totals: { views: number; clicks: number }; top_links: { id: string; title: string; clicks: number }[] }
const { data } = await useAsyncData('admin-overview', () => apiFetch<Overview>('/api/admin/analytics/overview?range=30d').catch(() => null))
</script>

<template>
  <main class="admin-layout">
    <AdminNav />
    <section class="stats">
      <div class="admin-card"><span class="muted">Views</span><strong>{{ data?.totals.views?.toLocaleString() || 0 }}</strong></div>
      <div class="admin-card"><span class="muted">Clicks</span><strong>{{ data?.totals.clicks?.toLocaleString() || 0 }}</strong></div>
      <div class="admin-card"><span class="muted">Top link</span><strong>{{ data?.top_links?.[0]?.title || 'No clicks yet' }}</strong></div>
    </section>
    <section class="admin-card intro">
      <h2>Polish your link-in-bio page</h2>
      <p class="muted">Update profile details, curate grouped links, tune themes, and inspect analytics from one dashboard.</p>
      <div class="actions"><NuxtLink class="btn primary" to="/admin/links">Manage links</NuxtLink><NuxtLink class="btn" to="/admin/appearance">Edit theme</NuxtLink></div>
    </section>
  </main>
</template>

<style scoped>
.stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; margin-bottom: 18px; }
.stats strong { display: block; font: 800 2rem var(--font-heading); margin-top: 8px; }
.intro h2 { font-family: var(--font-heading); margin-top: 0; }
.actions { display: flex; gap: 12px; flex-wrap: wrap; }
@media (max-width: 760px) { .stats { grid-template-columns: 1fr; } }
</style>
