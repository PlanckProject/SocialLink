<script setup lang="ts">
import type { PublicProfile, SocialLink } from '~/types/social'

definePageMeta({ middleware: 'auth' })
interface AdminProfile extends PublicProfile { email: string }
const config = useConfigStore()
await config.loadPublicConfig()
const auth = useAuthStore()
const { data, refresh } = await useAsyncData('admin-profile', () => apiFetch<AdminProfile>('/api/admin/profile'))
const form = reactive({ username: '', display_name: '', bio: '', location: '', socials: [] as SocialLink[] })
const message = ref('')
const error = ref('')
const passwordForm = reactive({ current_password: '', new_password: '', confirm_password: '' })
const passwordMessage = ref('')
const passwordError = ref('')
const changingPassword = ref(false)
const socialPlatforms = SOCIAL_PLATFORMS
const MIN_PASSWORD_LENGTH = 8

// Live username availability (multi mode only). Debounced check against the API
// so the user gets immediate feedback and cannot save a taken/invalid handle.
type UsernameState = 'idle' | 'checking' | 'available' | 'taken' | 'invalid'
const usernameState = ref<UsernameState>('idle')
const usernameReason = ref('')
let usernameTimer: ReturnType<typeof setTimeout> | null = null
// Wait until the user pauses typing before querying the API, so we don't hammer
// the backend on every keystroke.
const USERNAME_CHECK_DEBOUNCE_MS = 700

async function checkUsername(name: string) {
  try {
    const res = await apiFetch<{ valid: boolean; available: boolean; reason: string | null }>(`/api/username-available?username=${encodeURIComponent(name)}`)
    if (form.username.trim() !== name) return // a newer keystroke superseded this
    if (!res.valid) { usernameState.value = 'invalid'; usernameReason.value = res.reason || 'That username is not allowed.' }
    else if (!res.available) { usernameState.value = 'taken'; usernameReason.value = res.reason || 'That username is taken.' }
    else { usernameState.value = 'available'; usernameReason.value = '' }
  } catch {
    if (form.username.trim() !== name) return
    usernameState.value = 'idle'; usernameReason.value = ''
  }
}

watch(() => form.username, value => {
  if (usernameTimer) clearTimeout(usernameTimer)
  const name = value.trim()
  // Only relevant in multi mode, and never for the user's own current handle.
  if (config.mode !== 'multi' || !name || name === (data.value?.username || '')) {
    usernameState.value = 'idle'; usernameReason.value = ''
    return
  }
  usernameState.value = 'checking'
  usernameTimer = setTimeout(() => checkUsername(name), USERNAME_CHECK_DEBOUNCE_MS)
})

watchEffect(() => {
  if (!data.value) return
  form.username = data.value.username || ''
  form.display_name = data.value.display_name || ''
  form.bio = data.value.bio || ''
  form.location = data.value.location || ''
  form.socials = (data.value.socials || []).map(social => ({ platform: detectPlatform(social.platform, social.url).slug, url: social.url }))
})

function addSocial() { form.socials.push({ platform: 'website', url: '' }) }
function removeSocial(index: number) { form.socials.splice(index, 1) }
async function save() {
  error.value = ''; message.value = ''
  if (usernameState.value === 'taken' || usernameState.value === 'invalid') {
    error.value = usernameReason.value || 'Please choose a different username.'
    return
  }
  // Username is editable in multi mode only; send it just when it changed so
  // the API re-issues the auth cookie (and updates theme ownership) only then.
  const nextUsername = form.username.trim()
  const renaming = config.mode === 'multi' && nextUsername !== (data.value?.username || '')
  const body: Record<string, unknown> = { display_name: form.display_name, bio: form.bio, location: form.location, socials: form.socials }
  if (renaming) body.username = nextUsername
  try {
    await apiFetch('/api/admin/profile', { method: 'PUT', body })
    message.value = renaming ? `Profile saved. Your public page is now /@${nextUsername}.` : 'Profile saved.'
    if (renaming) await auth.fetchMe()
    await refresh()
  } catch (err: any) {
    error.value = err?.data?.message || err?.data?.error || 'Unable to save profile.'
  }
}
async function changePassword() {
  passwordMessage.value = ''
  passwordError.value = ''
  if (passwordForm.new_password !== passwordForm.confirm_password) {
    passwordError.value = 'New passwords do not match.'
    return
  }
  if (passwordForm.new_password.length < MIN_PASSWORD_LENGTH) {
    passwordError.value = `New password must be at least ${MIN_PASSWORD_LENGTH} characters.`
    return
  }
  if (passwordForm.new_password === passwordForm.current_password) {
    passwordError.value = 'New password must be different from the current password.'
    return
  }

  changingPassword.value = true
  try {
    await apiFetch('/api/admin/password', {
      method: 'PUT',
      body: {
        current_password: passwordForm.current_password,
        new_password: passwordForm.new_password
      }
    })
    Object.assign(passwordForm, { current_password: '', new_password: '', confirm_password: '' })
    passwordMessage.value = 'Password changed.'
  } catch (err: any) {
    passwordError.value = err?.data?.message || err?.data?.error || 'Unable to change password.'
  } finally {
    changingPassword.value = false
  }
}
// Upload constraints enforced client-side for instant, friendly feedback (the
// API also caps bytes). Avatars must be square and small enough to stay crisp;
// covers look best at a 2:1 ratio and are capped so they never balloon in size.
const AVATAR_MAX_DIM = 1024
const COVER_MAX_WIDTH = 2400
const COVER_MAX_HEIGHT = 1200

