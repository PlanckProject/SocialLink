export default defineNuxtConfig({
  ssr: true,
  srcDir: 'app/',
  compatibilityDate: '2025-07-15',
  devtools: { enabled: true },
  modules: ['@pinia/nuxt', '@vueuse/nuxt'],
  css: ['@fortawesome/fontawesome-free/css/all.min.css', '~/assets/css/main.css'],
  // Honor the PORT/HOST env vars for `nuxt dev` and `nuxt preview`. In
  // production our launcher (`node serve.mjs`) reads PORT/HOST itself before
  // handing requests to the Nitro `node`-preset handler.
  devServer: {
    host: process.env.HOST || '0.0.0.0',
    port: Number(process.env.PORT) || 3000
  },
  app: {
    head: {
      title: 'SocialLink',
      meta: [
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
        { name: 'description', content: 'A refined one-link-in-bio profile.' },
        { name: 'theme-color', content: '#0b0b0f' }
      ],
      link: [
        { rel: 'icon', href: '/favicon.ico' },
        // Google Fonts are pulled in at runtime by useTheme() as a render-blocking
        // stylesheet. Warm up the TCP+TLS connections to the font origins early so
        // the CSS (googleapis) and the actual woff2 files (gstatic) start sooner.
        { rel: 'preconnect', href: 'https://fonts.googleapis.com' },
        { rel: 'preconnect', href: 'https://fonts.gstatic.com', crossorigin: '' },
        { rel: 'dns-prefetch', href: 'https://fonts.googleapis.com' },
        { rel: 'dns-prefetch', href: 'https://fonts.gstatic.com' }
      ]
    }
  },
  runtimeConfig: {
    public: {
      apiBase: process.env.API_BASE || '',
      // On-the-fly image optimizer (server/routes/img). Set
      // NUXT_PUBLIC_IMAGE_OPTIMIZE=false to bypass it and serve upload originals
      // (e.g. if sharp is unavailable on the host).
      imageOptimize: process.env.NUXT_PUBLIC_IMAGE_OPTIMIZE !== 'false'
    }
  },

  // Extract the SSR payload into a cacheable _payload.json instead of inlining
  // it all into the HTML. This shrinks the document and lets client-side route
  // prefetching reuse cached payloads.
  experimental: {
    payloadExtraction: true,
    renderJsonPayloads: true
  },

  nitro: {
    // Emit a standalone Node handler (the `node`/`node-listener` preset exports
    // `handler`/`listener` instead of auto-starting a plain HTTP server). Our
    // custom launcher `serve.mjs` imports that handler and wraps it in an
    // HTTP/2 (h2) + HTTP/1.1 TLS server — see serve.mjs and DEVELOPMENT.md.
    preset: 'node',
    // The `node` preset does not serve `.output/public` by itself (unlike
    // `node-server`), so enable it explicitly: our launcher is the only thing
    // in front of the app, there is no separate static file server.
    serveStatic: true,

    // Pre-compress every static asset in .output/public at build time so the
    // Node server can serve ready-made .br / .gz variants (Nitro negotiates via
    // Accept-Encoding) with zero per-request CPU cost. Dynamic SSR HTML is
    // compressed separately in server/plugins/compression.ts.
    compressPublicAssets: { gzip: true, brotli: true },

    routeRules: {
      // Hashed build assets are content-addressed and safe to cache forever.
      // (Nitro already sets this by default; declared here to be explicit.)
      '/_nuxt/**': { headers: { 'cache-control': 'public, max-age=31536000, immutable' } },
      // Static files shipped in /public.
      '/favicon.ico': { headers: { 'cache-control': 'public, max-age=604800' } },
      '/robots.txt': { headers: { 'cache-control': 'public, max-age=86400' } },
      // User-uploaded media proxied from the API. Upload keys are content-
      // addressed UUIDs, so a given URL's bytes never change — cache hard. The
      // API already sends immutable; make it explicit so the CDN edge does too.
      '/uploads/**': { headers: { 'cache-control': 'public, max-age=31536000, immutable' } },
      // Optimized (resized / re-encoded) variants of the above. Fully content-
      // addressed by key + query params, so equally immutable.
      '/img/**': { headers: { 'cache-control': 'public, max-age=31536000, immutable' } },
      // The API proxy carries auth'd, always-fresh data. Never cache it.
      '/api/**': { headers: { 'cache-control': 'no-store' }, index: false },
      // Admin is private and already noindex'd in-page; make it uncacheable and
      // unindexable at the header level too.
      '/admin/**': { headers: { 'cache-control': 'no-store, must-revalidate', 'x-robots-tag': 'noindex, nofollow' }, index: false },
      // Public profile / landing documents: let browsers revalidate every time
      // but let shared caches (CDN / reverse proxy) serve a 60s copy and keep
      // serving stale for 5 min while they refresh in the background. Ideal for
      // a widely-shared link-in-bio page under traffic spikes.
      '/**': { headers: { 'cache-control': 'public, max-age=0, s-maxage=60, stale-while-revalidate=300' } }
    }
  },

  vite: {
    build: {
      // esbuild CSS minification is on by default in production; keep it explicit.
      cssMinify: true,
      cssCodeSplit: true,
      rollupOptions: {
        output: {
          // Chunking strategy tuned for "size vs number of files":
          //  - Consolidate all third-party runtime (Vue, Pinia, Router, VueUse)
          //    into ONE long-lived `vendor` chunk that rarely changes, so repeat
          //    visits and deploys reuse it from cache.
          //  - Keep the heavy, admin-only charting stack (chart.js + vue-chartjs,
          //    ~160 KB) in its own `charts` chunk so it is lazy-loaded only on the
          //    analytics page and never weighs down the public profile.
          // Route/component chunks are left to Nuxt's per-page code splitting.
          manualChunks(id: string) {
            if (id.includes('node_modules')) {
              if (/[\\/]node_modules[\\/](chart\.js|vue-chartjs)[\\/]/.test(id)) return 'charts'
              return 'vendor'
            }
          }
        }
      }
    }
  }
})
