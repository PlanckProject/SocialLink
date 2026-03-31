<script setup lang="ts">
import DailyLineChart from '~/components/charts/DailyLineChart.vue'
definePageMeta({ middleware: 'auth' })
interface Overview { totals: { views: number; clicks: number }; series: { date: string; views: number; clicks: number }[]; top_links: { id: string; title: string; clicks: number }[] }
interface LinkAnalytics { link_id: string; title: string; clicks: number; series: { date: string; clicks: number }[] }

const LINKS_PER_PAGE = 8
const ranges = [
  { value: '7d', label: '7d' },
  { value: '30d', label: '30d' },
  { value: '90d', label: '90d' },
  { value: 'custom', label: 'Custom' },
]
const range = ref('30d')
const customStart = ref('')
const customEnd = ref('')

const analyticsQuery = computed(() => {
  if (range.value !== 'custom') return `range=${range.value}`
  if (!customStart.value || !customEnd.value || customStart.value > customEnd.value) return 'range=30d'
  return `range=custom&start=${customStart.value}&end=${customEnd.value}`
})

function selectRange(value: string) {
  if (value === 'custom' && (!customStart.value || !customEnd.value)) {
    const end = new Date()
    const start = new Date()
    start.setDate(end.getDate() - 29)
    customEnd.value = end.toISOString().slice(0, 10)
    customStart.value = start.toISOString().slice(0, 10)
  }
  range.value = value
}

const { data: overview } = await useAsyncData('analytics-overview', () => apiFetch<Overview>(`/api/admin/analytics/overview?${analyticsQuery.value}`), { watch: [analyticsQuery] })
const { data: links } = await useAsyncData('analytics-links', () => apiFetch<LinkAnalytics[]>(`/api/admin/analytics/links?${analyticsQuery.value}`), { watch: [analyticsQuery] })
const labels = computed(() => overview.value?.series.map(point => point.date) || [])

const topLinks = computed(() => (links.value || []).slice(0, LINKS_PER_PAGE))
const linksPage = ref(1)
const linkPageCount = computed(() => Math.max(1, Math.ceil((links.value?.length || 0) / LINKS_PER_PAGE)))
const pagedLinks = computed(() => {
  const start = (linksPage.value - 1) * LINKS_PER_PAGE
  return (links.value || []).slice(start, start + LINKS_PER_PAGE)
})
watch(analyticsQuery, () => { linksPage.value = 1 })
watch(linkPageCount, count => { if (linksPage.value > count) linksPage.value = count })
</script>

<template>
  <main class="admin-layout">
    <AdminNav />
    <section class="admin-card controls">
      <h2>Analytics</h2>
      <div class="range-controls">
        <div class="segmented" role="group" aria-label="Date range">
          <button v-for="option in ranges" :key="option.value" type="button" class="seg-btn" :class="{ active: range === option.value }" @click="selectRange(option.value)">{{ option.label }}</button>
        </div>
        <div v-if="range === 'custom'" class="custom-range">
          <input v-model="customStart" type="date" :max="customEnd || undefined" aria-label="Start date">
          <span class="muted">to</span>
          <input v-model="customEnd" type="date" :min="customStart || undefined" aria-label="End date">
        </div>
      </div>
    </section>
    <section class="stats">
      <div class="admin-card"><span class="muted">Views</span><strong>{{ overview?.totals.views?.toLocaleString() || 0 }}</strong></div>
      <div class="admin-card"><span class="muted">Clicks</span><strong>{{ overview?.totals.clicks?.toLocaleString() || 0 }}</strong></div>
    </section>
    <section class="admin-card chart-card"><ClientOnly><DailyLineChart :labels="labels" :views="overview?.series.map(p => p.views) || []" :clicks="overview?.series.map(p => p.clicks) || []" /></ClientOnly></section>
    <section class="two-col">
      <div class="admin-card"><h3>Top links</h3><ol><li v-for="link in topLinks" :key="link.link_id"><span>{{ link.title }}</span><strong>{{ link.clicks }}</strong></li></ol></div>
      <div class="admin-card links-card">
        <h3>Per-link clicks</h3>
        <ol v-if="pagedLinks.length"><li v-for="link in pagedLinks" :key="link.link_id"><span>{{ link.title }}</span><strong>{{ link.clicks }}</strong></li></ol>
        <p v-else class="muted">No link activity in this range.</p>
        <div v-if="linkPageCount > 1" class="pager">
          <button type="button" class="btn ghost" :disabled="linksPage === 1" @click="linksPage--">Prev</button>
          <span class="muted">Page {{ linksPage }} of {{ linkPageCount }}</span>
          <button type="button" class="btn ghost" :disabled="linksPage === linkPageCount" @click="linksPage++">Next</button>
        </div>
      </div>
    </section>
  </main>
</template>

<style scoped>
.controls { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 16px; margin-bottom: 18px; }
.range-controls { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.segmented { display: inline-flex; gap: 4px; padding: 4px; background: var(--color-surface-alt); border: 1px solid var(--color-border); border-radius: var(--radius-button); }
.seg-btn { min-height: 36px; padding: 6px 16px; border: 0; border-radius: 999px; background: transparent; color: var(--color-text); font-weight: 600; font-size: .9rem; cursor: pointer; transition: background .18s ease, color .18s ease; }
.seg-btn:hover { background: color-mix(in srgb, var(--color-primary) 18%, transparent); }
.seg-btn.active { background: var(--color-primary); color: var(--color-primary-contrast); }
.custom-range { display: inline-flex; align-items: center; gap: 8px; }
.custom-range input { width: auto; min-height: 36px; padding: 6px 10px; }
.stats, .two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 18px; margin-bottom: 18px; }
.stats strong { display: block; font: 800 2rem var(--font-heading); margin-top: 8px; }
.chart-card { height: 380px; margin-bottom: 18px; }
ol { list-style: none; padding: 0; margin: 0; } li { margin: 10px 0; display: flex; justify-content: space-between; gap: 12px; }
h2, h3 { margin-top: 0; font-family: var(--font-heading); }
.links-card { display: flex; flex-direction: column; }
.pager { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: auto; padding-top: 14px; }
.pager .btn { min-height: 36px; padding: 6px 14px; }
.pager .btn:disabled { opacity: .45; cursor: not-allowed; transform: none; }
@media (max-width: 760px) { .stats, .two-col, .controls { grid-template-columns: 1fr; display: grid; } }
</style>
