<script setup lang="ts">
import type { PublicProfileResponse } from '~/types/social'

const props = defineProps<{ data: PublicProfileResponse; preview?: boolean }>()
const config = useConfigStore()
let themeOverrideKey = ''
watchEffect(() => {
  const nextKey = `profile:${props.data.profile.username}`
  if (themeOverrideKey && themeOverrideKey !== nextKey) config.clearThemeOverride(themeOverrideKey)
  themeOverrideKey = nextKey
  config.setThemeOverride(themeOverrideKey, props.data.theme)
})
onUnmounted(() => config.clearThemeOverride(themeOverrideKey))
useProfileSeo(computed(() => props.data))
</script>

<template>
  <main>
    <CoverImage :src="data.profile.cover_url" :alt="`${data.profile.display_name} cover photo`" />
    <div class="container profile-shell">
      <ProfileHeader :profile="data.profile" :views="data.stats?.views" />
      <div class="link-stack">
        <LinkGroup v-for="group in data.groups" :key="group.id" :group="group" :links="group.links" :preview="preview" />
        <LinkGroup v-if="data.ungrouped?.length" :links="data.ungrouped" :preview="preview" />
      </div>
      <!-- footer_text is owner-controlled branding rendered as HTML so it can
           contain a themed link (e.g. the default "Made with SocialLink"). -->
      <footer v-if="config.theme.branding.show_footer" class="footer" v-html="config.theme.branding.footer_text"></footer>
    </div>
  </main>
</template>

<style scoped>
.profile-shell { position: relative; z-index: 1; padding-bottom: 48px; }
.link-stack { display: grid; gap: 8px; }
.footer { padding: 28px 0 10px; color: var(--color-text-muted); text-align: center; font-size: .92rem; }
.footer :deep(a) { color: var(--color-accent); text-decoration: none; font-weight: 600; transition: opacity .15s ease; }
.footer :deep(a:hover) { text-decoration: underline; opacity: .85; }
</style>
