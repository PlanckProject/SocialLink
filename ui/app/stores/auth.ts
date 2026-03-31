import { defineStore } from 'pinia'

interface User { username: string; display_name: string }

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const loading = ref(false)

  async function fetchMe() {
    loading.value = true
    try {
      const response = await apiFetch<{ user: User }>('/api/auth/me')
      user.value = response.user
      return user.value
    } finally {
      loading.value = false
    }
  }

  function setUser(next: User | null) { user.value = next }
  return { user, loading, fetchMe, setUser }
})
