<script setup lang="ts">
import type { PublicProfileResponse } from '~/types/social'

const route = useRoute()
const config = useConfigStore()
await config.loadPublicConfig()
const username = computed(() => String(route.params.username || ''))
const { data, error } = await useAsyncData(`profile-${username.value}`, () => apiFetch<PublicProfileResponse>(`/api/u/${encodeURIComponent(username.value)}`))
if (config.mode === 'single') await navigateTo('/')
useSeoMeta({ title: data.value?.profile.display_name || username.value, description: data.value?.profile.bio || 'SocialLink profile' })
</script>

<template>
  <PublicProfilePage v-if="data" :data="data" />
  <main v-else class="container empty"><p class="error">{{ error ? 'Profile not found.' : 'Loading profile…' }}</p></main>
</template>

<style scoped>
.empty { min-height: 70vh; display: grid; place-items: center; }
</style>
