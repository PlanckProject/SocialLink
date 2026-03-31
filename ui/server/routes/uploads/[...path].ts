// Runtime reverse proxy for uploaded media served by the API's /uploads dir.
// The upstream target is read from API_INTERNAL per request (not baked at build).
export default defineEventHandler((event) => {
  const base = (process.env.API_INTERNAL || 'http://localhost:3001').replace(/\/+$/, '')
  const path = getRouterParam(event, 'path') || ''
  const search = getRequestURL(event).search
  return proxyRequest(event, `${base}/uploads/${path}${search}`)
})
