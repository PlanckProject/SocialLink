import { expect, test } from '@playwright/test'

// Feat 3 (icon library): profile social icons are rendered with Font Awesome
// Free — as <i class="fa-..."> elements — instead of inline SVG.
test.describe('feat 3: Font Awesome social icons', () => {
  async function currentProfile(request: any) {
    return (await (await request.get('/api/admin/profile')).json()).data
  }
  async function setSocials(request: any, base: any, socials: any[]) {
    const res = await request.put('/api/admin/profile', {
      data: {
        display_name: base.display_name || 'Admin',
        bio: base.bio || '',
        location: base.location || '',
        socials,
      },
    })
    expect(res.ok(), `set socials (${res.status()})`).toBeTruthy()
  }

  test('social links render as Font Awesome <i> icons', async ({ page, request }) => {
    const base = await currentProfile(request)
    const original = (base.socials || []).map((s: any) => ({ platform: s.platform, url: s.url }))
    try {
      await setSocials(request, base, [
        { platform: 'github', url: 'https://github.com/octocat' },
        { platform: 'website', url: 'https://example.com' },
      ])

      await page.goto('/')
      const icons = page.locator('nav.socials a.social-btn i.social-icon')
      await expect(icons).toHaveCount(2)
      // Brand icon uses a Font Awesome class; no inline <svg> is rendered.
      await expect(icons.first()).toHaveClass(/fa-/)
      await expect(page.locator('nav.socials svg')).toHaveCount(0)
    } finally {
      await setSocials(request, base, original)
    }
  })
})
