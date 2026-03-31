<script setup lang="ts">
const config = useConfigStore()
await config.loadPublicConfig()
if (config.mode !== 'multi' || !config.registration_enabled) await navigateTo('/admin/login')

const form = reactive({ username: '', email: '', password: '', display_name: '' })
const loading = ref(false)
const error = ref('')
const { register } = useAuth()

async function submit() {
  error.value = ''
  loading.value = true
  try {
    await register({ ...form, display_name: form.display_name || undefined })
    await navigateTo('/admin')
  } catch {
    error.value = 'Unable to create that account.'
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <main class="auth-page container">
    <form class="surface auth-card" @submit.prevent="submit">
      <p class="muted">SocialLink</p>
      <h1>Create your profile</h1>
      <p v-if="error" class="error">{{ error }}</p>
      <label class="form-row">Username<input v-model="form.username" autocomplete="username" required></label>
      <label class="form-row">Email<input v-model="form.email" type="email" autocomplete="email" required></label>
      <label class="form-row">Display name<input v-model="form.display_name" autocomplete="name"></label>
      <label class="form-row">Password<input v-model="form.password" type="password" autocomplete="new-password" required minlength="8"></label>
      <button class="btn primary" :disabled="loading">{{ loading ? 'Creating…' : 'Create account' }}</button>
      <NuxtLink class="muted" to="/admin/login">Already have an account?</NuxtLink>
    </form>
  </main>
</template>

<style scoped>
.auth-page { min-height: 100vh; display: grid; place-items: center; }
.auth-card { width: min(100%, 460px); padding: 28px; display: grid; gap: 16px; }
h1 { margin: 0; font-family: var(--font-heading); }
</style>
