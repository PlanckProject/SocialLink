<script setup lang="ts">
interface Swatch { background: string; primary: string; accent: string }
type Action = 'edit' | 'download' | 'favorite' | 'delete'

const props = defineProps<{
  title: string
  swatch: Swatch
  active?: boolean
  owner?: string | null
  favorite?: boolean
  downloads?: number
  actions?: Action[]
}>()

const emit = defineEmits<{
  apply: []
  edit: []
  download: []
  favorite: []
  delete: []
}>()

const menuActions = computed(() => props.actions ?? [])
const editOnly = computed(() => menuActions.value.length === 1 && menuActions.value[0] === 'edit')
const open = ref(false)
const confirmingDelete = ref(false)
const root = ref<HTMLElement | null>(null)
const deleteConfirmButton = ref<HTMLButtonElement | null>(null)
onClickOutside(root, () => { open.value = false })
onKeyStroke('Escape', () => {
  open.value = false
  confirmingDelete.value = false
})
watch(confirmingDelete, async (visible) => {
  if (!visible) return
  await nextTick()
  deleteConfirmButton.value?.focus()
})

function run(action: Action) {
  open.value = false
  if (action === 'delete') {
    confirmingDelete.value = true
    return
  }
  emit(action)
}

function confirmDelete() {
  confirmingDelete.value = false
  emit('delete')
}

const labels: Record<Action, string> = {
  edit: 'Edit in editor',
  download: 'Download',
  favorite: 'Favorite',
  delete: 'Delete'
}
</script>

