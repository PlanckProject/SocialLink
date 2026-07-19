<script setup lang="ts">
import type { Branding, Theme } from '~/app.config'
import { DEFAULT_BRANDING } from '~/app.config'
import type { AdminGroup, GroupStyle } from '~/types/social'
import { DEFAULT_GROUP_STYLE } from '~/types/social'
import { cloneTheme, PRESET_THEMES } from '~/config/themes'

definePageMeta({ middleware: 'auth' })
interface ThemeSummary { id: string; name: string; owner: string | null; is_active: boolean; is_favorite: boolean; is_public: boolean; description: string | null; tags: string[]; source: string; download_count: number; swatch: { background: string; primary: string; accent: string }; created_at: string; updated_at: string }
interface ThemeDetail extends ThemeSummary { config: Theme }

const config = useConfigStore()
const configBus = useConfigBus()
const activeTheme = await apiFetch<Theme>('/api/admin/theme')
config.setTheme(activeTheme)
const editor = ref<Theme>(cloneTheme(activeTheme))
const styleText = computed(() => varsToStyle(themeToCssVars(editor.value)))

// Branding is a profile-level property (not part of the theme), so it stays
// unchanged when themes are applied/saved/imported. Loaded from and saved to
// the admin profile.
interface AdminProfile { username: string; email: string; display_name: string; bio: string | null; location: string | null; socials: { platform: string; url: string }[]; avatar_url: string | null; cover_url: string | null; branding: Branding }
const adminProfile = await apiFetch<AdminProfile>('/api/admin/profile').catch(() => null)
const branding = reactive<Branding>({ ...DEFAULT_BRANDING, ...(adminProfile?.branding || {}) })
config.setBranding({ ...branding })
const brandingStatus = ref('')
const brandingError = ref('')
async function saveBranding() {
  brandingStatus.value = ''; brandingError.value = ''
  try {
    const updated = await apiFetch<AdminProfile>('/api/admin/profile', { method: 'PUT', body: { branding: { ...branding } } })
    Object.assign(branding, updated.branding)
    config.setBranding({ ...branding })
    brandingStatus.value = 'Branding saved.'
  } catch { brandingError.value = 'Unable to save branding.' }
}

const previewLinks = [
  { title: 'Featured collection and new releases', url: 'shop.example.com/collections/featured', icon: '🛍️', clicks: '1.2k' },
  { title: 'Book a strategy call', url: 'calendar.example.com/consultation', icon: '📅', clicks: '318' }
]
const { data: themes, refresh } = await useAsyncData('theme-presets', () => apiFetch<ThemeSummary[]>('/api/admin/themes').catch(() => []))
const status = ref('')
const error = ref('')
const showFavoritesOnly = ref(false)

// Per-group appearance. Layout, corner radii, link spacing and collapsing are
// set per group here (not globally) so every link in a group stays consistent.
// The synthetic "Ungrouped" tab edits the theme-level ungrouped link style.
const UNGROUPED = '__ungrouped__'
const { data: adminGroups, refresh: refreshGroups } = await useAsyncData('appearance-groups', () => apiFetch<AdminGroup[]>('/api/admin/groups').catch(() => []))
const selectedGroupId = ref('')
const groupStyle = reactive<GroupStyle & { collapsible: boolean }>({ ...DEFAULT_GROUP_STYLE, collapsible: true })
const groupStatus = ref('')
const groupError = ref('')
const isUngrouped = computed(() => selectedGroupId.value === UNGROUPED)
const selectedGroup = computed(() => (adminGroups.value || []).find(group => group.id === selectedGroupId.value) || null)
const groupTabs = computed(() => [...(adminGroups.value || []).map(group => ({ id: group.id, title: group.title })), { id: UNGROUPED, title: 'Ungrouped' }])

// Live preview link cards mirror the currently-edited group's style so both the
// preview and the "Link spacing" control visibly reflect edits.
const previewGroupVars = computed<Record<string, string>>(() => ({
  '--group-spacing': groupStyle.spacing || DEFAULT_GROUP_STYLE.spacing,
  '--radius-link': cssRadius(groupStyle.link_radius, 50),
  '--radius-link-icon': cssRadius(groupStyle.icon_radius, 50)
}))

function loadGroupStyle() {
  if (isUngrouped.value) {
    const style = editor.value.ungrouped || DEFAULT_GROUP_STYLE
    Object.assign(groupStyle, {
      layout: style.layout,
      link_radius: style.link_radius,
      icon_radius: style.icon_radius,
      spacing: style.spacing,
      title_align: style.title_align || 'left',
      collapsible: false
    })
    return
  }
  const group = selectedGroup.value
  const style = group?.style || DEFAULT_GROUP_STYLE
  Object.assign(groupStyle, {
    layout: style.layout,
    link_radius: style.link_radius,
    icon_radius: style.icon_radius,
    spacing: style.spacing,
    title_align: style.title_align || 'left',
    collapsible: group ? group.collapsible : true
  })
}
watch(adminGroups, groups => {
  const ids = (groups || []).map(group => group.id)
  if (selectedGroupId.value !== UNGROUPED && !ids.includes(selectedGroupId.value)) {
    selectedGroupId.value = ids[0] || UNGROUPED
  }
}, { immediate: true })
watch(selectedGroupId, loadGroupStyle, { immediate: true })
watch(adminGroups, loadGroupStyle)

