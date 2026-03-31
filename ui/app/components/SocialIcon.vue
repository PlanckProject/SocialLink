<script setup lang="ts">
const props = defineProps<{ platform?: string | null; url?: string | null; title?: string; decorative?: boolean }>()
const resolved = computed(() => detectPlatform(props.platform, props.url))
const iconClass = computed(() => socialIconClass(resolved.value.slug))
const label = computed(() => props.title || resolved.value.label)
</script>

<template>
  <i
    class="social-icon"
    :class="iconClass"
    :role="decorative ? undefined : 'img'"
    :aria-hidden="decorative ? 'true' : undefined"
    :aria-label="decorative ? undefined : label"
  ></i>
</template>

<style scoped>
.social-icon { display: inline-grid; place-items: center; font-size: 1.15rem; line-height: 1; }
</style>
