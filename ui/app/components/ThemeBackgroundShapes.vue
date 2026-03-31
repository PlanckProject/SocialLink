<script setup lang="ts">
const props = withDefaults(defineProps<{
  contained?: boolean
}>(), {
  contained: false
})

const shapeIndexes = Array.from({ length: 12 }, (_, index) => index + 1)
const shapeStyle = (index: number) => {
  const prefix = `--background-shape-${index}`
  return {
    display: `var(${prefix}-display, none)`,
    left: `var(${prefix}-left, 50%)`,
    top: `var(${prefix}-top, 50%)`,
    width: `var(${prefix}-width, 0px)`,
    height: `var(${prefix}-height, 0px)`,
    borderRadius: `var(${prefix}-radius, 50%)`,
    background: `var(${prefix}-color, transparent)`,
    opacity: 'var(--background-shape-opacity, 0)',
    filter: 'blur(var(--background-shape-blur, 90px))',
    transform: `translate3d(-50%, -50%, 0) rotate(var(${prefix}-rotation, 0deg)) scale(var(--background-shape-scale, 1))`
  }
}
</script>

<template>
  <div class="theme-background-shapes" :class="{ contained }" aria-hidden="true">
    <span v-for="index in shapeIndexes" :key="index" class="theme-background-shape" :style="shapeStyle(index)" />
  </div>
</template>

<style scoped>
.theme-background-shapes {
  position: fixed;
  inset: 0;
  z-index: 0;
  overflow: hidden;
  pointer-events: none;
  --background-shape-scale: 1;
}
.theme-background-shapes.contained {
  position: absolute;
  --background-shape-scale: .42;
}
.theme-background-shape {
  position: absolute;
  display: block;
  transform-origin: center;
}
</style>
