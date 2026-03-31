<script setup lang="ts">
import type { AdminGroup, AdminLink } from '~/types/social'
import { DEFAULT_GROUP_STYLE } from '~/types/social'

definePageMeta({ middleware: 'auth' })
const { data: groups, refresh: refreshGroups } = await useAsyncData('admin-groups', () => apiFetch<AdminGroup[]>('/api/admin/groups'))
const { data: links, refresh: refreshLinks } = await useAsyncData('admin-links', () => apiFetch<AdminLink[]>('/api/admin/links'))
const linkForm = reactive({ group_id: '', title: '', url: '', description: '', icon: '', icon_image: '', icon_font: '', expires_at: '' })
const iconType = ref<'none' | 'emoji' | 'image' | 'icon'>('none')
const editingLink = ref<AdminLink | null>(null)
const uploadingImage = ref(false)
const linkError = ref('')
const groupError = ref('')
const newGroup = reactive({ title: '', description: '', collapsible: true, layout: 'list', corners: 'rounded', icon: 'round' })
const editingGroupId = ref<string | null>(null)
const editGroupForm = reactive({ title: '', description: '', collapsible: true, layout: 'list', corners: 'rounded', icon: 'round' })

const groupOptions = computed<AdminGroup[]>(() => [{ id: '', title: 'Ungrouped', description: '', collapsible: false, is_active: true, sort_order: -1, style: DEFAULT_GROUP_STYLE }, ...(groups.value || [])])
const linksByGroup = (group_id: string | null) => (links.value || []).filter(link => (link.group_id || '') === (group_id || '')).sort((a, b) => a.sort_order - b.sort_order)
async function reload() { await Promise.all([refreshGroups(), refreshLinks()]) }
async function createGroup() {
  if (!newGroup.title.trim()) return
  await apiFetch('/api/admin/groups', { method: 'POST', body: { title: newGroup.title.trim(), description: newGroup.description, collapsible: newGroup.collapsible, style: { layout: newGroup.layout, corners: newGroup.corners, icon: newGroup.icon } } })
  Object.assign(newGroup, { title: '', description: '', collapsible: true, layout: 'list', corners: 'rounded', icon: 'round' }); await refreshGroups()
}
function startEditGroup(group: AdminGroup) { editingGroupId.value = group.id; const s = group.style || DEFAULT_GROUP_STYLE; Object.assign(editGroupForm, { title: group.title, description: group.description || '', collapsible: group.collapsible, layout: s.layout, corners: s.corners, icon: s.icon }) }
function cancelEditGroup() { editingGroupId.value = null }
async function saveEditGroup(group: AdminGroup) {
  if (!editGroupForm.title.trim()) return
  await apiFetch(`/api/admin/groups/${group.id}`, { method: 'PUT', body: { title: editGroupForm.title.trim(), description: editGroupForm.description, collapsible: editGroupForm.collapsible, style: { layout: editGroupForm.layout, corners: editGroupForm.corners, icon: editGroupForm.icon } } })
  editingGroupId.value = null; await refreshGroups()
}
async function deleteGroup(id: string) { await apiFetch(`/api/admin/groups/${id}`, { method: 'DELETE' }); await reload() }
function iconTypeOf(link: AdminLink): 'none' | 'emoji' | 'image' | 'icon' {
  if (link.icon_image) return 'image'
  if (link.icon) return 'emoji'
  if (link.icon_font) return 'icon'
  return 'none'
}
function setIconType(type: 'none' | 'emoji' | 'image' | 'icon') {
  iconType.value = type
  if (type !== 'emoji') linkForm.icon = ''
  if (type !== 'image') linkForm.icon_image = ''
  if (type !== 'icon') linkForm.icon_font = ''
}
function resetLinkForm() { Object.assign(linkForm, { group_id: '', title: '', url: '', description: '', icon: '', icon_image: '', icon_font: '', expires_at: '' }); iconType.value = 'none'; editingLink.value = null; linkError.value = '' }
function editLink(link: AdminLink) { linkError.value = ''; editingLink.value = link; iconType.value = iconTypeOf(link); Object.assign(linkForm, { group_id: link.group_id || '', title: link.title, url: link.url, description: link.description, icon: link.icon, icon_image: link.icon_image || '', icon_font: link.icon_font || '', expires_at: link.expires_at ? link.expires_at.slice(0, 16) : '' }) }
function validateLinkUrl(raw: string): { ok: true; url: string } | { ok: false; error: string } {
  const t = (raw || '').trim()
  if (!t) return { ok: false, error: 'Add a URL for this link.' }
  const candidate = /^[a-z][a-z0-9+.-]*:\/\//i.test(t) ? t : `https://${t}`
  let u: URL
  try { u = new URL(candidate) } catch { return { ok: false, error: 'Enter a valid URL, e.g. https://example.com' } }
  if (u.protocol !== 'https:') return { ok: false, error: 'Links must use https://' }
  if (!u.hostname.includes('.')) return { ok: false, error: 'Enter a valid domain, e.g. https://example.com' }
  return { ok: true, url: candidate }
}
async function saveLink() {
  linkError.value = ''
  if (!linkForm.title.trim()) { linkError.value = 'Add a title for this link.'; return }
  const res = validateLinkUrl(linkForm.url)
  if (!res.ok) { linkError.value = res.error; return }
  linkForm.url = res.url
  const body = { ...linkForm, title: linkForm.title.trim(), group_id: linkForm.group_id || null, icon: iconType.value === 'emoji' ? linkForm.icon : '', icon_image: iconType.value === 'image' ? linkForm.icon_image : '', icon_font: iconType.value === 'icon' ? linkForm.icon_font.trim() : '', expires_at: linkForm.expires_at ? new Date(linkForm.expires_at).toISOString() : null }
  if (editingLink.value) await apiFetch(`/api/admin/links/${editingLink.value.id}`, { method: 'PUT', body })
  else await apiFetch('/api/admin/links', { method: 'POST', body })
  resetLinkForm(); await refreshLinks()
}
async function uploadLinkImage(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file) return
  uploadingImage.value = true
  linkError.value = ''
  try {
    const body = new FormData(); body.append('file', file)
    const res = await apiFetch<{ url: string }>('/api/admin/uploads', { method: 'POST', body })
    linkForm.icon_image = res.url
    setIconType('image')
  } catch {
    linkError.value = 'Could not upload that image. Use a PNG, JPEG, WEBP or GIF file.'
  } finally {
    uploadingImage.value = false
    ;(event.target as HTMLInputElement).value = ''
  }
}
function clearLinkImage() { linkForm.icon_image = '' }
async function deleteLink(id: string) { await apiFetch(`/api/admin/links/${id}`, { method: 'DELETE' }); await refreshLinks() }
async function move(list: AdminLink[], index: number, dir: -1 | 1) {
  const next = [...list]; const target = index + dir
  if (target < 0 || target >= next.length) return
  ;[next[index], next[target]] = [next[target], next[index]]
  const orderMap = new Map(next.map((item, i) => [item.id, i]))
  const previous = links.value
  links.value = (links.value || []).map(l => orderMap.has(l.id) ? { ...l, sort_order: orderMap.get(l.id) as number } : l)
  try {
    await apiFetch('/api/admin/links/reorder', { method: 'POST', body: { group_id: next[0]?.group_id || null, ordered_ids: next.map(item => item.id) } })
    await refreshLinks()
  } catch {
    links.value = previous
    linkError.value = 'Could not reorder links. Please try again.'
  }
}
async function moveGroup(group: AdminGroup, dir: -1 | 1) {
  groupError.value = ''
  const list = [...(groups.value || [])]
  const index = list.findIndex(g => g.id === group.id)
  const target = index + dir
  if (index < 0 || target < 0 || target >= list.length) return
  ;[list[index], list[target]] = [list[target], list[index]]
  const previous = groups.value
  groups.value = list.map((g, i) => ({ ...g, sort_order: i }))
  try {
    await apiFetch('/api/admin/groups/reorder', { method: 'POST', body: { ordered_ids: list.map(g => g.id) } })
    await refreshGroups()
  } catch {
    groups.value = previous
    groupError.value = 'Could not reorder groups. Please try again.'
  }
}
</script>

