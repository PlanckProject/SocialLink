<script setup lang="ts">
import type { PublicProfileResponse } from '~/types/social'

definePageMeta({ middleware: 'auth' })

const config = useConfigStore()
await config.loadPublicConfig()
const auth = useAuthStore()
const { data, refresh } = await useAsyncData(
  `admin-public-preview-${auth.user?.username || 'current'}`,
  () => apiFetch<PublicProfileResponse>('/api/admin/preview')
)
const publicPath = computed(() => {
  if (!data.value) return '/'
  return config.mode === 'single' ? '/' : `/${encodeURIComponent(data.value.profile.username)}`
})

useSeoMeta({ title: 'Public page preview' })
</script>

<template>
  <div class="preview-page">
    <div class="preview-controls" aria-label="Preview controls">
      <NuxtLink class="btn" to="/admin">Back to admin</NuxtLink>
      <button class="btn" type="button" @click="refresh()">Refresh</button>
      <a class="btn primary" :href="publicPath" target="_blank" rel="noopener">Open public page</a>
    </div>
    <PublicProfilePage v-if="data" :data="data" preview />
  </div>
</template>

<style scoped>
.preview-page { min-height: 100vh; }
.preview-controls {
  position: fixed;
  top: 14px;
  right: 14px;
  z-index: 100;
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
  padding: 8px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-background);
  background: color-mix(in srgb, var(--color-surface) 82%, transparent);
  box-shadow: var(--button-shadow);
  backdrop-filter: blur(18px);
}

@media (max-width: 760px) {
  .preview-controls {
    inset: auto 10px 10px;
    justify-content: center;
    border-radius: var(--radius-background);
  }
}
</style>
