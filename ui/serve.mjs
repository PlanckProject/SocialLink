// Production launcher for the Nuxt SSR UI with HTTP/2 (h2) + HTTP/1.1.
//
// Nitro's default `node-server` preset starts a plain HTTP/1.1 server (or
// HTTPS/1.1 when NITRO_SSL_* is set) and can never negotiate HTTP/2. We instead
// build with the `node` preset (nuxt.config.ts), which exports the request
// `handler` without listening, and wrap it here in a real HTTP/2 server.
//
// Behaviour is driven entirely by environment variables (set by the `ui`
// service in docker-compose.yml, mirrored as the `tls` block in
// config/<env>/server.yaml):
//
//   UI_TLS_ENABLED  "true" to terminate TLS here. Default: false (plain HTTP).
//   UI_TLS_CERT     Path to the PEM certificate (fullchain).
//                   Default: /app/certs/fullchain.pem
//   UI_TLS_KEY      Path to the PEM private key.
//                   Default: /app/certs/privkey.pem
//   UI_HTTP2        "false" to serve HTTPS/1.1 instead of h2. Default: true.
//   UI_HTTP3        "true" to advertise HTTP/3 via an `Alt-Svc` header. Node
//                   cannot terminate QUIC itself, so this only helps when an
//                   external QUIC terminator (e.g. Caddy) fronts this server.
//                   Default: false.
//   UI_HTTP3_PORT   UDP port advertised for h3. Default: the listen port.
//   HOST / PORT     Listen address. Falls back to NITRO_HOST / NITRO_PORT.
//
// When TLS is enabled the HTTP/2 server keeps `allowHTTP1: true`, so HTTP/1.1
// clients (older browsers, health checks, `curl` without nghttp2) still work.

import { createServer as createHttpServer } from 'node:http'
import { createServer as createHttpsServer } from 'node:https'
import { createSecureServer as createHttp2SecureServer } from 'node:http2'
import { readFileSync } from 'node:fs'
import process from 'node:process'

// The `node` preset entry exports `handler` (=== `listener`); fall back across
// the possible export names to stay robust to preset/version differences.
import * as nitro from './.output/server/index.mjs'

const handler = nitro.handler ?? nitro.listener ?? nitro.default
if (typeof handler !== 'function') {
  console.error(
    '[serve] Could not resolve the Nitro request handler from ./.output/server/index.mjs. ' +
      'Was the app built with `nitro.preset = "node"`?'
  )
  process.exit(1)
}

const env = process.env

function flag(value, fallback) {
  if (value === undefined || value === null || value === '') return fallback
  return ['1', 'true', 'yes', 'on'].includes(String(value).trim().toLowerCase())
}

const port = Number(env.PORT || env.NITRO_PORT || 3000)
const host = env.HOST || env.NITRO_HOST || '0.0.0.0'

const tlsEnabled = flag(env.UI_TLS_ENABLED, false)
const http2Enabled = flag(env.UI_HTTP2, true)
const http3Advertise = flag(env.UI_HTTP3, false)
const certPath = env.UI_TLS_CERT || '/app/certs/fullchain.pem'
const keyPath = env.UI_TLS_KEY || '/app/certs/privkey.pem'

// HTTP/2 forbids the connection-specific (hop-by-hop) headers that an HTTP/1.1
// upstream legitimately returns. Nitro's /api proxy (ui/server/api/[...path].ts)
// forwards the API's response headers verbatim, so under h2 it would try to set
// e.g. `Connection` / `Transfer-Encoding` on the Http2ServerResponse — which Node
// rejects with ERR_HTTP2_INVALID_CONNECTION_HEADERS, surfacing to the browser as a
// 502 on every proxied call (login included). Strip these headers on h2 responses
// so proxied responses (and their Set-Cookie) pass through; h2 frames the message
// and its length itself, so removing them is safe. HTTP/1.1 fallback connections on
// the same server are left untouched (Node manages their framing).
const HTTP2_FORBIDDEN_HEADERS = new Set([
  'connection',
  'keep-alive',
  'proxy-connection',
  'transfer-encoding',
  'upgrade',
])

