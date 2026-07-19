<script setup lang="ts">
import type { PublicGroup, PublicLink } from '~/types/social'
import { DEFAULT_GROUP_STYLE } from '~/types/social'

const props = defineProps<{ group?: PublicGroup; links: PublicLink[]; title?: string; preview?: boolean }>()
const open = ref(true)
const canCollapse = computed(() => !!props.group?.collapsible)

const style = computed(() => props.group?.style ?? DEFAULT_GROUP_STYLE)
const layout = computed(() => style.value.layout)
const groupVars = computed<Record<string, string>>(() => ({
  '--radius-link': cssRadius(style.value.link_radius, 22),
  '--radius-link-icon': cssRadius(style.value.icon_radius, 50),
  '--group-spacing': style.value.spacing || DEFAULT_GROUP_STYLE.spacing
}))
</script>

<template>
  <section v-if="links.length" class="link-group">
    <button v-if="group || title" class="group-title" type="button" :disabled="!canCollapse" :aria-expanded="open" @click="canCollapse && (open = !open)">
      <span><strong>{{ group?.title || title }}</strong><small v-if="group?.description">{{ group.description }}</small></span>
      <span v-if="canCollapse">{{ open ? '−' : '+' }}</span>
    </button>
    <Transition name="fade">
      <div v-show="open" class="links" :class="`style-${layout}`" :style="groupVars">
        <LinkCard v-for="link in links" :key="link.id" :link="link" :preview="preview" :layout="layout" />
      </div>
    </Transition>
  </section>
</template>

<style scoped>
.link-group { min-width: 0; display: grid; gap: 12px; margin-bottom: var(--layout-spacing); }
.group-title { width: 100%; display: flex; justify-content: space-between; gap: 14px; align-items: center; color: var(--color-text); background: transparent; border: 0; padding: 8px 4px; text-align: left; }
.group-title strong { font: 750 1.05rem var(--font-heading); }
.group-title small { display: block; color: var(--color-text-muted); font-weight: 400; margin-top: 3px; }
.group-title:disabled { cursor: default; }
.links { min-width: 0; display: grid; gap: var(--group-spacing, 12px); }
.links.style-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.fade-enter-active, .fade-leave-active { transition: opacity .18s ease, transform .18s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; transform: translateY(-4px); }
@media (max-width: 560px) { .links.style-grid { grid-template-columns: 1fr; } }
</style>
