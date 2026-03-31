<script setup lang="ts">
import type { PublicProfile } from '~/types/social'

const props = defineProps<{ profile: PublicProfile; views?: number }>()
const config = useConfigStore()
const avatar = useOptimizedImage(() => props.profile.avatar_url, { width: 112 })
</script>

<template>
  <header class="profile-header" :class="`align-${config.theme.layout.align}`">
    <div class="avatar-frame">
      <img v-if="profile.avatar_url" :src="avatar.src" :srcset="avatar.srcset" :alt="`${profile.display_name} profile photo`" class="avatar" width="112" height="112" loading="eager" fetchpriority="high" decoding="async">
      <div v-else class="avatar placeholder" aria-hidden="true">{{ profile.display_name?.slice(0, 1) || '?' }}</div>
    </div>
    <p v-if="profile.location" class="eyebrow location">
      <svg class="location-icon" viewBox="0 0 24 24" width="14" height="14" aria-hidden="true" focusable="false">
        <path fill="currentColor" d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5a2.5 2.5 0 1 1 0-5 2.5 2.5 0 0 1 0 5z"/>
      </svg>
      <span>{{ profile.location }}</span>
    </p>
    <h1>{{ profile.display_name }}</h1>
    <p v-if="profile.username && config.mode !== 'single'" class="handle">@{{ profile.username }}</p>
    <p v-if="profile.bio" class="bio">{{ profile.bio }}</p>
    <nav v-if="profile.socials?.length" class="socials" aria-label="Social profiles">
      <a
        v-for="social in profile.socials"
        :key="`${social.platform}-${social.url}`"
        class="social-btn"
        :href="socialHref(social.platform, social.url)"
        :aria-label="socialLabel(social.platform, social.url)"
        :title="socialLabel(social.platform, social.url)"
        target="_blank"
        rel="me noopener noreferrer"
      >
        <SocialIcon :platform="social.platform" :url="social.url" decorative />
      </a>
    </nav>
    <p v-if="config.theme.features.show_view_count && typeof views === 'number'" class="views">{{ views.toLocaleString() }} views</p>
  </header>
</template>

<style scoped>
.profile-header { display: grid; justify-items: center; text-align: center; gap: 10px; padding: calc(var(--cover-height) - 92px) 0 28px; position: relative; z-index: 1; }
.profile-header.align-left { justify-items: start; text-align: left; }
.profile-header.align-right { justify-items: end; text-align: right; }
.avatar-frame { padding: 5px; border-radius: var(--radius-avatar); background: linear-gradient(135deg, var(--color-accent), var(--color-primary)); box-shadow: var(--button-shadow); }
.avatar { width: var(--avatar-size); height: var(--avatar-size); object-fit: cover; border-radius: var(--radius-avatar); border: 3px solid var(--color-background); }
.placeholder { display: grid; place-items: center; font: 700 2.6rem/1 var(--font-heading); color: var(--color-primary-contrast); background: var(--color-primary); }
h1 { margin: 6px 0 0; font: 800 clamp(2rem, 7vw, 4rem)/.95 var(--font-heading); letter-spacing: -0.055em; overflow-wrap: anywhere; }
.eyebrow, .views { color: var(--color-text-muted); font-size: .92rem; }
.location { display: inline-flex; align-items: center; gap: 6px; }
.location-icon { flex: none; opacity: .85; }
.handle { margin: 0; color: var(--color-text-muted); font-weight: 600; overflow-wrap: anywhere; }
.bio { max-width: 56ch; margin: 0; color: var(--color-text-muted); overflow-wrap: anywhere; }
.socials { display: flex; flex-wrap: wrap; gap: 10px; justify-content: inherit; margin-top: 4px; }
.social-btn { display: grid; place-items: center; width: 44px; height: 44px; border-radius: var(--radius-social-icon); border: 1px solid var(--color-border); color: var(--color-text); background: color-mix(in srgb, var(--color-surface) 78%, transparent); backdrop-filter: blur(14px); transition: transform .18s ease, color .18s ease, background .18s ease, border-color .18s ease; }
.social-btn:hover { transform: translateY(-2px); color: var(--color-primary-contrast); background: var(--color-primary); border-color: transparent; }
</style>
