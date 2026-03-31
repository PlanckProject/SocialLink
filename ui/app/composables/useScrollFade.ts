import type { ComputedRef, Ref } from 'vue'
import { usePreferredReducedMotion, useWindowScroll } from '@vueuse/core'
import type { Theme } from '~/app.config'

export function useScrollFade(theme: Ref<Theme> | ComputedRef<Theme>) {
  const { y } = useWindowScroll()
  const reducedMotion = usePreferredReducedMotion()
  const progress = computed(() => Math.min(Number(y.value) / 260, 1))
  const style = computed(() => {
    const disabled = reducedMotion.value === 'reduce'
    const opacity = theme.value.effects.cover_fade && !disabled ? Math.max(0.18, 1 - progress.value * 0.82) : 1
    const translate = theme.value.effects.cover_parallax && !disabled ? progress.value * -34 : 0
    const scale = theme.value.effects.cover_parallax && !disabled ? 1 + progress.value * 0.045 : 1
    return { opacity, transform: `translate3d(0, ${translate}px, 0) scale(${scale})` }
  })
  return { progress, style }
}
