import { Buffer } from 'node:buffer'
import { constants, brotliCompress, gzip } from 'node:zlib'
import { promisify } from 'node:util'

const gzipAsync = promisify(gzip)
const brotliAsync = promisify(brotliCompress)

// Only bother compressing responses that are big enough to beat the ~1 packet
// overhead of the compressed framing.
const MIN_SIZE = 1024

// Dynamic responses are compressed on every request, so favour speed over the
// last few bytes: gzip level 6 and brotli quality 5 are the sweet spot and are
// far cheaper than the default brotli quality 11.
const GZIP_OPTS = { level: 6 }
const brotliOpts = (size: number) => ({
  params: {
    [constants.BROTLI_PARAM_QUALITY]: 5,
    [constants.BROTLI_PARAM_SIZE_HINT]: size
  }
})

/**
 * Compresses dynamic SSR HTML responses.
 *
 * `nitro.compressPublicAssets` only pre-compresses the static files in
 * .output/public; the server-rendered HTML documents are produced per request
 * and would otherwise go out uncompressed (the app is served by a bare Nitro
 * node server with no reverse proxy doing gzip). The `render:response` hook
 * fires only for the Nuxt page renderer, so the `/api/**` and `/uploads/**`
 * reverse proxies are never touched here.
 */
export default defineNitroPlugin((nitro) => {
  nitro.hooks.hook('render:response', async (response, { event }) => {
    const body = response.body
    if (typeof body !== 'string' || body.length < MIN_SIZE) return

    const headers = (response.headers ||= {}) as Record<string, string>

    // Don't touch anything that is already encoded, and only compress HTML.
    if (headers['content-encoding']) return
    const contentType = String(headers['content-type'] || 'text/html')
    if (!contentType.includes('text/html')) return

    const accept = String(getRequestHeader(event, 'accept-encoding') || '')
    const useBrotli = /\bbr\b/.test(accept)
    const useGzip = !useBrotli && /\bgzip\b/.test(accept)
    if (!useBrotli && !useGzip) return

    const raw = Buffer.from(body, 'utf-8')
    const compressed = useBrotli
      ? await brotliAsync(raw, brotliOpts(raw.length))
      : await gzipAsync(raw, GZIP_OPTS)

    response.body = compressed
    headers['content-encoding'] = useBrotli ? 'br' : 'gzip'
    headers['content-length'] = String(compressed.length)
    headers['vary'] = headers['vary']
      ? Array.from(new Set(`${headers['vary']}, Accept-Encoding`.split(/,\s*/))).join(', ')
      : 'Accept-Encoding'
  })
})
