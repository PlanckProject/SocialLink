// On-the-fly image optimizer for user-uploaded media.
//
// Avatars and cover photos are stored by the API at their original resolution
// (only link icons are pre-processed upstream), so a 3000px avatar would other-
// wise be shipped for a 112px slot. This route fetches the original from the
// API (API_INTERNAL, read per request like the /uploads proxy), resizes it and
// re-encodes to a modern format with sharp, then serves it with immutable cache
// headers. Because upload keys are UUIDs and every transform parameter is in the
// URL, each variant is content-addressed and cached hard by the browser and the
// Cloudflare CDN in front — the origin only ever encodes each variant once.
import { Buffer } from 'node:buffer'
import sharp from 'sharp'

// Keep in sync with the ladder in app/utils/images.ts. A fixed ladder bounds the
// number of variants a caller can request (cache/CPU abuse guard).
const WIDTH_LADDER = [48, 96, 128, 192, 256, 384, 512, 768, 1024, 1440, 1920]
const MAX_WIDTH = WIDTH_LADDER[WIDTH_LADDER.length - 1]!

// Flat "<uuid>.<ext>" keys only — no slashes, no traversal, no absolute URLs.
// This is the SSRF/path-traversal guard since the key is interpolated into the
// upstream fetch URL.
const KEY_RE = /^[A-Za-z0-9][A-Za-z0-9._-]*\.(png|jpe?g|webp|avif|gif)$/i
const OUTPUT_FORMATS = new Set(['webp', 'avif', 'jpeg', 'png'])
const IMMUTABLE = 'public, max-age=31536000, immutable'

const MIME: Record<string, string> = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  webp: 'image/webp',
  avif: 'image/avif',
  gif: 'image/gif'
}

function snapWidth(width: number): number {
  const n = Math.round(width)
  if (!Number.isFinite(n) || n <= 0) return 0
  for (const step of WIDTH_LADDER) if (n <= step) return step
  return MAX_WIDTH
}

function extensionOf(key: string): string {
  return key.slice(key.lastIndexOf('.') + 1).toLowerCase()
}

function sendBuffer(event: any, body: Buffer, contentType: string) {
  setResponseHeader(event, 'content-type', contentType)
  setResponseHeader(event, 'cache-control', IMMUTABLE)
  setResponseHeader(event, 'x-content-type-options', 'nosniff')
  setResponseHeader(event, 'content-length', String(body.length))
  return body
}

export default defineEventHandler(async (event) => {
  const key = getRouterParam(event, 'path') || ''
  if (!KEY_RE.test(key)) {
    throw createError({ statusCode: 400, statusMessage: 'Invalid image key' })
  }

  const base = (process.env.API_INTERNAL || 'http://localhost:3001').replace(/\/+$/, '')
  const query = getQuery(event)
  const width = snapWidth(Number(query.w) || 0)
  const requestedFormat = String(query.f || '')
  const format = OUTPUT_FORMATS.has(requestedFormat) ? requestedFormat : ''
  const quality = Math.min(90, Math.max(30, Math.round(Number(query.q) || 72)))

  const upstream = await fetch(`${base}/uploads/${encodeURIComponent(key)}`)
  if (!upstream.ok) {
    throw createError({ statusCode: upstream.status === 404 ? 404 : 502, statusMessage: 'Upstream image unavailable' })
  }
  const upstreamType = upstream.headers.get('content-type') || MIME[extensionOf(key)] || 'application/octet-stream'
  const input = Buffer.from(await upstream.arrayBuffer())

  const isGif = extensionOf(key) === 'gif' || upstreamType.includes('gif')
  // Nothing to do (no resize + no re-encode) or an animated format we won't
  // flatten: hand the original bytes straight back.
  if (isGif || (!width && !format)) {
    return sendBuffer(event, input, upstreamType)
  }

  try {
    let pipeline = sharp(input, { failOn: 'error' }).rotate() // bake in EXIF orientation
    if (width) pipeline = pipeline.resize({ width, withoutEnlargement: true })

    const outFormat = format || extensionOf(key).replace('jpg', 'jpeg')
    switch (outFormat) {
      case 'webp':
        pipeline = pipeline.webp({ quality })
        break
      case 'avif':
        pipeline = pipeline.avif({ quality: Math.max(30, quality - 12) })
        break
      case 'jpeg':
        pipeline = pipeline.jpeg({ quality, mozjpeg: true })
        break
      case 'png':
        pipeline = pipeline.png({ compressionLevel: 9 })
        break
      default:
        // Unknown original format we can't target explicitly: return original.
        return sendBuffer(event, input, upstreamType)
    }

    const output = await pipeline.toBuffer()
    return sendBuffer(event, output, MIME[outFormat] || `image/${outFormat}`)
  } catch {
    // Decode/encode failure: never break the image, fall back to the original.
    return sendBuffer(event, input, upstreamType)
  }
})
