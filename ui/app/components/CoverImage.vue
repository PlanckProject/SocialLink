<script setup lang="ts">
const props = defineProps<{ src?: string | null; alt?: string }>()
const config = useConfigStore()
const { style } = useScrollFade(computed(() => config.theme))
const cover = useOptimizedImage(() => props.src, { width: 640 })
</script>

<template>
  <div v-if="config.theme.features.show_cover_photo" class="cover-wrap" aria-hidden="true">
    <div class="cover" :style="style">
      <img v-if="props.src" :src="cover.src" :srcset="cover.srcset" :alt="props.alt || ''" class="cover-img" loading="eager" decoding="async" fetchpriority="high">
      <div v-else class="cover-fallback" />
      <div class="cover-overlay" />
    </div>
  </div>
</template>

<style scoped>
.cover-wrap {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  width: min(100%, var(--layout-max-width));
  margin-inline: auto;
  height: var(--cover-height);
  overflow: hidden;
  pointer-events: none;
  z-index: 0;
  -webkit-mask-image: linear-gradient(to bottom, #000 0%, #000 52%, rgba(0,0,0,.92) 62%, transparent 100%);
  mask-image: linear-gradient(to bottom, #000 0%, #000 52%, rgba(0,0,0,.92) 62%, transparent 100%);
}
.cover { height: 100%; transform-origin: center top; will-change: opacity, transform; }
.cover-img, .cover-fallback { width: 100%; height: 100%; object-fit: cover; }
.cover-fallback { background: var(--background-gradient); }
.cover-overlay {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    to bottom,
    var(--background-overlay) 0%,
    color-mix(in srgb, var(--background-overlay) 62%, transparent) 48%,
    transparent 100%
  );
}
</style>