async function saveGroupAppearance() {
  groupStatus.value = ''; groupError.value = ''
  const style: GroupStyle = { layout: groupStyle.layout, link_radius: groupStyle.link_radius, icon_radius: groupStyle.icon_radius, spacing: groupStyle.spacing, title_align: groupStyle.title_align }
  if (isUngrouped.value) {
    editor.value.ungrouped = { ...style }
    try {
      const applied = await apiFetch<Theme>('/api/admin/theme', { method: 'PUT', body: { config: editor.value } })
      editor.value = cloneTheme(applied)
      syncConfig(applied)
      await refresh()
      groupStatus.value = 'Saved ungrouped link appearance.'
    } catch { groupError.value = 'Unable to save ungrouped appearance.' }
    return
  }
  const group = selectedGroup.value
  if (!group) return
  try {
    await apiFetch(`/api/admin/groups/${group.id}`, {
      method: 'PUT',
      body: {
        title: group.title,
        description: group.description,
        collapsible: groupStyle.collapsible,
        style
      }
    })
    await refreshGroups()
    groupStatus.value = `Saved appearance for ${group.title}.`
  } catch { groupError.value = 'Unable to save group appearance.' }
}
const googleFontsText = computed({ get: () => editor.value.fonts.google_fonts.join(', '), set: value => { editor.value.fonts.google_fonts = value.split(',').map(item => item.trim()).filter(Boolean) } })
const backgroundRadiusPercent = computed({
  get: () => {
    const value = Number.parseFloat(editor.value.radius.background)
    return Number.isFinite(value) ? Math.min(20, Math.max(0, value)) : 20
  },
  set: value => {
    const parsed = Number(value)
    editor.value.radius.background = `${Number.isFinite(parsed) ? Math.min(20, Math.max(0, parsed)) : 20}%`
  }
})
const colorLabels: Record<keyof Theme['colors'], string> = {
  background: 'Background',
  surface: 'Surface',
  surface_alt: 'Alternate surface',
  text: 'Text',
  text_muted: 'Muted text',
  primary: 'Primary',
  primary_contrast: 'Primary contrast',
  accent: 'Accent',
  border: 'Border'
}
const colorEntries = computed(() => Object.entries(editor.value.colors) as [keyof Theme['colors'], string][])
const displayedThemes = computed(() => showFavoritesOnly.value ? (themes.value || []).filter(theme => theme.is_favorite) : (themes.value || []))
const activeMyTheme = computed(() => (themes.value || []).find(theme => theme.is_active) || null)

function isPresetActive(preset: Theme) { return !activeMyTheme.value && config.theme.name === preset.name }
function presetSwatch(theme: Theme) {
  const background = theme.background.type === 'gradient' ? theme.background.gradient : theme.background.value
  return { background, primary: theme.colors.primary, accent: theme.colors.accent }
}

function syncConfig(theme: Theme) {
  const applied = cloneTheme(theme)
  config.setTheme(applied)
  configBus.emit(applied)
}

async function applyPreset(preset: Theme) {
  status.value = ''; error.value = ''
  try {
    const applied = await apiFetch<Theme>('/api/admin/presets/apply', { method: 'POST', body: { name: preset.name, config: preset } })
    editor.value = cloneTheme(applied)
    syncConfig(applied)
    await refresh()
    status.value = `Applied ${preset.name}.`
  } catch { error.value = 'Unable to apply preset.' }
}

