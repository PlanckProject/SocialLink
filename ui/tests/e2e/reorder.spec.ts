import { expect, test } from '@playwright/test'
import { adminGroupOrder, cleanupE2E, createGroup, createLink, publicGroupOrder } from './helpers'

// Bugfix: reordering groups (the ↑/↓ buttons on /admin/links) must actually
// change the order — in the admin list, after a reload, and on the public page.
test.describe('bugfix: group reorder', () => {
  test.beforeEach(async ({ request }) => { await cleanupE2E(request) })
  test.afterEach(async ({ request }) => { await cleanupE2E(request) })

  test('move buttons reorder groups and persist everywhere', async ({ page, request }) => {
    const alpha = await createGroup(request, 'E2E-Alpha')
    const bravo = await createGroup(request, 'E2E-Bravo')
    const charlie = await createGroup(request, 'E2E-Charlie')
    // Public groups only render when they contain links.
    await createLink(request, alpha.id, 'E2E-a', { icon: '🔗' })
    await createLink(request, bravo.id, 'E2E-b', { icon: '🔗' })
    await createLink(request, charlie.id, 'E2E-c', { icon: '🔗' })

    await page.goto('/admin/links')
    await expect.poll(() => adminGroupOrder(page)).toEqual(['E2E-Alpha', 'E2E-Bravo', 'E2E-Charlie'])

    // Move Alpha down one slot.
    const alphaBlock = page.locator('.group-block', { has: page.locator('h3', { hasText: 'E2E-Alpha' }) })
    await alphaBlock.getByTitle('Move group down').click()
    await expect.poll(() => adminGroupOrder(page)).toEqual(['E2E-Bravo', 'E2E-Alpha', 'E2E-Charlie'])

    // Persists across a reload.
    await page.reload()
    await expect.poll(() => adminGroupOrder(page)).toEqual(['E2E-Bravo', 'E2E-Alpha', 'E2E-Charlie'])

    // Reflected on the public profile.
    await page.goto('/')
    await expect.poll(() => publicGroupOrder(page)).toEqual(['E2E-Bravo', 'E2E-Alpha', 'E2E-Charlie'])

    // Move up returns Alpha to the top.
    await page.goto('/admin/links')
    await page.locator('.group-block', { has: page.locator('h3', { hasText: 'E2E-Alpha' }) })
      .getByTitle('Move group up').click()
    await expect.poll(() => adminGroupOrder(page)).toEqual(['E2E-Alpha', 'E2E-Bravo', 'E2E-Charlie'])
  })
})
