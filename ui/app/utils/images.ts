// Helpers for requesting resized / re-encoded variants of user-uploaded images
// from the /img optimizer route (see server/routes/img/[...path].ts).
//
// Only our own /uploads/<key> images are rewritten. External avatar/cover URLs
// (e.g. https://cdn.example.com/x.png), data URIs and animated GIFs are passed
// through untouched. WebP is emitted directly (no <picture> fallback) because
// the app's baseline CSS already requires browsers newer than WebP support.

// Fixed resolution ladder so the number of generated variants — and therefore
// the number of things the CDN has to cache and the origin has to encode —
// stays small. Requested widths snap up to the next rung. Keep in sync with the
// ladder in server/routes/img/[...path].ts.
const WIDTH_LADDER = [48, 96, 128, 192, 256, 384, 512, 768, 1024, 1440, 1920]
const MAX_WIDTH = WIDTH_LADDER[WIDTH_LADDER.length - 1]!

// Raster uploads we can safely transform. GIF is excluded (may be animated) and
// SVG is rejected at upload time by the API, so it never appears here.
const OPTIMIZABLE = /\.(png|jpe?g|webp|avif)$/i

export interface OptimizedImage {
  /** Value for the <img src> attribute. */
  src: string
  /** 1x/2x WebP candidates for the <img srcset>, or undefined when not rewritten. */
  srcset?: string
}

export function snapWidth(width: number): number {
  const n = Math.round(Number(width))
  if (!Number.isFinite(n) || n <= 0) return WIDTH_LADDER[0]!
  for (const step of WIDTH_LADDER) if (n <= step) return step
  return MAX_WIDTH
}

/** Returns the flat upload key for a local /uploads image, or null otherwise. */
function uploadKey(src: string): string | null {
  if (!src.startsWith('/uploads/')) return null
  const key = src.slice('/uploads/'.length)
  if (!key || key.includes('/') || key.includes('\\') || key.includes('..')) return null
  if (!OPTIMIZABLE.test(key)) return null
  return key
}

export interface OptimizeOptions {
  /** Intended CSS render width in px; used to pick the 1x resolution. */
  width: number
  /** Encoder quality 30-90 (default 72). */
  quality?: number
}

export function optimizeImage(src: string | null | undefined, opts: OptimizeOptions): OptimizedImage {
  const raw = (src || '').trim()
  const key = raw ? uploadKey(raw) : null
  if (!raw || !key) return { src: raw }

  const quality = Math.min(90, Math.max(30, Math.round(opts.quality ?? 72)))
  const w1 = snapWidth(opts.width)
  const w2 = snapWidth(opts.width * 2)
  const url = (w: number) => `/img/${key}?w=${w}&f=webp&q=${quality}`

  return w2 > w1
    ? { src: url(w1), srcset: `${url(w1)} 1x, ${url(w2)} 2x` }
    : { src: url(w1), srcset: `${url(w1)} 1x` }
}
