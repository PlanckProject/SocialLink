<script setup lang="ts">
const { user, logout } = useAuth()
const route = useRoute()

const items = [
  { to: '/admin', label: 'Dashboard' },
  { to: '/admin/profile', label: 'Profile' },
  { to: '/admin/links', label: 'Links' },
  { to: '/admin/analytics', label: 'Analytics' },
  { to: '/admin/appearance', label: 'Appearance' },
  { to: '/admin/preview', label: 'Preview' }
]

const firstName = computed(() => {
  const display = user.value?.display_name?.trim()
  if (display) return display.split(/\s+/)[0]
  return user.value?.username || 'there'
})

const open = ref(false)
const close = () => { open.value = false }

watch(() => route.fullPath, close)
onKeyStroke('Escape', close)

if (import.meta.client) {
  watch(open, value => {
    document.body.style.overflow = value ? 'hidden' : ''
  })
  onBeforeUnmount(() => { document.body.style.overflow = '' })
}
</script>

<template>
  <header class="admin-header">
    <div class="bar">
      <button class="hamburger" type="button" aria-label="Open navigation" aria-controls="admin-drawer" :aria-expanded="open" @click="open = true">
        <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true"><path d="M3 6h18M3 12h18M3 18h18" stroke="currentColor" stroke-width="2" stroke-linecap="round" /></svg>
      </button>
      <span class="brand">Hi {{ firstName }}!</span>
      <nav class="top-nav" aria-label="Admin navigation">
        <NuxtLink v-for="item in items" :key="item.to" :to="item.to">{{ item.label }}</NuxtLink>
      </nav>
      <button class="btn ghost logout" type="button" @click="logout">Logout</button>
    </div>

    <div class="scrim" :class="{ show: open }" @click="close" />
    <aside id="admin-drawer" class="drawer" :class="{ open }" :aria-hidden="!open" :inert="!open">
      <div class="drawer-head">
        <span class="brand">Hi {{ firstName }}!</span>
        <button class="icon-btn" type="button" aria-label="Close navigation" @click="close">
          <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" stroke="currentColor" stroke-width="2" stroke-linecap="round" /></svg>
        </button>
      </div>
      <nav class="drawer-nav" aria-label="Admin navigation">
        <NuxtLink v-for="item in items" :key="item.to" :to="item.to" @click="close">{{ item.label }}</NuxtLink>
      </nav>
      <button class="btn ghost" type="button" @click="logout">Logout</button>
    </aside>
  </header>
</template>

<style scoped>
.admin-header { margin-bottom: 28px; }
.bar { display: flex; align-items: center; gap: 12px; }
.brand { font-family: var(--font-heading); font-weight: 800; font-size: clamp(1.15rem, 4vw, 1.5rem); white-space: nowrap; }

.hamburger { display: inline-grid; place-items: center; width: 44px; height: 44px; border: 1px solid var(--color-border); border-radius: var(--radius-button); background: color-mix(in srgb, var(--color-surface-alt) 85%, transparent); color: var(--color-text); cursor: pointer; }
.hamburger:hover { border-color: var(--color-primary); }

.top-nav { display: none; }
.top-nav a { display: inline-flex; align-items: center; min-height: 42px; padding: 8px 15px; border-radius: var(--radius-button); color: var(--color-text); border: 1px solid transparent; transition: background .18s ease, color .18s ease; }
.top-nav a:hover { background: color-mix(in srgb, var(--color-surface-alt) 85%, transparent); }
.top-nav a.router-link-exact-active { background: var(--color-primary); color: var(--color-primary-contrast); }
.logout { display: none; }

/* Material-style left navigation drawer (phones) */
.scrim { position: fixed; inset: 0; z-index: 40; background: rgba(0,0,0,.45); opacity: 0; pointer-events: none; transition: opacity .25s ease; }
.scrim.show { opacity: 1; pointer-events: auto; }
.drawer { position: fixed; top: 0; left: 0; z-index: 50; display: flex; flex-direction: column; gap: 6px; width: min(80vw, 300px); height: 100dvh; padding: 16px; background: var(--color-surface); border-right: 1px solid var(--color-border); box-shadow: 4px 0 24px rgba(0,0,0,.28); transform: translateX(-100%); transition: transform .28s cubic-bezier(.4,0,.2,1); }
.drawer.open { transform: translateX(0); }
.drawer-head { display: flex; align-items: center; justify-content: space-between; padding-bottom: 10px; margin-bottom: 6px; border-bottom: 1px solid var(--color-border); }
.icon-btn { display: inline-grid; place-items: center; width: 40px; height: 40px; border: 0; border-radius: var(--radius-button); background: transparent; color: var(--color-text); cursor: pointer; }
.icon-btn:hover { background: color-mix(in srgb, var(--color-surface-alt) 85%, transparent); }
.drawer-nav { display: flex; flex-direction: column; gap: 4px; margin-bottom: 10px; }
.drawer-nav a { display: flex; align-items: center; min-height: 48px; padding: 12px 14px; border-radius: var(--radius-button); color: var(--color-text); }
.drawer-nav a:hover { background: color-mix(in srgb, var(--color-surface-alt) 85%, transparent); }
.drawer-nav a.router-link-exact-active { background: color-mix(in srgb, var(--color-primary) 16%, transparent); color: var(--color-primary); font-weight: 600; }

@media (min-width: 761px) {
  .hamburger { display: none; }
  .top-nav { display: flex; flex-wrap: wrap; gap: 8px; margin-left: auto; }
  .logout { display: inline-flex; }
  .scrim, .drawer { display: none; }
}
</style>
