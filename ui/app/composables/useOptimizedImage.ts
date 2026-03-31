import type { MaybeRefOrGetter } from 'vue'
import { optimizeImage, type OptimizeOptions, type OptimizedImage } from '~/utils/images'

/**
 * Reactive wrapper around {@link optimizeImage} that also honours the
 * `NUXT_PUBLIC_IMAGE_OPTIMIZE` runtime flag. When optimization is turned off
 * (or the source isn't a local upload) the original URL is returned unchanged,
 * so images always render even if the optimizer route is unavailable.
 */
export function useOptimizedImage(
  src: MaybeRefOrGetter<string | null | undefined>,
  opts: OptimizeOptions
) {
  const config = useRuntimeConfig()
  return computed<OptimizedImage>(() => {
    const value = toValue(src)
    if (config.public.imageOptimize === false) return { src: (value || '').trim() }
    return optimizeImage(value, opts)
  })
}
