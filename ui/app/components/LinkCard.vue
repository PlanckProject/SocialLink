<script setup lang="ts">
import type { PublicLink } from '~/types/social'

const props = defineProps<{ link: PublicLink; preview?: boolean; layout?: 'list' | 'grid' }>()
const config = useConfigStore()
const expired = computed(() => props.link.expires_at ? new Date(props.link.expires_at).getTime() < Date.now() : false)
const href = computed(() => props.preview ? `/api/admin/links/${props.link.id}/preview` : `/api/l/${props.link.id}`)
const iconImg = useOptimizedImage(() => props.link.icon_image, { width: 38 })
</script>

<template>
  <a
    v-if="!expired"
    class="link-card"
    :class="[
      `variant-${config.theme.button.variant}`,
      {
        lift: config.theme.button.hover_lift,
        grid: layout === 'grid'
      }
    ]"
    :href="href"
    target="_blank"
    rel="noopener"
  >
    <span v-if="link.icon_image" class="icon img"><img :src="iconImg.src" :srcset="iconImg.srcset" :alt="`${link.title} icon`" loading="lazy" decoding="async" width="38" height="38"></span>
    <span v-else-if="link.icon" class="icon">{{ link.icon }}</span>
    <span v-else-if="link.icon_font" class="icon"><i :class="link.icon_font" aria-hidden="true"></i></span>
    <span class="content"><strong>{{ link.title }}</strong><small v-if="link.description">{{ link.description }}</small></span>
    <span v-if="config.theme.features.show_click_count" class="clicks">{{ link.click_count }}</span>
  </a>
</template>

<style scoped>
.link-card { width: 100%; min-width: 0; min-height: 54px; display: grid; grid-template-columns: auto minmax(0, 1fr) auto; grid-template-areas: 'icon content clicks'; gap: 14px; align-items: start; padding: 15px 17px; border-radius: var(--radius-link); border: 1px solid var(--color-border); background: var(--color-surface); box-shadow: var(--button-shadow); transition: transform .2s ease, border-color .2s ease, background .2s ease; }
.link-card.grid { grid-template-columns: auto minmax(0, 1fr); grid-template-areas: 'icon content' 'icon clicks'; gap: 5px 10px; align-items: start; padding: 13px; }
.link-card.lift:hover { transform: translateY(-3px); }
.link-card:hover { border-color: color-mix(in srgb, var(--color-primary) 60%, var(--color-border)); }
.variant-glass { background: color-mix(in srgb, var(--color-surface) 72%, transparent); backdrop-filter: blur(18px); }
.variant-solid { background: var(--color-primary); color: var(--color-primary-contrast); }
.variant-outline { background: transparent; border-color: var(--color-primary); }
.variant-soft { background: color-mix(in srgb, var(--color-primary) 16%, var(--color-surface)); }
.icon { grid-area: icon; display: grid; place-items: center; width: 38px; height: 38px; border-radius: var(--radius-link-icon); background: color-mix(in srgb, var(--color-accent) 14%, transparent); }
.icon.img { background: none; padding: 0; overflow: hidden; }
.icon.img img { width: 100%; height: 100%; object-fit: cover; border-radius: inherit; }
.icon i { font-size: 18px; line-height: 1; }
.content { grid-area: content; display: grid; gap: 2px; min-width: 0; max-width: 100%; overflow: hidden; }
.content strong, .content small { display: block; min-width: 0; max-width: 100%; overflow-wrap: anywhere; word-break: break-word; }
.content strong { font-family: var(--font-heading); }
.content small, .clicks { color: var(--color-text-muted); }
.clicks { grid-area: clicks; min-width: 0; justify-self: end; overflow-wrap: anywhere; }
.link-card.grid .clicks { justify-self: start; }
</style>
