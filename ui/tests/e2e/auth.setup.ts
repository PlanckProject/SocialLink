import { test as setup } from '@playwright/test'
import fs from 'node:fs'
import path from 'node:path'
import { ADMIN_PASSWORD, ADMIN_USERNAME } from './helpers'

const authFile = 'tests/e2e/.auth/admin.json'

// Logs in through the real UI form once and stores the session cookie so
// every spec (and the `request` fixture) is authenticated.
setup('authenticate', async ({ page }) => {
  await page.goto('/admin/login')
  await page.fill('input[autocomplete="username"]', ADMIN_USERNAME)
  await page.fill('input[type="password"]', ADMIN_PASSWORD)
  await page.click('button.primary')
  await page.waitForURL('**/admin')
  fs.mkdirSync(path.dirname(authFile), { recursive: true })
  await page.context().storageState({ path: authFile })
})
