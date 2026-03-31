import { expect, test } from '@playwright/test'
import { cleanupE2E, createGroup, createLink, getProfile, updateGroup } from './helpers'

// Feat 2: Redis is enabled by default. The public profile is read-through
// cached, and mutations must invalidate it so no stale data is served.
test.describe('feat 2: redis cache', () => {
  test.beforeEach(async ({ request }) => { await cleanupE2E(request) })
  test.afterEach(async ({ request }) => { await cleanupE2E(request) })

  test('a group rename is reflected on the public profile (cache invalidation)', async ({ request }) => {
    const group = await createGroup(request, 'E2E-Cache')
    await createLink(request, group.id, 'E2E-cache-link', { icon: '⚡' })

    // Warm the cache, then read again (served from Redis).
    const first = await getProfile(request)
    expect(JSON.stringify(first.groups)).toContain('E2E-Cache')
    const second = await getProfile(request)
    expect(JSON.stringify(second.groups)).toContain('E2E-Cache')

    // Mutate and confirm the cached profile is invalidated, not stale.
    await updateGroup(request, group.id, { title: 'E2E-Cache-Renamed' })
    await expect.poll(async () => JSON.stringify((await getProfile(request)).groups))
      .toContain('E2E-Cache-Renamed')
  })
})