function readImageSize(file: File): Promise<{ width: number; height: number }> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file)
    const image = new Image()
    image.onload = () => { URL.revokeObjectURL(url); resolve({ width: image.naturalWidth, height: image.naturalHeight }) }
    image.onerror = () => { URL.revokeObjectURL(url); reject(new Error('That file could not be read as an image.')) }
    image.src = url
  })
}

function validateImage(kind: 'avatar' | 'cover', width: number, height: number): string | null {
  if (kind === 'avatar') {
    if (width !== height) return 'Profile picture must be square (e.g. 512×512 or 1024×1024).'
    if (width > AVATAR_MAX_DIM) return `Profile picture must not exceed ${AVATAR_MAX_DIM}×${AVATAR_MAX_DIM}.`
  } else if (width > COVER_MAX_WIDTH || height > COVER_MAX_HEIGHT) {
    return `Cover must not exceed ${COVER_MAX_WIDTH}×${COVER_MAX_HEIGHT}. A 2:1 image (e.g. 1600×800) works best.`
  }
  return null
}

async function upload(kind: 'avatar' | 'cover', event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  error.value = ''; message.value = ''
  try {
    const { width, height } = await readImageSize(file)
    const problem = validateImage(kind, width, height)
    if (problem) { error.value = problem; return }
    const body = new FormData(); body.append('file', file)
    await apiFetch(`/api/admin/profile/${kind}`, { method: 'POST', body })
    await refresh()
    message.value = kind === 'avatar' ? 'Profile picture updated.' : 'Cover updated.'
  } catch (err: any) {
    error.value = err?.data?.message || err?.data?.error || err?.message || 'Unable to upload image.'
  } finally {
    input.value = '' // let the user re-pick the same file after fixing it
  }
}
</script>