<template>
  <article
    ref="root"
    class="theme-card"
    :class="{ active, 'menu-open': open }"
    role="button"
    :tabindex="0"
    :aria-label="`Apply ${title} theme`"
    :aria-pressed="active"
    @click="emit('apply')"
    @keydown.enter.prevent="emit('apply')"
    @keydown.space.prevent="emit('apply')"
  >
    <span class="swatch" :style="{ background: swatch.background || swatch.primary }">
      <span class="dot" :style="{ background: swatch.primary }" />
      <span class="dot" :style="{ background: swatch.accent }" />
    </span>

    <span class="meta">
      <strong class="name">{{ title }}</strong>
      <small v-if="owner" class="owner">@{{ owner }}</small>
    </span>

    <span v-if="active" class="active-tag" aria-hidden="true">Active</span>
    <span v-else class="apply-hint" aria-hidden="true">Tap to apply</span>

    <div v-if="menuActions.length" class="menu-wrap" @click.stop>
      <button
        class="menu-btn"
        :class="{ 'edit-only': editOnly }"
        type="button"
        :aria-label="`More options for ${title}`"
        aria-haspopup="menu"
        :aria-expanded="open"
        @click="open = !open"
      >
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><circle cx="12" cy="5" r="1.6" fill="currentColor" /><circle cx="12" cy="12" r="1.6" fill="currentColor" /><circle cx="12" cy="19" r="1.6" fill="currentColor" /></svg>
      </button>
      <div v-if="open" class="menu" role="menu">
        <button
          v-for="action in menuActions"
          :key="action"
          class="menu-item"
          :class="{ danger: action === 'delete', 'edit-action': action === 'edit' }"
          type="button"
          role="menuitem"
          @click="run(action)"
        >
          <span v-if="action === 'favorite'">{{ favorite ? '★ Unfavorite' : '☆ Favorite' }}</span>
          <span v-else-if="action === 'download'">{{ labels.download }}<template v-if="downloads"> · {{ downloads }}</template></span>
          <span v-else>{{ labels[action] }}</span>
        </button>
      </div>
    </div>
  </article>

  <Teleport to="body">
    <div
      v-if="confirmingDelete"
      class="delete-backdrop"
      role="presentation"
      @click.self="confirmingDelete = false"
    >
      <section
        class="delete-dialog"
        role="alertdialog"
        aria-modal="true"
        :aria-label="`Delete ${title} theme`"
        @click.stop
      >
        <div>
          <p class="delete-eyebrow">Delete theme</p>
          <h2>{{ title }}</h2>
          <p>This permanently removes the theme from your library.</p>
        </div>
        <div class="delete-actions">
          <button class="btn ghost" type="button" @click="confirmingDelete = false">Cancel</button>
          <button ref="deleteConfirmButton" class="btn danger" type="button" @click="confirmDelete">Delete theme</button>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.theme-card {
  position: relative;
  display: grid;
  grid-template-rows: auto auto;
  gap: 8px;
  padding: 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-background);
  background: color-mix(in srgb, var(--color-surface) 70%, transparent);
  cursor: pointer;
  text-align: left;
  transition: border-color .15s ease, transform .15s ease, box-shadow .15s ease;
}
.theme-card:focus-visible { outline: 2px solid var(--color-primary); outline-offset: 2px; }
.theme-card.active { border-color: var(--color-primary); box-shadow: 0 0 0 1px var(--color-primary) inset; }
.theme-card.menu-open { z-index: 30; }
@media (hover: hover) {
  .theme-card:hover { transform: translateY(-2px); box-shadow: var(--button-shadow); border-color: color-mix(in srgb, var(--color-primary) 60%, var(--color-border)); }
  .apply-hint { opacity: 0; transition: opacity .15s ease; }
  .theme-card:hover .apply-hint { opacity: 1; }
}
.swatch { position: relative; display: block; height: 56px; border-radius: max(0px, calc(var(--radius-background) - 4px)); border: 1px solid var(--color-border); overflow: hidden; }
.dot { position: absolute; bottom: 6px; width: 14px; height: 14px; border-radius: 999px; border: 2px solid #fff; box-shadow: 0 1px 3px rgba(0,0,0,.3); }
.dot:first-of-type { right: 24px; }
.dot:last-of-type { right: 6px; }
.meta { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.name { font-size: .9rem; line-height: 1.2; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.owner { color: var(--color-text-muted); font-size: .72rem; }
.active-tag { position: absolute; top: 8px; left: 8px; font-size: .66rem; font-weight: 600; padding: 2px 7px; border-radius: var(--radius-button); background: var(--color-primary); color: var(--color-primary-contrast); }
.apply-hint { position: absolute; top: 8px; left: 8px; font-size: .66rem; padding: 2px 7px; border-radius: var(--radius-button); background: color-mix(in srgb, var(--color-surface) 80%, transparent); color: var(--color-text-muted); border: 1px solid var(--color-border); }
.menu-wrap { position: absolute; top: 6px; right: 6px; z-index: 2; }
.menu-btn { display: grid; place-items: center; width: 30px; height: 30px; border-radius: var(--radius-button); border: 1px solid var(--color-border); background: color-mix(in srgb, var(--color-surface) 85%, transparent); color: var(--color-text); cursor: pointer; }
.menu-btn:hover { background: var(--color-surface); }
.menu { position: absolute; top: 34px; right: 0; z-index: 20; min-width: 150px; display: grid; gap: 2px; padding: 6px; border: 1px solid var(--color-border); border-radius: var(--radius-background); background: var(--color-surface); box-shadow: 0 12px 30px rgba(0,0,0,.28); }
.menu-item { text-align: left; padding: 8px 10px; border: 0; border-radius: var(--radius-button); background: transparent; color: var(--color-text); font-size: .84rem; cursor: pointer; }
.menu-item:hover { background: color-mix(in srgb, var(--color-primary) 14%, transparent); }
.menu-item.danger { color: #ef4444; }
.menu-item.danger:hover { background: color-mix(in srgb, #ef4444 14%, transparent); }
.delete-backdrop { position: fixed; inset: 0; z-index: 1000; display: grid; place-items: center; padding: 20px; background: rgba(0,0,0,.62); backdrop-filter: blur(8px); }
.delete-dialog { width: min(100%, 420px); display: grid; gap: 20px; padding: 22px; border: 1px solid var(--color-border); border-radius: var(--radius-background); background: var(--color-surface); color: var(--color-text); box-shadow: 0 24px 70px rgba(0,0,0,.5); }
.delete-dialog h2 { margin: 2px 0 6px; overflow-wrap: anywhere; }
.delete-dialog p { margin: 0; color: var(--color-text-muted); }
.delete-dialog .delete-eyebrow { color: #ef4444; font-size: .76rem; font-weight: 700; letter-spacing: .04em; }
.delete-actions { display: flex; justify-content: flex-end; gap: 10px; }

/* Editing isn't available on phones, so drop the Edit action there. A preset
   card whose only action is Edit then has no menu, so hide the button too. */
@media (max-width: 760px) {
  .menu-item.edit-action { display: none; }
  .menu-btn.edit-only { display: none; }
}
</style>
