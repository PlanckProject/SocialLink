// Runtime reverse proxy: forwards /api/** to the backend API. The upstream
// target is read from API_INTERNAL on every request so it is configured at
// deploy time, not baked at build time. Nitro strips the /api prefix here, so
// /api/config is proxied to <API_INTERNAL>/config.
//
// redirect: 'manual' makes this behave like a real reverse proxy: upstream 3xx
// responses (e.g. the /l/:id click redirect) are passed straight through to the
// browser instead of being followed server-side, so the browser navigates to
// the link's target rather than rendering it under the localhost URL.
export default defineEventHandler((event) => {
  const base = (process.env.API_INTERNAL || 'http://localhost:3001').replace(/\/+$/, '')
  const path = getRouterParam(event, 'path') || ''
  const search = getRequestURL(event).search
  return proxyRequest(event, `${base}/${path}${search}`, { fetchOptions: { redirect: 'manual' } })
})
