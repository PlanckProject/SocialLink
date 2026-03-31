<script setup lang="ts">
import { CategoryScale, Chart as ChartJS, Legend, LinearScale, LineElement, PointElement, Title, Tooltip } from 'chart.js'
import { Line } from 'vue-chartjs'

ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Title, Tooltip, Legend)

const props = defineProps<{ labels: string[]; views?: number[]; clicks: number[] }>()
const data = computed(() => ({
  labels: props.labels,
  datasets: [
    ...(props.views ? [{ label: 'Views', data: props.views, borderColor: '#22d3ee', backgroundColor: 'rgba(34,211,238,.18)', tension: .35 }] : []),
    { label: 'Clicks', data: props.clicks, borderColor: '#7c3aed', backgroundColor: 'rgba(124,58,237,.18)', tension: .35 }
  ]
}))
const options = { responsive: true, maintainAspectRatio: false, plugins: { legend: { labels: { color: '#a1a1aa' } } }, scales: { x: { ticks: { color: '#a1a1aa' }, grid: { color: 'rgba(255,255,255,.08)' } }, y: { ticks: { color: '#a1a1aa' }, grid: { color: 'rgba(255,255,255,.08)' } } } }
</script>

<template><Line :data="data" :options="options" /></template>
