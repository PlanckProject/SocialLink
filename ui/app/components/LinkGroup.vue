<script setup lang="ts">
import type { PublicGroup, PublicLink } from '~/types/social'
import { DEFAULT_GROUP_STYLE } from '~/types/social'

const props = defineProps<{ group?: PublicGroup; links: PublicLink[]; title?: string; preview?: boolean }>()
const config = useConfigStore()
const open = ref(true)
const canCollapse = computed(() => !!props.group?.collapsible && config.theme.features.collapsible_groups)

const style = computed(() => props.group?.style ?? DEFAULT_GROUP_STYLE)
const layout = computed(() => style.value.layout)
const groupVars = computed(() => {
  const vars: Record<string, string> = {
    '--radius-link-icon': style.value.icon === 'square' ? '20%' : '50%',
  }
  if (style.value.corners === 'sharp') vars['--radius-link'] = '0px'
  return vars
})
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
.links { min-width: 0; display: grid; gap: 12px; }
.links.style-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.fade-enter-active, .fade-leave-active { transition: opacity .18s ease, transform .18s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; transform: translateY(-4px); }
@media (max-width: 560px) { .links.style-grid { grid-template-columns: 1fr; } }
</style>
