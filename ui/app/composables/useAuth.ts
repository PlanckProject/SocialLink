interface LoginPayload { username: string; password: string }
interface RegisterPayload { username: string; email: string; password: string; display_name?: string }

export function useAuth() {
  const auth = useAuthStore()

  async function login(payload: LoginPayload) {
    await apiFetch('/api/auth/login', { method: 'POST', body: payload })
    return auth.fetchMe()
  }

  async function register(payload: RegisterPayload) {
    await apiFetch('/api/auth/register', { method: 'POST', body: payload })
    return auth.fetchMe()
  }

  async function logout() {
    await apiFetch('/api/auth/logout', { method: 'POST' })
    auth.setUser(null)
    await navigateTo('/admin/login')
  }

  return { user: computed(() => auth.user), login, logout, register, fetchMe: auth.fetchMe }
}