<template>
  <main class="admin-layout">
    <AdminNav />
    <div class="two-col">
      <section class="admin-card form-grid link-form">
        <h2>{{ editingLink ? 'Edit link' : 'New link' }}</h2>
        <label class="form-row">Group<select v-model="linkForm.group_id"><option v-for="group in groupOptions" :key="group.id" :value="group.id">{{ group.title }}</option></select></label>
        <label class="form-row">Title<input v-model="linkForm.title" placeholder="My latest drop"></label>
        <label class="form-row">URL<input v-model="linkForm.url" type="url" inputmode="url" placeholder="https://example.com"></label>
        <label class="form-row">Description<input v-model="linkForm.description"></label>
        <div class="form-row">
          <span>Icon <small class="muted">(optional — pick one)</small></span>
          <div class="icon-type-tabs">
            <button type="button" class="tab" :class="{ active: iconType === 'none' }" @click="setIconType('none')">None</button>
            <button type="button" class="tab" :class="{ active: iconType === 'emoji' }" @click="setIconType('emoji')">Emoji</button>
            <button type="button" class="tab" :class="{ active: iconType === 'image' }" @click="setIconType('image')">Image</button>
            <button type="button" class="tab" :class="{ active: iconType === 'icon' }" @click="setIconType('icon')">Icon</button>
          </div>
        </div>
        <label v-if="iconType === 'emoji'" class="form-row">Emoji<input v-model="linkForm.icon" placeholder="✨" maxlength="8"></label>
        <div v-else-if="iconType === 'image'" class="form-row">
          <span>Image</span>
          <div class="img-field">
            <span v-if="linkForm.icon_image" class="img-preview"><img :src="linkForm.icon_image" alt="Link image preview"></span>
            <label class="btn">{{ uploadingImage ? 'Uploading…' : (linkForm.icon_image ? 'Replace image' : 'Upload image') }}<input hidden type="file" accept="image/png,image/jpeg,image/webp,image/gif" @change="uploadLinkImage"></label>
            <button v-if="linkForm.icon_image" type="button" class="btn danger" @click="clearLinkImage">Remove</button>
          </div>
          <small class="hint">Square images look best. Anything larger than 1024×1024 is center-cropped and downscaled.</small>
        </div>
        <div v-else-if="iconType === 'icon'" class="form-row">
          <span>Font Awesome class</span>
          <div class="icon-field">
            <input v-model="linkForm.icon_font" placeholder="fa-brands fa-github" spellcheck="false" autocapitalize="off">
            <span class="icon-preview"><i v-if="linkForm.icon_font.trim()" :class="linkForm.icon_font" aria-hidden="true"></i></span>
          </div>
          <small class="hint">Any <a href="https://fontawesome.com/search?o=r&m=free" target="_blank" rel="noopener">Font Awesome Free</a> class, e.g. <code>fa-brands fa-github</code> or <code>fa-solid fa-globe</code>.</small>
        </div>
        <label class="form-row">Expires at<input v-model="linkForm.expires_at" type="datetime-local"></label>
        <p v-if="linkError" class="form-error">{{ linkError }}</p>
        <div class="form-actions">
          <button class="btn primary" @click="saveLink">{{ editingLink ? 'Save changes' : 'Add link' }}</button>
          <button v-if="editingLink" type="button" class="btn ghost" @click="resetLinkForm">Cancel</button>
        </div>
      </section>

      <section class="admin-card list-card">
        <h2>Groups &amp; links</h2>
        <form class="group-add" @submit.prevent="createGroup">
          <input v-model="newGroup.title" class="ga-input" placeholder="New group name" required>
          <input v-model="newGroup.description" class="ga-input" placeholder="Description (optional)">
          <label class="check"><input v-model="newGroup.collapsible" type="checkbox"> Collapsible</label>
          <select v-model="newGroup.layout" class="ga-select" title="Link layout"><option value="list">List</option><option value="grid">Grid</option></select>
          <select v-model="newGroup.corners" class="ga-select" title="Card corners"><option value="rounded">Rounded</option><option value="sharp">Sharp</option></select>
          <select v-model="newGroup.icon" class="ga-select" title="Icon shape"><option value="round">Round icons</option><option value="square">Square icons</option></select>
          <button class="btn primary" type="submit">Add group</button>
        </form>
        <p v-if="groupError" class="form-error">{{ groupError }}</p>

        <div v-for="group in groupOptions" :key="group.id || 'ungrouped'" class="group-block">
          <div class="group-heading">
            <template v-if="editingGroupId === group.id">
              <div class="group-edit">
                <input v-model="editGroupForm.title" class="ga-input" placeholder="Group name">
                <input v-model="editGroupForm.description" class="ga-input" placeholder="Description">
                <label class="check"><input v-model="editGroupForm.collapsible" type="checkbox"> Collapsible</label>
                <select v-model="editGroupForm.layout" class="ga-select" title="Link layout"><option value="list">List</option><option value="grid">Grid</option></select>
                <select v-model="editGroupForm.corners" class="ga-select" title="Card corners"><option value="rounded">Rounded</option><option value="sharp">Sharp</option></select>
                <select v-model="editGroupForm.icon" class="ga-select" title="Icon shape"><option value="round">Round icons</option><option value="square">Square icons</option></select>
              </div>
              <span class="group-actions">
                <button class="btn primary" @click="saveEditGroup(group)">Save</button>
                <button class="btn ghost" @click="cancelEditGroup">Cancel</button>
              </span>
            </template>
            <template v-else>
              <h3>{{ group.title }}</h3>
              <span v-if="group.id" class="group-actions">
                <button class="btn" title="Move group up" @click="moveGroup(group, -1)">↑</button>
                <button class="btn" title="Move group down" @click="moveGroup(group, 1)">↓</button>
                <button class="btn" title="Edit group" @click="startEditGroup(group)">Edit</button>
                <button class="btn danger" title="Delete group" @click="deleteGroup(group.id)">Delete</button>
              </span>
            </template>
          </div>
          <article v-for="(link, index) in linksByGroup(group.id || null)" :key="link.id" class="link-row">
            <span class="link-meta">
              <img v-if="link.icon_image" class="row-thumb" :src="link.icon_image" alt="" width="28" height="28">
              <span v-else-if="link.icon" class="row-emoji">{{ link.icon }}</span>
              <span v-else-if="link.icon_font" class="row-emoji"><i :class="link.icon_font" aria-hidden="true"></i></span>
              <span><strong>{{ link.title }}</strong><small>{{ link.url }} · {{ link.click_count }} clicks</small></span>
            </span>
            <span class="row-actions"><button class="btn" title="Move up" @click="move(linksByGroup(group.id || null), index, -1)">↑</button><button class="btn" title="Move down" @click="move(linksByGroup(group.id || null), index, 1)">↓</button><button class="btn" @click="editLink(link)">Edit</button><button class="btn danger" @click="deleteLink(link.id)">Delete</button></span>
          </article>
          <p v-if="group.id && !linksByGroup(group.id).length" class="empty-hint">No links in this group yet.</p>
        </div>
      </section>
    </div>
  </main>
