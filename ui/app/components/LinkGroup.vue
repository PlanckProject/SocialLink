<script setup lang="ts">
import type { GroupStyle, PublicGroup, PublicLink } from '~/types/social'
import { DEFAULT_GROUP_STYLE } from '~/types/social'

const props = defineProps<{ group?: PublicGroup; links: PublicLink[]; title?: string; styleOverride?: GroupStyle; preview?: boolean }>()
const open = ref(true)
const canCollapse = computed(() => !!props.group?.collapsible)

// The collapse box (.links-inner) must clip its content while the open/close
// animation runs, otherwise the grid-rows transition can't hide it. But once a
// group is fully expanded we switch back to `overflow: visible` so the card
// shadows and the hover-lift aren't sliced off into a hard rectangle.
// `expanded` tracks that settled-open state.
const expanded = ref(true)
let revealTimer: ReturnType<typeof setTimeout> | null = null
watch(open, (isOpen) => {
  if (revealTimer) { clearTimeout(revealTimer); revealTimer = null }
  if (!isOpen) { expanded.value = false; return } // collapsing: clip immediately
  if (import.meta.client && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) {
    expanded.value = true; return // no animation to wait for
  }
  revealTimer = setTimeout(() => { expanded.value = true }, 340) // just after the .3s expand
})
onBeforeUnmount(() => { if (revealTimer) clearTimeout(revealTimer) })

const style = computed(() => props.group?.style ?? props.styleOverride ?? DEFAULT_GROUP_STYLE)
const layout = computed(() => style.value.layout)
const titleAlign = computed(() => style.value.title_align === 'center' ? 'center' : 'left')
const groupVars = computed<Record<string, string>>(() => ({
  '--radius-link': cssRadius(style.value.link_radius, 50),
  '--radius-link-icon': cssRadius(style.value.icon_radius, 50),
  '--group-spacing': style.value.spacing || DEFAULT_GROUP_STYLE.spacing
}))
</script>

<template>
  <section v-if="links.length" class="link-group">
    <button
      v-if="group || title"
      class="group-title"
      :class="[`align-${titleAlign}`, { collapsible: canCollapse }]"
      type="button"
      :disabled="!canCollapse"
      :aria-expanded="open"
      @click="canCollapse && (open = !open)"
    >
      <span class="group-title-text"><strong>{{ group?.title || title }}</strong><small v-if="group?.description">{{ group.description }}</small></span>
      <span v-if="canCollapse" class="chevron" :class="{ open }" aria-hidden="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"></polyline></svg>
      </span>
    </button>
    <div class="links-wrap" :class="{ open: open || !canCollapse }">
      <div class="links-inner" :class="{ expanded }">
        <div class="links" :class="`style-${layout}`" :style="groupVars">
          <LinkCard v-for="link in links" :key="link.id" :link="link" :preview="preview" :layout="layout" />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.link-group { min-width: 0; display: grid; gap: 12px; margin-bottom: var(--layout-spacing, 12px); }
.group-title { width: 100%; position: relative; display: flex; gap: 14px; align-items: center; color: var(--color-text); background: transparent; border: 0; padding: 8px 4px; }
.group-title.align-left { justify-content: space-between; text-align: left; }
.group-title.align-center { justify-content: center; text-align: center; }
.group-title-text { min-width: 0; }
.group-title strong { font: 750 1.05rem var(--font-heading); }
.group-title small { display: block; color: var(--color-text-muted); font-weight: 400; margin-top: 3px; }
.group-title:disabled { cursor: default; }
.group-title.collapsible { cursor: pointer; }
.chevron { display: inline-flex; align-items: center; justify-content: center; width: 26px; height: 26px; flex: none; border-radius: 999px; color: var(--color-text-muted); transition: transform .28s ease, color .18s ease, background .18s ease; }
.chevron svg { width: 18px; height: 18px; }
.chevron.open { transform: rotate(180deg); }
.group-title.collapsible:hover .chevron { color: var(--color-text); background: color-mix(in srgb, var(--color-primary) 14%, transparent); }
.group-title.align-center .chevron { position: absolute; right: 4px; top: 50%; transform: translateY(-50%); }
.group-title.align-center .chevron.open { transform: translateY(-50%) rotate(180deg); }
.links-wrap { display: grid; grid-template-rows: 0fr; transition: grid-template-rows .3s ease; }
.links-wrap.open { grid-template-rows: 1fr; }
.links-inner { min-height: 0; overflow: hidden; }
.links-inner.expanded { overflow: visible; }
.links { min-width: 0; display: grid; gap: var(--group-spacing, 12px); padding: 1px; }
.links.style-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
@media (max-width: 560px) { .links.style-grid { grid-template-columns: 1fr; } }
@media (prefers-reduced-motion: reduce) { .links-wrap { transition: none; } .chevron { transition: none; } }
</style>
