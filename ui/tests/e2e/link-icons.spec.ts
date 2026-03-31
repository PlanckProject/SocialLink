import { expect, test } from '@playwright/test'
import { cleanupE2E, createGroup, imageSize, makePng } from './helpers'

// Feat 3: a link icon is optional and at most one of { image, emoji, Font
// Awesome class }. Images are center-cropped to a square ≤ 1024×1024.
test.describe('feat 3: link icons', () => {
  test.beforeEach(async ({ request }) => { await cleanupE2E(request) })
  test.afterEach(async ({ request }) => { await cleanupE2E(request) })

  test('emoji, image and Font Awesome icons create, render and stay exclusive', async ({ page, request }) => {
    const group = await createGroup(request, 'E2E-Icons')

    await page.goto('/admin/links')
    const form = page.locator('.link-form')
    const groupSelect = form.locator('select').first()
    const title = form.getByPlaceholder('My latest drop')
    const url = form.getByPlaceholder('https://example.com')
    const tab = (name: string) => form.locator('.icon-type-tabs .tab', { hasText: new RegExp(`^${name}$`) })
    const addBtn = form.getByRole('button', { name: 'Add link' })

    // 1) Emoji link.
    await groupSelect.selectOption({ label: 'E2E-Icons' })
    await title.fill('E2E-emoji')
    await url.fill('https://example.com/emoji')
    await tab('Emoji').click()
    await form.getByPlaceholder('✨').fill('🌟')
    await addBtn.click()
    await expect(page.locator('.link-row', { has: page.locator('strong', { hasText: 'E2E-emoji' }) })).toBeVisible()

    // 2) Font Awesome link — live preview must render the typed class.
    await groupSelect.selectOption({ label: 'E2E-Icons' })
    await title.fill('E2E-fa')
    await url.fill('https://example.com/fa')
    await tab('Icon').click()
    await form.getByPlaceholder('fa-brands fa-github').fill('fa-brands fa-github')
    await expect(form.locator('.icon-preview i')).toHaveClass(/fa-github/)
    await addBtn.click()
    const faRow = page.locator('.link-row', { has: page.locator('strong', { hasText: 'E2E-fa' }) })
    await expect(faRow).toBeVisible()
    await expect(faRow.locator('i')).toHaveClass(/fa-github/)

    // 3) Image link — upload an oversized, non-square PNG (2000×1200).
    await groupSelect.selectOption({ label: 'E2E-Icons' })
    await title.fill('E2E-image')
    await url.fill('https://example.com/image')
    await tab('Image').click()
    await form.locator('input[type="file"]').setInputFiles({
      name: 'wide.png',
      mimeType: 'image/png',
      buffer: makePng(2000, 1200),
    })
    await expect(form.locator('.img-preview img')).toBeVisible()

    // Switching to another type clears the uploaded image (mutual exclusion in UI).
    await tab('Emoji').click()
    await expect(form.locator('.img-preview')).toHaveCount(0)
    // Switch back and re-upload for the actual save.
    await tab('Image').click()
    await form.locator('input[type="file"]').setInputFiles({
      name: 'wide.png',
      mimeType: 'image/png',
      buffer: makePng(2000, 1200),
    })
    await expect(form.locator('.img-preview img')).toBeVisible()
    await addBtn.click()
    const imgRow = page.locator('.link-row', { has: page.locator('strong', { hasText: 'E2E-image' }) })
    await expect(imgRow).toBeVisible()
    await expect(imgRow.locator('img.row-thumb')).toBeVisible()

    // The stored image is a square no larger than 1024×1024.
    const src = await imgRow.locator('img.row-thumb').getAttribute('src')
    expect(src).toBeTruthy()
    const size = await imageSize(page, src!)
    expect(size.width).toBe(size.height)
    expect(size.width).toBeLessThanOrEqual(1024)
    expect(size.width).toBeGreaterThan(0)

    // Public profile renders each icon in its expected form.
    await page.goto('/')
    const iconsGroup = page.locator('.link-group', { has: page.locator('.group-title strong', { hasText: 'E2E-Icons' }) })
    const emojiCard = iconsGroup.locator('.link-card', { has: page.locator('strong', { hasText: 'E2E-emoji' }) })
    const faCard = iconsGroup.locator('.link-card', { has: page.locator('strong', { hasText: 'E2E-fa' }) })
    const imgCard = iconsGroup.locator('.link-card', { has: page.locator('strong', { hasText: 'E2E-image' }) })
    await expect(emojiCard.locator('.icon')).toContainText('🌟')
    await expect(faCard.locator('.icon i')).toHaveClass(/fa-github/)
    await expect(imgCard.locator('.icon.img img')).toBeVisible()
  })

  test('API rejects a link with more than one icon type', async ({ request }) => {
    const group = await createGroup(request, 'E2E-Conflict')
    const res = await request.post('/api/admin/links', {
      data: {
        group_id: group.id,
        title: 'E2E-conflict',
        url: 'https://example.com',
        icon: '🌟',
        icon_font: 'fa-solid fa-globe',
      },
    })
    expect(res.status()).toBe(400)
  })
})