<template>
  <main class="admin-layout">
    <AdminNav />
    <section class="admin-card form-grid">
      <h2>Profile</h2>
      <p v-if="message" class="success">{{ message }}</p><p v-if="error" class="error">{{ error }}</p>
      <template v-if="config.mode === 'multi'">
        <label class="form-row">Username<span class="username-input" :class="usernameState"><span class="at">@</span><input v-model="form.username" autocomplete="off" autocapitalize="none" spellcheck="false" placeholder="username"></span></label>
        <p class="username-feedback" :class="usernameState">
          <template v-if="usernameState === 'checking'">Checking availability…</template>
          <template v-else-if="usernameState === 'available'">✓ @{{ form.username.trim() }} is available</template>
          <template v-else-if="usernameState === 'taken' || usernameState === 'invalid'">✗ {{ usernameReason }}</template>
          <template v-else>Your public page lives at <code>/@{{ (form.username || 'username').trim() }}</code> — changing it updates that link and frees the old one.</template>
        </p>
      </template>
      <label class="form-row">Display name<input v-model="form.display_name" required></label>
      <label class="form-row">Bio<textarea v-model="form.bio" maxlength="500" rows="4" /><span class="char-count" :class="{ over: form.bio.length >= 500 }">{{ form.bio.length }}/500</span></label>
      <label class="form-row">Location<input v-model="form.location"></label>
      <div class="uploads"><label class="form-row">Avatar<input type="file" accept="image/*" @change="upload('avatar', $event)"><small class="upload-hint">Square image · max 1024×1024</small></label><label class="form-row">Cover<input type="file" accept="image/*" @change="upload('cover', $event)"><small class="upload-hint">Recommended 2:1 ratio · max 2400×1200 (e.g. 1600×800)</small></label></div>
      <div class="social-editor">
        <div class="row-head"><h3>Socials</h3><button class="btn" type="button" @click="addSocial">Add social</button></div>
        <p class="muted hint">Choose an app — its brand icon appears around your bio. Paste a full link, username, or email.</p>
        <div v-for="(social, index) in form.socials" :key="index" class="social-row">
          <span class="social-preview"><SocialIcon :platform="social.platform" :url="social.url" decorative /></span>
          <select v-model="social.platform" class="social-platform" aria-label="Platform">
            <option v-for="platform in socialPlatforms" :key="platform.slug" :value="platform.slug">{{ platform.label }}</option>
          </select>
          <input v-model="social.url" class="social-url" placeholder="https://… , @username or you@email.com">
          <button class="btn danger social-remove" type="button" aria-label="Remove social" @click="removeSocial(index)">Remove</button>
        </div>
      </div>
      <button class="btn primary" type="button" :disabled="usernameState === 'checking' || usernameState === 'taken' || usernameState === 'invalid'" @click="save">Save profile</button>
    </section>

    <section class="admin-card password-card">
      <form class="form-grid password-form" @submit.prevent="changePassword">
        <h2>Change password</h2>
        <p class="muted password-hint">Enter your current password, then choose a new password with at least {{ MIN_PASSWORD_LENGTH }} characters.</p>
        <p v-if="passwordMessage" class="success">{{ passwordMessage }}</p>
        <p v-if="passwordError" class="error">{{ passwordError }}</p>
        <label class="form-row">Current password<input v-model="passwordForm.current_password" type="password" name="current-password" autocomplete="current-password" required></label>
        <label class="form-row">New password<input v-model="passwordForm.new_password" type="password" name="new-password" autocomplete="new-password" required :minlength="MIN_PASSWORD_LENGTH"></label>
        <label class="form-row">Confirm new password<input v-model="passwordForm.confirm_password" type="password" name="confirm-password" autocomplete="new-password" required :minlength="MIN_PASSWORD_LENGTH"></label>
        <button class="btn primary" type="submit" :disabled="changingPassword">{{ changingPassword ? 'Changing password…' : 'Change password' }}</button>
      </form>
    </section>
  </main>
</template>

<style scoped>
h2, h3 { font-family: var(--font-heading); margin: 0; }
.password-card { width: min(100%, 560px); margin-top: 18px; }
.password-hint { margin: 0; }
.uploads { display: grid; grid-template-columns: 1fr; gap: 12px; }
.upload-hint { font-size: .78rem; color: var(--color-text-muted); opacity: .7; }
.social-editor { display: grid; gap: 12px; }
.hint { font-size: .88rem; margin: 0; }
.char-count { display: block; margin-top: 4px; font-size: .78rem; color: var(--color-text-muted); text-align: right; }
.char-count.over { color: #fca5a5; }
.username-input { display: flex; align-items: stretch; border: 1px solid var(--color-border); border-radius: var(--radius-input); background: var(--color-surface-alt); }
.username-input:focus-within { border-color: color-mix(in srgb, var(--color-primary) 55%, var(--color-border)); }
.username-input.available { border-color: color-mix(in srgb, #4ade80 55%, var(--color-border)); }
.username-input.taken, .username-input.invalid { border-color: color-mix(in srgb, #f87171 60%, var(--color-border)); }
.username-input .at { display: inline-flex; align-items: center; padding-left: 12px; color: var(--color-text-muted); font-weight: 600; }
.username-input input { flex: 1; min-width: 0; border: 0; background: transparent; padding-left: 4px; }
.username-feedback { margin: -6px 0 0; font-size: .85rem; color: var(--color-text-muted); }
.username-feedback.available { color: #4ade80; }
.username-feedback.taken, .username-feedback.invalid { color: #fca5a5; }
.username-feedback code { padding: 1px 6px; border-radius: 6px; background: color-mix(in srgb, var(--color-surface-alt) 85%, transparent); }
.social-row { display: grid; grid-template-columns: auto 1fr; gap: 10px 12px; align-items: center; }
.social-preview { display: grid; place-items: center; width: 40px; height: 40px; border-radius: var(--radius-icon); border: 1px solid var(--color-border); background: var(--color-surface-alt); color: var(--color-text); }
.social-url { grid-column: 1 / -1; }
.social-remove { grid-column: 1 / -1; justify-self: start; }
.row-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
@media (min-width: 760px) {
  .uploads { grid-template-columns: 1fr 1fr; }
  .social-row { grid-template-columns: auto minmax(150px, .9fr) 1.6fr auto; }
  .social-url, .social-remove { grid-column: auto; justify-self: stretch; }
}
</style>
