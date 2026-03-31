<script setup lang="ts">
const config = useConfigStore()
await config.loadPublicConfig()

const form = reactive({ username: '', password: '' })
const loading = ref(false)
const error = ref('')
const { login } = useAuth()

async function submit() {
  error.value = ''
  loading.value = true
  try {
    await login(form)
    await navigateTo('/admin')
  } catch {
    error.value = 'Invalid username or password.'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <main class="auth-page container">
    <form class="surface auth-card" @submit.prevent="submit">
      <p class="muted">Admin</p>
      <h1>Welcome back</h1>
      <p v-if="error" class="error">{{ error }}</p>
      <label class="form-row">Username<input v-model="form.username" autocomplete="username" required></label>
      <label class="form-row">Password<input v-model="form.password" type="password" autocomplete="current-password" required></label>
      <button class="btn primary" :disabled="loading">{{ loading ? 'Signing in…' : 'Sign in' }}</button>
      <NuxtLink v-if="config.registration_enabled" class="muted" to="/admin/register">Need an account?</NuxtLink>
    </form>
  </main>
</template>

<style scoped>
.auth-page { min-height: 100vh; display: grid; place-items: center; }
.auth-card { width: min(100%, 440px); padding: 28px; display: grid; gap: 16px; }
h1 { margin: 0; font-family: var(--font-heading); }
</style>