function http2SafeHandler(inner) {
  return (req, res) => {
    if (req.httpVersionMajor >= 2) {
      const setHeader = res.setHeader.bind(res)
      res.setHeader = (name, value) => {
        if (typeof name === 'string' && HTTP2_FORBIDDEN_HEADERS.has(name.toLowerCase())) {
          return res
        }
        return setHeader(name, value)
      }
      const writeHead = res.writeHead.bind(res)
      res.writeHead = (...args) => {
        const last = args[args.length - 1]
        if (last && typeof last === 'object' && !Array.isArray(last)) {
          for (const name of Object.keys(last)) {
            if (HTTP2_FORBIDDEN_HEADERS.has(name.toLowerCase())) delete last[name]
          }
        }
        return writeHead(...args)
      }
    }
    return inner(req, res)
  }
}

// Advertise HTTP/3 (QUIC) via Alt-Svc when requested. Real h3 needs an external
// QUIC terminator; the header only tells capable clients where to find it.
let requestHandler = handler
if (http3Advertise) {
  const h3Port = Number(env.UI_HTTP3_PORT || port)
  const altSvc = `h3=":${h3Port}"; ma=86400`
  requestHandler = (req, res) => {
    try {
      res.setHeader('Alt-Svc', altSvc)
    } catch {
      // Headers may already be flushed on some code paths; advertising is
      // best-effort and must never break the response.
    }
    return handler(req, res)
  }
}

// Applied outermost (so the patch is in place before the Alt-Svc wrapper and
// Nitro run) and only when an h2 server is actually created. See
// http2SafeHandler above for why this is required for the /api proxy under h2.
if (tlsEnabled && http2Enabled) {
  requestHandler = http2SafeHandler(requestHandler)
}

function loadTlsMaterial() {
  try {
    return { key: readFileSync(keyPath), cert: readFileSync(certPath) }
  } catch (err) {
    console.error(
      `[serve] UI_TLS_ENABLED is set but the certificate could not be read ` +
        `(cert: ${certPath}, key: ${keyPath}): ${err.message}`
    )
    process.exit(1)
  }
}

let server
let describe
if (tlsEnabled) {
  const { key, cert } = loadTlsMaterial()
  if (http2Enabled) {
    // h2 over TLS with transparent HTTP/1.1 fallback via ALPN.
    server = createHttp2SecureServer({ key, cert, allowHTTP1: true }, requestHandler)
    describe = 'https (HTTP/2 with HTTP/1.1 fallback)'
  } else {
    server = createHttpsServer({ key, cert }, requestHandler)
    describe = 'https (HTTP/1.1)'
  }
} else {
  server = createHttpServer(requestHandler)
  describe = 'http (HTTP/1.1)'
}

server.on('error', (err) => {
  console.error(`[serve] server error: ${err.message}`)
  process.exit(1)
})

const listener = server.listen(port, host, () => {
  console.log(`[serve] UI listening on ${describe} at ${host}:${port}`)
  if (http3Advertise) {
    console.log(
      '[serve] Advertising HTTP/3 via Alt-Svc. Node does not terminate QUIC; ' +
        'run a QUIC-capable reverse proxy (e.g. Caddy) to actually serve h3.'
    )
  }
})

// Graceful shutdown so `docker compose stop` / SIGTERM drains in-flight requests.
let shuttingDown = false
for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => {
    if (shuttingDown) return
    shuttingDown = true
    console.log(`[serve] ${signal} received, shutting down`)
    listener.close(() => process.exit(0))
    // Don't hang forever if connections are slow to close.
    setTimeout(() => process.exit(0), 10_000).unref()
  })
}
