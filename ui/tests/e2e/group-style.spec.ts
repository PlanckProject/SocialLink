import { expect, test } from '@playwright/test'
import { cleanupE2E, createGroup, createLink } from './helpers'

// Feat 1: each group governs the *shape* of its links (layout, card corners,
// icon shape). Different groups can differ; colors/text stay theme-owned.
test.describe('feat 1: per-group link shape styling', () => {
  test.beforeEach(async ({ request }) => { await cleanupE2E(request) })
  test.afterEach(async ({ request }) => { await cleanupE2E(request) })

  test('groups render their own layout/corners/icon shape independently', async ({ page, request }) => {
    const grid = await createGroup(request, 'E2E-Grid', { layout: 'grid', corners: 'sharp', icon: 'square' })
    const list = await createGroup(request, 'E2E-List', { layout: 'list', corners: 'rounded', icon: 'round' })
    await createLink(request, grid.id, 'E2E-grid-link', { icon: '🔷' })
    await createLink(request, list.id, 'E2E-list-link', { icon: '🔶' })

    await page.goto('/')
    const gridGroup = page.locator('.link-group', { has: page.locator('.group-title strong', { hasText: 'E2E-Grid' }) })
    const listGroup = page.locator('.link-group', { has: page.locator('.group-title strong', { hasText: 'E2E-List' }) })

    // Layout class differs.
    await expect(gridGroup.locator('.links')).toHaveClass(/style-grid/)
    await expect(listGroup.locator('.links')).toHaveClass(/style-list/)

    // Grid group renders two columns; list group a single column.
    const gridCols = await gridGroup.locator('.links').evaluate(el => getComputedStyle(el).gridTemplateColumns)
    const listCols = await listGroup.locator('.links').evaluate(el => getComputedStyle(el).gridTemplateColumns)
    expect(gridCols.split(' ').length).toBeGreaterThan(1)
    expect(listCols.split(' ').length).toBe(1)

    // Corners: sharp cards have 0 radius; rounded cards have a positive radius.
    const gridRadius = await gridGroup.locator('.link-card').first().evaluate(el => getComputedStyle(el).borderTopLeftRadius)
    const listRadius = await listGroup.locator('.link-card').first().evaluate(el => getComputedStyle(el).borderTopLeftRadius)
    expect(parseFloat(gridRadius)).toBe(0)
    expect(parseFloat(listRadius)).toBeGreaterThan(0)

    // Icon shape: square icons are less rounded than round icons.
    const gridIcon = await gridGroup.locator('.link-card .icon').first().evaluate(el => getComputedStyle(el).borderTopLeftRadius)
    const listIcon = await listGroup.locator('.link-card .icon').first().evaluate(el => getComputedStyle(el).borderTopLeftRadius)
    expect(parseFloat(gridIcon)).toBeLessThan(parseFloat(listIcon))

    // Colors stay theme-owned: identical across both groups.
    const gridBg = await gridGroup.locator('.link-card').first().evaluate(el => getComputedStyle(el).backgroundColor)
    const listBg = await listGroup.locator('.link-card').first().evaluate(el => getComputedStyle(el).backgroundColor)
    const gridColor = await gridGroup.locator('.link-card').first().evaluate(el => getComputedStyle(el).color)
    const listColor = await listGroup.locator('.link-card').first().evaluate(el => getComputedStyle(el).color)
    expect(gridBg).toBe(listBg)
    expect(gridColor).toBe(listColor)
  })

  test('a group with no explicit style uses the list/rounded/round defaults', async ({ page, request }) => {
    const plain = await createGroup(request, 'E2E-Default')
    await createLink(request, plain.id, 'E2E-default-link', { icon: '⭐' })

    await page.goto('/')
    const group = page.locator('.link-group', { has: page.locator('.group-title strong', { hasText: 'E2E-Default' }) })
    await expect(group.locator('.links')).toHaveClass(/style-list/)
    const radius = await group.locator('.link-card').first().evaluate(el => getComputedStyle(el).borderTopLeftRadius)
    expect(parseFloat(radius)).toBeGreaterThan(0)
  })
})
