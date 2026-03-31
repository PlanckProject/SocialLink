<script setup lang="ts">
const configStore = useConfigStore()
await configStore.loadPublicConfig()
useTheme()

if (import.meta.client) {
  const bus = useConfigBus()
  bus.on(() => refreshNuxtData())
}

const route = useRoute()
useHead(() => ({
  htmlAttrs: { lang: 'en' },
  meta: route.path.startsWith('/admin')
    ? [{ name: 'robots', content: 'noindex, nofollow' }]
    : []
}))
</script>

<template>
  <div class="app-shell">
    <div class="app-content">
      <NuxtRouteAnnouncer />
      <NuxtPage />
    </div>
    <ThemeBackgroundShapes />
  </div>
</template>
