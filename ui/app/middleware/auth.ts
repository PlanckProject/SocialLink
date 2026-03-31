export default defineNuxtRouteMiddleware(async to => {
  if (!to.path.startsWith('/admin') || ['/admin/login', '/admin/register'].includes(to.path)) return
  const auth = useAuthStore()
  if (auth.user) return
  try {
    await auth.fetchMe()
  } catch {
    return navigateTo('/admin/login')
  }
})