</template>

<style scoped>
.two-col { display: grid; grid-template-columns: minmax(280px, 360px) 1fr; gap: 18px; align-items: start; }
h2, h3 { font-family: var(--font-heading); margin: 0; }
.list-card, .group-block { display: grid; gap: 14px; }
.form-actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.form-error { margin: 0; color: #fca5a5; font-size: .9rem; }
.group-add { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; padding-bottom: 14px; border-bottom: 1px dashed var(--color-border); }
.group-add .ga-input { flex: 1 1 150px; width: auto; min-width: 0; }
.group-add .ga-select, .group-edit .ga-select { flex: 0 1 auto; width: auto; min-width: 0; padding: 9px 10px; }
.group-add .btn { flex: none; }
.icon-type-tabs { display: inline-flex; flex-wrap: wrap; gap: 6px; }
.icon-type-tabs .tab { padding: 7px 13px; border: 1px solid var(--color-border); border-radius: var(--radius-input); background: var(--color-surface-alt); color: var(--color-text); font-size: .85rem; cursor: pointer; transition: border-color .15s ease, background .15s ease; }
.icon-type-tabs .tab.active { border-color: var(--color-primary); background: color-mix(in srgb, var(--color-primary) 14%, var(--color-surface)); }
.icon-field { display: flex; align-items: center; gap: 10px; }
.icon-field input { flex: 1 1 auto; min-width: 0; }
.icon-preview { display: grid; place-items: center; width: 44px; height: 44px; border-radius: var(--radius-icon); border: 1px solid var(--color-border); background: var(--color-surface-alt); color: var(--color-text); flex: none; font-size: 1.15rem; }
.hint { color: var(--color-text-muted); font-size: .78rem; line-height: 1.4; }
.hint code { padding: 1px 5px; border-radius: 5px; background: color-mix(in srgb, var(--color-surface-alt) 85%, transparent); }
.muted { color: var(--color-text-muted); font-weight: 400; }
.group-heading, .link-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
.group-edit { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; flex: 1 1 auto; min-width: 0; }
.group-edit .ga-input { flex: 1 1 140px; width: auto; min-width: 0; }
.link-row { padding: 12px; border: 1px solid var(--color-border); border-radius: var(--radius-input); background: var(--color-surface-alt); }
.link-row small { display: block; color: var(--color-text-muted); overflow-wrap: anywhere; }
.link-meta { display: flex; align-items: center; gap: 10px; min-width: 0; }
.row-thumb { width: 28px; height: 28px; border-radius: var(--radius-icon); object-fit: cover; flex: none; }
.row-emoji { font-size: 1.2rem; flex: none; }
.empty-hint { margin: 0; color: var(--color-text-muted); font-size: .88rem; }
.img-field { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.img-preview { width: 44px; height: 44px; border-radius: var(--radius-icon); overflow: hidden; border: 1px solid var(--color-border); flex: none; }
.img-preview img { width: 100%; height: 100%; object-fit: cover; }
.row-actions, .group-actions { display: flex; gap: 8px; flex-wrap: wrap; flex: none; }
.check { display: inline-flex; align-items: center; gap: 8px; margin: 0; padding: 9px 12px; border: 1px solid var(--color-border); border-radius: var(--radius-input); background: var(--color-surface-alt); color: var(--color-text); font-size: .88rem; cursor: pointer; user-select: none; white-space: nowrap; flex: none; }
.check input[type='checkbox'] { width: 18px; height: 18px; flex: none; margin: 0; accent-color: var(--color-primary); cursor: pointer; }
.check:has(input:checked) { border-color: var(--color-primary); background: color-mix(in srgb, var(--color-primary) 12%, var(--color-surface)); }
@media (max-width: 860px) { .two-col { grid-template-columns: 1fr; } .group-heading, .link-row { align-items: stretch; } }
</style>
