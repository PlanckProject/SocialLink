<script setup lang="ts">
import type { PublicProfileResponse } from '~/types/social'

const config = useConfigStore()
await config.loadPublicConfig()
const { data, error } = await useAsyncData('single-profile', () => config.mode === 'single' ? apiFetch<PublicProfileResponse>('/api/profile') : Promise.resolve(null))
useSeoMeta({ title: data.value?.profile.display_name || 'SocialLink', description: data.value?.profile.bio || 'One beautiful link-in-bio page.' })
</script>

<template>
  <PublicProfilePage v-if="config.mode === 'single' && data" :data="data" />
  <main v-else class="container landing">
    <section class="surface hero">
      <p class="muted">SocialLink</p>
      <h1>One polished home for every link you share.</h1>
      <p class="muted">Find your profile by username, or sign in to manage your page.</p>
      <div class="actions">
        <NuxtLink class="btn primary" to="/admin/login">Sign in</NuxtLink>
        <NuxtLink v-if="config.registration_enabled" class="btn" to="/admin/register">Create account</NuxtLink>
      </div>
    </section>
    <p v-if="error" class="error">Unable to load the public profile right now.</p>
  </main>
</template>

<style scoped>
.landing { min-height: 100vh; display: grid; place-items: center; }
.hero { padding: clamp(28px, 8vw, 56px); text-align: center; }
h1 { max-width: 760px; margin: 0 auto 12px; font: 800 clamp(2.6rem, 8vw, 5.5rem)/.92 var(--font-heading); letter-spacing: -.06em; }
.actions { display: flex; justify-content: center; gap: 12px; margin-top: 24px; flex-wrap: wrap; }
</style>
