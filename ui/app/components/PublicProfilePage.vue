<script setup lang="ts">
import type { PublicProfileResponse } from '~/types/social'

const props = defineProps<{ data: PublicProfileResponse; preview?: boolean }>()
const config = useConfigStore()

// Render groups with the synthetic "Ungrouped" block spliced in at the
// owner-chosen index (clamped by the backend). When empty it's omitted.
const orderedBlocks = computed(() => {
  const blocks = props.data.groups.map(group => ({ kind: 'group' as const, key: group.id, group }))
  if (props.data.ungrouped?.length) {
    const index = Math.min(props.data.ungrouped_position ?? blocks.length, blocks.length)
    blocks.splice(index, 0, { kind: 'ungrouped' as const, key: 'ungrouped', group: undefined as never })
  }
  return blocks
})
let themeOverrideKey = ''
watchEffect(() => {
  const nextKey = `profile:${props.data.profile.username}`
  if (themeOverrideKey && themeOverrideKey !== nextKey) {
    config.clearThemeOverride(themeOverrideKey)
    config.clearBrandingOverride(themeOverrideKey)
  }
  themeOverrideKey = nextKey
  config.setThemeOverride(themeOverrideKey, props.data.theme)
  if (props.data.branding) config.setBrandingOverride(themeOverrideKey, props.data.branding)
})
onUnmounted(() => {
  config.clearThemeOverride(themeOverrideKey)
  config.clearBrandingOverride(themeOverrideKey)
})
useProfileSeo(computed(() => props.data))
</script>

<template>
  <main class="profile-main">
    <CoverImage :src="data.profile.cover_url" :alt="`${data.profile.display_name} cover photo`" />
    <div class="container profile-shell">
      <ProfileHeader :profile="data.profile" :views="data.stats?.views" />
      <div class="link-stack">
        <template v-for="block in orderedBlocks" :key="block.key">
          <LinkGroup v-if="block.kind === 'group'" :group="block.group" :links="block.group.links" :preview="preview" />
          <LinkGroup v-else :links="data.ungrouped" :style-override="config.theme.ungrouped" :preview="preview" />
        </template>
      </div>
      <!-- footer_text is owner-controlled branding rendered as HTML so it can
           contain a themed link (e.g. the default "Made with SocialLink"). -->
      <footer v-if="config.branding.show_footer" class="footer" v-html="config.branding.footer_text"></footer>
    </div>
  </main>
</template>

<style scoped>
.profile-main { display: flex; flex-direction: column; min-height: 100vh; }
.profile-shell { position: relative; z-index: 1; display: flex; flex-direction: column; flex: 1 1 auto; padding-bottom: 48px; }
.link-stack { display: grid; gap: 8px; }
.footer { margin-top: auto; padding: 28px 0 10px; color: var(--color-text-muted); text-align: center; font-size: .92rem; }
.footer :deep(a) { color: var(--color-accent); text-decoration: none; font-weight: 600; transition: opacity .15s ease; }
.footer :deep(a:hover) { text-decoration: underline; opacity: .85; }
</style>