async function saveActive() {
  status.value = ''; error.value = ''
  try {
    const applied = await apiFetch<Theme>('/api/admin/theme', { method: 'PUT', body: { config: editor.value } })
    editor.value = cloneTheme(applied)
    syncConfig(applied)
    status.value = 'Theme saved.'
    await refresh()
  } catch { error.value = 'Unable to save theme.' }
}
async function createPreset(theme = editor.value) {
  const name = (theme.name || '').trim() || 'Custom theme'
  await apiFetch('/api/admin/themes', { method: 'POST', body: { name, config: theme } })
  await refresh()
}
async function loadTheme(id: string) {
  const detail = await apiFetch<ThemeDetail>(`/api/admin/themes/${id}`)
  editor.value = cloneTheme(detail.config)
}
async function activateTheme(id: string) {
  const applied = await apiFetch<Theme>(`/api/admin/themes/${id}/activate`, { method: 'POST' })
  editor.value = cloneTheme(applied)
  syncConfig(applied)
  await refresh()
}
async function deleteTheme(id: string) { await apiFetch(`/api/admin/themes/${id}`, { method: 'DELETE' }); await refresh() }
async function toggleFavorite(theme: ThemeSummary) {
  await apiFetch<ThemeSummary>(`/api/admin/themes/${theme.id}/favorite`, { method: 'POST' })
  await refresh()
}
async function usePreset(theme: Theme) { editor.value = cloneTheme(theme) }
async function exportTheme() {
  const blob = await apiFetch<Blob>('/api/admin/theme/export', { responseType: 'blob' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a'); a.href = url; a.download = `${editor.value.name || 'theme'}.json`; a.click(); URL.revokeObjectURL(url)
}
async function downloadTheme(theme: ThemeSummary) {
  const blob = await apiFetch<Blob>(`/api/admin/themes/${theme.id}/export`, { responseType: 'blob' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a'); a.href = url; a.download = `${theme.name || 'theme'}.json`; a.click(); URL.revokeObjectURL(url)
}
async function importTheme(event: Event, activate = false) {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file) return
  status.value = ''; error.value = ''
  try {
    const body = new FormData(); body.append('file', file); body.append('activate', activate ? 'true' : 'false')
    const theme = await apiFetch<ThemeSummary>('/api/admin/themes/import', { method: 'POST', body })
    await refresh()
    if (activate) { await loadTheme(theme.id); syncConfig(editor.value) }
    status.value = activate ? 'Theme imported and applied.' : 'Theme imported to library.'
  } catch {
    error.value = 'Unable to import theme.'
  } finally {
    ;(event.target as HTMLInputElement).value = ''
  }
}

const uploadingFavicon = ref(false)
const FAVICON_TYPES = ['image/x-icon', 'image/vnd.microsoft.icon', 'image/ico', 'image/icon', 'image/x-ico']
function randomizeBackgroundShapes() {
  editor.value.background.shapes.seed = Math.floor(Math.random() * 1_000_000)
}
async function uploadFavicon(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  status.value = ''; error.value = ''
  // Enforce .ico client-side too, for immediate feedback (the API also enforces it).
  if (!/\.ico$/i.test(file.name) && !FAVICON_TYPES.includes(file.type)) {
    error.value = 'Favicon must be a .ico file.'
    input.value = ''
    return
  }
  uploadingFavicon.value = true
  try {
    const body = new FormData(); body.append('file', file)
    const res = await apiFetch<{ url: string }>('/api/admin/uploads/favicon', { method: 'POST', body })
    branding.favicon = res.url
    status.value = 'Favicon uploaded. Save branding to apply it.'
  } catch {
    error.value = 'Unable to upload favicon. Only .ico files are allowed.'
  } finally {
    uploadingFavicon.value = false
    input.value = ''
  }
}
</script>

<template>
  <main class="admin-layout">
    <AdminNav />
    <p class="desktop-note">
      <strong>Tap a theme to apply it instantly.</strong>
      Color, font, and layout editing, plus the live preview, are available on larger screens. Open this page on a desktop for full customization.
    </p>
    <div class="appearance-grid">
      <aside class="admin-card presets">
        <h2>Presets</h2>
        <p class="section-hint">Select a built-in theme to apply it. Use its menu to edit a copy.</p>
        <div class="theme-gallery">
          <ThemeCard
            v-for="preset in PRESET_THEMES"
            :key="preset.name"
            :title="preset.name"
            :swatch="presetSwatch(preset)"
            :active="isPresetActive(preset)"
            :actions="['edit']"
            @apply="applyPreset(preset)"
            @edit="usePreset(preset)"
          />
        </div>

        <div class="my-themes-head">
          <h3>My themes</h3>
          <label class="fav-filter"><input v-model="showFavoritesOnly" type="checkbox"> Favorites</label>
        </div>
        <div v-if="displayedThemes.length" class="theme-gallery">
          <ThemeCard
            v-for="theme in displayedThemes"
            :key="theme.id"
            :title="theme.name"
            :swatch="theme.swatch"
            :active="theme.is_active"
            :owner="theme.owner"
            :favorite="theme.is_favorite"
            :downloads="theme.download_count"
            :actions="['edit', 'download', 'favorite', 'delete']"
            @apply="activateTheme(theme.id)"
            @edit="loadTheme(theme.id)"
            @download="downloadTheme(theme)"
            @favorite="toggleFavorite(theme)"
            @delete="deleteTheme(theme.id)"
          />
        </div>
        <p v-else class="section-hint empty">No saved themes yet. Edit a preset, customize it, then save it as a new theme.</p>

        <div class="editor-actions">
          <label class="icon-btn import-btn" title="Import a theme file" aria-label="Import a theme file">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="17 8 12 3 7 8"></polyline><line x1="12" y1="3" x2="12" y2="15"></line></svg>
            <input hidden type="file" accept="application/json,.json" @change="importTheme($event, false)">
          </label>
          <span class="io-hint">Import theme</span>
        </div>
        <p v-if="status" class="success">{{ status }}</p><p v-if="error" class="error">{{ error }}</p>
      </aside>

      <section class="admin-card editor form-grid">
        <h2>Theme editor</h2>
        <label class="form-row">Theme name<input v-model="editor.name"></label>

        <fieldset>
          <legend>Colors</legend>
          <div class="color-grid">
            <label v-for="([key, value]) in colorEntries" :key="key" class="form-row color-row">
              <span>{{ colorLabels[key] }}</span>
              <span class="color-controls">
                <input v-if="value.startsWith('#')" v-model="editor.colors[key]" type="color" :aria-label="`${colorLabels[key]} color picker`">
                <input v-model="editor.colors[key]" :aria-label="`${colorLabels[key]} value`">
              </span>
            </label>
          </div>
        </fieldset>

        <fieldset><legend>Typography</legend><div class="two"><label class="form-row">Heading font<input v-model="editor.fonts.heading"></label><label class="form-row">Body font<input v-model="editor.fonts.body"></label></div><label class="form-row">Google Fonts<input v-model="googleFontsText" placeholder="Inter:wght@400;600;800"></label></fieldset>

        <fieldset>
          <legend>Page layout</legend>
          <label class="form-row">Content width<input v-model="editor.layout.max_width"></label>
        </fieldset>

        <fieldset>
          <legend>Links</legend>
          <p class="fieldset-hint">Card style shared by every link. Layout, corners, spacing and collapsing are set per group below.</p>
          <div class="two">
            <label class="form-row">Style<select v-model="editor.button.variant"><option value="solid">Solid</option><option value="outline">Outline</option><option value="glass">Glass</option><option value="soft">Soft</option></select></label>
            <label class="form-row">Shadow<input v-model="editor.button.shadow"></label>
          </div>
          <div class="toggles section-toggles">
            <label class="check"><input v-model="editor.button.hover_lift" type="checkbox"> Hover lift</label>
            <label class="check"><input v-model="editor.features.show_click_count" type="checkbox"> Show click count</label>
          </div>
        </fieldset>

        <fieldset>
          <legend>Group appearance</legend>
          <p class="fieldset-hint">Pick a group, then set how its links look. These apply to every link in that group. Use “Ungrouped” to style links that aren’t in any group.</p>
          <div class="group-picker" role="tablist" aria-label="Groups">
            <button
              v-for="tab in groupTabs"
              :key="tab.id"
              type="button"
              role="tab"
              class="group-chip"
              :class="{ active: tab.id === selectedGroupId, ungrouped: tab.id === UNGROUPED }"
              :aria-selected="tab.id === selectedGroupId"
              @click="selectedGroupId = tab.id"
            >{{ tab.title }}</button>
          </div>
          <div class="two">
            <label class="form-row">Layout<select v-model="groupStyle.layout"><option value="list">List</option><option value="grid">Grid</option></select></label>
            <label class="form-row">Link spacing<input v-model="groupStyle.spacing" placeholder="12px"></label>
            <label class="form-row radius-row"><span class="lbl">Link corners (%) <small>Roundness of each link card</small></span><input v-model="groupStyle.link_radius" inputmode="decimal" placeholder="22%"></label>
            <label class="form-row radius-row"><span class="lbl">Link icon corners (%) <small>Images and icon badges inside links</small></span><input v-model="groupStyle.icon_radius" inputmode="decimal" placeholder="50%"></label>
            <label v-if="!isUngrouped" class="form-row">Title alignment<select v-model="groupStyle.title_align"><option value="left">Left</option><option value="center">Center</option></select></label>
          </div>
          <div v-if="!isUngrouped" class="toggles section-toggles">
            <label class="check"><input v-model="groupStyle.collapsible" type="checkbox"> Collapsible</label>
          </div>
          <div class="group-appearance-actions">
            <button class="btn primary" type="button" @click="saveGroupAppearance">{{ isUngrouped ? 'Save ungrouped appearance' : 'Save group appearance' }}</button>
            <p v-if="groupStatus" class="success">{{ groupStatus }}</p><p v-if="groupError" class="error">{{ groupError }}</p>
          </div>
        </fieldset>

        <fieldset>
          <legend>Glass</legend>
          <p class="fieldset-hint">Translucent panels with a blurred background. Applies to glass link cards and, when enabled, the whole UI — panels stacked on panels blur progressively.</p>
          <div class="toggles section-toggles">
            <label class="check"><input v-model="editor.effects.glass" type="checkbox"> Glass UI (translucent panels)</label>
          </div>
          <div class="two">
            <label class="form-row">Glass opacity (%)<input v-model.number="editor.effects.glass_opacity" type="number" min="0" max="100" step="1"></label>
            <label class="form-row">Glass blur (px)<input v-model.number="editor.effects.glass_blur" type="number" min="0" max="60" step="1"></label>
          </div>
        </fieldset>

        <fieldset>
          <legend>Profile</legend>
          <p class="fieldset-hint">Profile picture, alignment, social icons, and profile analytics.</p>
          <div class="two">
            <label class="form-row">Alignment<select v-model="editor.layout.align"><option value="left">Left</option><option value="center">Center</option><option value="right">Right</option></select></label>
            <label class="form-row radius-row"><span class="lbl">Profile picture corners (%) <small>50% produces a circle</small></span><input v-model="editor.radius.avatar" inputmode="decimal" placeholder="50%"></label>
            <label class="form-row radius-row"><span class="lbl">Social icon corners (%) <small>Icons shown below the profile bio</small></span><input v-model="editor.radius.social_icon" inputmode="decimal" placeholder="50%"></label>
          </div>
          <div class="toggles section-toggles">
            <label class="check"><input v-model="editor.features.show_view_count" type="checkbox"> Show view count</label>
          </div>
        </fieldset>

        <fieldset>
          <legend>Background &amp; cover</legend>
          <p class="fieldset-hint">Background, cover presentation, surfaces, and decorative accent shapes.</p>
          <div class="two">
            <label class="form-row">Type<select v-model="editor.background.type"><option value="solid">Solid</option><option value="gradient">Gradient</option><option value="image">Image</option></select></label>
            <label class="form-row">Base color<input v-model="editor.background.value"></label>
          </div>
          <label v-if="editor.background.type === 'gradient'" class="form-row">Gradient<input v-model="editor.background.gradient"></label>
          <label v-if="editor.background.type === 'image'" class="form-row">Image URL<input v-model="editor.background.image"></label>
          <div class="two">
            <label class="form-row">Overlay<input v-model="editor.background.overlay"></label>
            <label class="form-row">Cover height<input v-model="editor.layout.cover_height"></label>
            <label class="form-row radius-row"><span class="lbl">Background &amp; panel corners (%) <small>0–20% for preview, cards, menus, and surface panels</small></span><input v-model.number="backgroundRadiusPercent" type="number" min="0" max="20" step="1"></label>
          </div>
          <div class="toggles section-toggles">
            <label class="check"><input v-model="editor.features.show_cover_photo" type="checkbox"> Show cover photo</label>
            <label class="check"><input v-model="editor.effects.cover_fade" type="checkbox"> Blend cover edge</label>
            <label class="check"><input v-model="editor.effects.cover_parallax" type="checkbox"> Cover parallax</label>
          </div>
          <div class="shape-settings">
            <label class="check"><input v-model="editor.background.shapes.enabled" type="checkbox"> Blurred accent shapes</label>
            <p class="fieldset-hint">Uses the primary and accent colors. The saved seed keeps the random layout stable between page loads.</p>
            <div class="three">
              <label class="form-row">Shape count<input v-model.number="editor.background.shapes.count" type="number" min="0" max="12"></label>
              <label class="form-row">Opacity<input v-model.number="editor.background.shapes.opacity" type="number" min="0" max="1" step="0.05"></label>
              <label class="form-row">Blur (px)<input v-model.number="editor.background.shapes.blur" type="number" min="0" max="240"></label>
              <label class="form-row">Minimum size (px)<input v-model.number="editor.background.shapes.min_size" type="number" min="40" max="1200"></label>
              <label class="form-row">Maximum size (px)<input v-model.number="editor.background.shapes.max_size" type="number" min="40" max="1600"></label>
              <label class="form-row">Shape layout seed<input v-model.number="editor.background.shapes.seed" type="number"></label>
            </div>
            <button class="btn shape-shuffle" type="button" @click="randomizeBackgroundShapes">Shuffle shape layout</button>
          </div>
        </fieldset>

        <fieldset><legend>Branding</legend><p class="fieldset-hint">Site identity that stays the same when you switch themes. Saved to your profile, not the theme.</p><div class="two"><label class="form-row">Site name<input v-model="branding.site_name" maxlength="256"></label><label class="form-row">Logo URL<input v-model="branding.logo"></label><div class="form-row favicon-field">Favicon<div class="input-group"><input v-model="branding.favicon" placeholder="/favicon.ico"><label class="upload-btn" :class="{ busy: uploadingFavicon }" :title="uploadingFavicon ? 'Uploading…' : 'Upload a .ico file'" aria-label="Upload favicon (.ico)"><svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="17 8 12 3 7 8"></polyline><line x1="12" y1="3" x2="12" y2="15"></line></svg><input hidden type="file" accept=".ico,image/x-icon,image/vnd.microsoft.icon" @change="uploadFavicon"></label></div></div><label class="form-row">Footer text<input v-model="branding.footer_text" maxlength="256"></label></div><p class="fieldset-hint">Footer text supports HTML (e.g. a link); links use your theme colors.</p><div class="toggles section-toggles branding-toggles"><label class="check"><input v-model="branding.show_footer" type="checkbox"> Show footer</label></div><div class="group-appearance-actions"><button class="btn primary" type="button" @click="saveBranding">Save branding</button><p v-if="brandingStatus" class="success">{{ brandingStatus }}</p><p v-if="brandingError" class="error">{{ brandingError }}</p></div></fieldset>
      </section>

      <div class="preview-col">
        <section class="preview" :style="styleText">
          <ThemeBackgroundShapes contained />
          <div class="preview-content">
            <p class="preview-label">Live preview · updates as you edit</p>
            <div class="phone">
              <div v-if="editor.features.show_cover_photo" class="mini-cover"></div>
              <div class="mini-body" :class="[`align-${editor.layout.align}`, { 'has-cover': editor.features.show_cover_photo }]">
                <div class="mini-avatar"></div>
                <h2>{{ editor.name || 'Your name' }}</h2>
                <p class="mini-tag">Welcome to my links</p>
                <p class="mini-bio">This preview reflects every setting live.</p>
                <div class="mini-socials"><span></span><span></span><span></span></div>
                <p v-if="editor.features.show_view_count" class="mini-meta">1,248 views</p>
                <div class="mini-links" :class="`style-${groupStyle.layout}`" :style="previewGroupVars">
                  <a
                    v-for="link in previewLinks"
                    :key="link.title"
                    class="mini-link"
                    :class="[`variant-${editor.button.variant}`, { lift: editor.button.hover_lift }]"
                  >
                    <span class="mini-icon">{{ link.icon }}</span>
                    <span class="mini-content"><strong>{{ link.title }}</strong><small>{{ link.url }}</small></span>
                    <span v-if="editor.features.show_click_count" class="mini-clicks">{{ link.clicks }}</span>
                  </a>
                </div>
                <p v-if="branding.show_footer" class="mini-footer" v-html="branding.footer_text || 'Made with SocialLink'"></p>
              </div>
            </div>
          </div>
        </section>

        <div class="preview-actions">
          <div class="action-row">
            <button class="icon-btn" title="Save as a new theme" aria-label="Save as a new theme" @click="createPreset()">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
            </button>
            <button class="icon-btn primary" title="Save &amp; apply to your live page" aria-label="Save and apply to your live page" @click="saveActive">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"></polyline></svg>
            </button>
            <button class="icon-btn" title="Export theme as JSON" aria-label="Export theme as JSON" @click="exportTheme">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>
            </button>
          </div>
        </div>
      </div>
    </div>
  </main>
</template>

<style scoped>
.desktop-note { display: none; }
.favicon-field .input-group { display: flex; align-items: stretch; }
.favicon-field .input-group input { flex: 1 1 auto; min-width: 0; border-top-right-radius: 0; border-bottom-right-radius: 0; }
.favicon-field .upload-btn { flex: none; display: inline-flex; align-items: center; justify-content: center; width: 44px; height: 44px; color: var(--color-text-muted); background: var(--color-surface-alt); border: 1px solid var(--color-border); border-left: none; border-top-right-radius: var(--radius-input); border-bottom-right-radius: var(--radius-input); cursor: pointer; transition: color .15s ease, background .15s ease; }
.favicon-field .upload-btn:hover { color: var(--color-text); background: color-mix(in srgb, var(--color-primary) 12%, var(--color-surface-alt)); }
.favicon-field .upload-btn.busy { opacity: .6; pointer-events: none; }
.appearance-grid { display: grid; grid-template-columns: 280px minmax(0, 1fr) 340px; gap: 18px; align-items: start; }
h2, h3, legend { font-family: var(--font-heading); }
.presets { display: grid; gap: 12px; }
.section-hint { color: var(--color-text-muted); font-size: .8rem; margin: 0; }
.section-hint.empty { padding: 14px; border: 1px dashed var(--color-border); border-radius: var(--radius-background); text-align: center; }
.theme-gallery { display: grid; grid-template-columns: repeat(auto-fill, minmax(128px, 1fr)); gap: 10px; }
.my-themes-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: 4px; }
.my-themes-head h3 { margin: 0; }
.fav-filter { display: flex; align-items: center; gap: 6px; color: var(--color-text-muted); font-size: .82rem; }
.editor-actions { display: flex; align-items: center; gap: 10px; margin-top: 4px; padding-top: 12px; border-top: 1px solid var(--color-border); }
.io-hint { color: var(--color-text-muted); font-size: .82rem; }
.import-btn input { display: none; }
.icon-btn { display: inline-grid; place-items: center; width: 44px; height: 44px; flex: none; border: 1px solid var(--color-border); border-radius: var(--radius-button); background: color-mix(in srgb, var(--color-surface-alt) 85%, transparent); color: var(--color-text); cursor: pointer; transition: transform .18s ease, border-color .18s ease, background .18s ease; }
.icon-btn:hover { transform: translateY(-1px); border-color: color-mix(in srgb, var(--color-primary) 50%, var(--color-border)); }
.icon-btn.primary { background: var(--color-primary); color: var(--color-primary-contrast); border-color: transparent; }
.icon-btn svg { width: 20px; height: 20px; }
fieldset { border: 1px solid var(--color-border); border-radius: var(--radius-background); padding: 16px; }
.editor input:not([type='checkbox']):not([type='file']), .editor select { height: 44px; min-height: 44px; }
.fieldset-hint { margin: 0 0 12px; color: var(--color-text-muted); font-size: .8rem; }
.shape-settings { display: grid; gap: 12px; margin-top: 16px; padding-top: 16px; border-top: 1px solid var(--color-border); }
.shape-settings .fieldset-hint { margin: -4px 0 0; }
.shape-shuffle { justify-self: start; }
.radius-row .lbl { display: flex; flex-direction: column; gap: 1px; color: var(--color-text); font-size: .9rem; }
.radius-row .lbl small { color: var(--color-text-muted); font-size: .74rem; font-weight: 400; line-height: 1.3; }
.color-grid, .two, .three, .four { display: grid; gap: 12px; }
.color-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.two { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.three { grid-template-columns: repeat(3, minmax(0, 1fr)); }
.four { grid-template-columns: repeat(4, minmax(0, 1fr)); }
.color-controls { display: grid; grid-template-columns: 44px minmax(0, 1fr); gap: 8px; }
.color-controls > input:only-child { grid-column: 1 / -1; }
.color-controls input[type='color'] { width: 44px; min-width: 44px; padding: 4px; }
.toggles { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
.section-toggles { margin-top: 14px; }
.group-picker { display: flex; gap: 8px; overflow-x: auto; padding-bottom: 6px; margin-bottom: 14px; scrollbar-width: thin; -webkit-overflow-scrolling: touch; }
.group-chip { flex: 0 0 auto; min-height: 40px; padding: 8px 14px; border: 1px solid var(--color-border); border-radius: 999px; background: var(--color-surface-alt); color: var(--color-text); font-size: .85rem; cursor: pointer; white-space: nowrap; transition: border-color .15s ease, background .15s ease, color .15s ease; }
.group-chip:hover { border-color: color-mix(in srgb, var(--color-primary) 45%, var(--color-border)); }
.group-chip.active { background: var(--color-primary); color: var(--color-primary-contrast); border-color: transparent; }
.group-chip.ungrouped { border-style: dashed; }
.group-chip.ungrouped.active { border-style: solid; }
.group-appearance-actions { display: flex; flex-direction: column; gap: 10px; margin-top: 14px; }
.group-appearance-actions .btn { align-self: start; }
.branding-toggles { grid-template-columns: 1fr; }
.check { min-height: 44px; display: flex; align-items: center; gap: 10px; margin: 0; padding: 10px 12px; border: 1px solid var(--color-border); border-radius: var(--radius-input); background: var(--color-surface-alt); color: var(--color-text); font-size: .88rem; cursor: pointer; user-select: none; transition: border-color .15s ease, background .15s ease; }
.check:hover { border-color: color-mix(in srgb, var(--color-primary) 45%, var(--color-border)); }
.check input[type='checkbox'] { width: 18px; height: 18px; flex: none; margin: 0; accent-color: var(--color-primary); cursor: pointer; }
.check:has(input:checked) { border-color: var(--color-primary); background: color-mix(in srgb, var(--color-primary) 12%, var(--color-surface)); }
.import-btn input { display: none; }
.preview-col { position: sticky; top: 18px; align-self: start; display: grid; gap: 12px; }
.preview { position: relative; isolation: isolate; overflow: hidden; border-radius: var(--radius-background); padding: 14px; background: var(--background-image), var(--background-gradient), var(--background-value); color: var(--color-text); border: 1px solid var(--color-border); }
.preview-content { position: relative; z-index: 1; }
.preview-actions { display: grid; gap: 10px; }
.action-row { display: flex; gap: 8px; justify-content: flex-end; }
.preview-label { margin: 0 0 10px; font-size: .72rem; letter-spacing: .04em; color: var(--color-text-muted); }
.phone { min-height: 540px; border-radius: var(--radius-background); padding: 16px; background: color-mix(in srgb, var(--color-surface) 86%, transparent); border: 1px solid var(--color-border); box-shadow: var(--button-shadow); overflow: hidden; }
.mini-cover {
  height: clamp(80px, calc(var(--cover-height) * .4), 180px);
  border-radius: var(--radius-background);
  background: var(--background-gradient);
  -webkit-mask-image: linear-gradient(to bottom, #000 0%, #000 52%, rgba(0,0,0,.92) 62%, transparent 100%);
  mask-image: linear-gradient(to bottom, #000 0%, #000 52%, rgba(0,0,0,.92) 62%, transparent 100%);
}
.mini-body { display: grid; justify-items: center; text-align: center; gap: 6px; }
.mini-body.align-left { justify-items: start; text-align: left; }
.mini-body.align-right { justify-items: end; text-align: right; }
.mini-body.has-cover .mini-avatar { margin-top: calc(min(var(--avatar-size), 84px) * -0.42); }
.mini-avatar { width: min(var(--avatar-size), 84px); height: min(var(--avatar-size), 84px); border-radius: var(--radius-avatar); background: linear-gradient(135deg, var(--color-accent), var(--color-primary)); border: 4px solid var(--color-background); }
.preview h2 { margin: 4px 0 0; font: 800 1.5rem var(--font-heading); letter-spacing: -.03em; }
.mini-tag { margin: 0; color: var(--color-accent); font-weight: 700; font-size: .82rem; }
.mini-bio { margin: 0; color: var(--color-text-muted); font-size: .78rem; }
.mini-socials { display: flex; gap: 8px; margin-top: 2px; }
.mini-socials span { width: 30px; height: 30px; border-radius: var(--radius-social-icon); border: 1px solid var(--color-border); background: color-mix(in srgb, var(--color-surface) 78%, transparent); }
.mini-meta { margin: 2px 0 0; color: var(--color-text-muted); font-size: .74rem; }
.mini-links { display: grid; gap: var(--group-spacing, 12px); width: 100%; margin-top: 10px; }
.mini-links.style-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.mini-link { min-width: 0; max-width: 100%; display: grid; grid-template-columns: auto minmax(0, 1fr) auto; grid-template-areas: 'icon content clicks'; gap: 10px; align-items: center; padding: 11px 13px; border-radius: var(--radius-link); border: 1px solid var(--color-border); background: var(--color-surface); color: var(--color-text); box-shadow: var(--button-shadow); transition: transform .2s ease; }
.mini-links.style-grid .mini-link { grid-template-columns: auto minmax(0, 1fr); grid-template-areas: 'icon content' 'icon clicks'; gap: 4px 8px; align-items: start; padding: 10px; }
.mini-link.lift:hover { transform: translateY(-3px); }
.mini-link.variant-glass { background: color-mix(in srgb, var(--color-surface) var(--glass-opacity, 72%), transparent); backdrop-filter: blur(var(--glass-blur, 18px)); -webkit-backdrop-filter: blur(var(--glass-blur, 18px)); }
.mini-link.variant-solid { background: var(--color-primary); color: var(--color-primary-contrast); }
.mini-link.variant-outline { background: transparent; border-color: var(--color-primary); }
.mini-link.variant-soft { background: color-mix(in srgb, var(--color-primary) 16%, var(--color-surface)); }
.mini-icon { grid-area: icon; display: grid; place-items: center; width: 30px; height: 30px; border-radius: var(--radius-link-icon); background: color-mix(in srgb, var(--color-accent) 16%, transparent); font-size: .82rem; }
.mini-content { grid-area: content; display: grid; gap: 1px; min-width: 0; max-width: 100%; overflow: hidden; text-align: left; }
.mini-content strong { font: 700 .9rem var(--font-heading); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.mini-content small { min-width: 0; max-width: 100%; overflow: hidden; color: color-mix(in srgb, currentColor 62%, transparent); font-size: .72rem; text-overflow: ellipsis; white-space: nowrap; }
.mini-clicks { grid-area: clicks; min-width: 0; justify-self: end; font-size: .74rem; opacity: .75; overflow-wrap: anywhere; }
.mini-links.style-grid .mini-clicks { justify-self: start; }
.mini-footer { margin: 12px 0 0; width: 100%; text-align: center; color: var(--color-text-muted); font-size: .72rem; }
.mini-footer :deep(a) { color: var(--color-accent); text-decoration: none; font-weight: 600; }
@media (max-width: 1180px) { .appearance-grid { grid-template-columns: 1fr; } .preview-col { position: static; } }
@media (max-width: 760px) {
  .color-grid, .two, .three, .four, .toggles { grid-template-columns: 1fr; }
  /* Phones get a focused theme picker only: applying a theme is live, while
     the editor + preview (which sit far below the fold) are desktop-only. */
  .desktop-note { display: block; margin: 0 0 4px; padding: 12px 14px; border: 1px solid var(--color-border); border-radius: var(--radius-background); background: color-mix(in srgb, var(--color-primary) 8%, var(--color-surface)); color: var(--color-text); font-size: .84rem; line-height: 1.4; }
  .desktop-note strong { display: block; margin-bottom: 2px; }
  .editor, .preview-col, .editor-actions { display: none; }
  .theme-gallery { grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); }
}
</style>
