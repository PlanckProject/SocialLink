import { expect, type APIRequestContext, type Page } from '@playwright/test'
import zlib from 'node:zlib'

export const ADMIN_USERNAME = process.env.ADMIN_USERNAME || 'admin'
export const ADMIN_PASSWORD = process.env.ADMIN_PASSWORD || 'changeme'

// Marker prefix so test data is easy to find and clean up.
export const E2E = 'E2E-'

export interface GroupStyleInput {
  layout?: 'list' | 'grid'
  corners?: 'rounded' | 'sharp'
  icon?: 'round' | 'square'
}

// The API wraps every response in an envelope; unwrap `.data`.
async function data<T = any>(res: { json: () => Promise<any> }): Promise<T> {
  return (await res.json()).data as T
}

export async function createGroup(request: APIRequestContext, title: string, style?: GroupStyleInput) {
  const res = await request.post('/api/admin/groups', { data: { title, style } })
  expect(res.ok(), `create group ${title} (${res.status()})`).toBeTruthy()
  return data(res)
}

export async function updateGroup(request: APIRequestContext, id: string, body: Record<string, unknown>) {
  const res = await request.put(`/api/admin/groups/${id}`, { data: body })
  expect(res.ok(), `update group ${id} (${res.status()})`).toBeTruthy()
  return data(res)
}

export async function createLink(
  request: APIRequestContext,
  groupId: string | null,
  title: string,
  extra: Record<string, unknown> = {},
) {
  const res = await request.post('/api/admin/links', {
    data: { group_id: groupId, title, url: 'https://example.com', description: '', ...extra },
  })
  expect(res.ok(), `create link ${title} (${res.status()})`).toBeTruthy()
  return data(res)
}

export async function getProfile(request: APIRequestContext) {
  const res = await request.get('/api/profile')
  expect(res.ok(), `get profile (${res.status()})`).toBeTruthy()
  return data(res)
}

// Remove every group/link created by the E2E suite (idempotent).
export async function cleanupE2E(request: APIRequestContext) {
  const groups: any[] = (await data(await request.get('/api/admin/groups'))) || []
  const links: any[] = (await data(await request.get('/api/admin/links'))) || []
  const e2eGroupIds = new Set(groups.filter(g => isE2E(g.title)).map(g => g.id))
  for (const l of links) {
    if (e2eGroupIds.has(l.group_id) || isE2E(l.title)) {
      await request.delete(`/api/admin/links/${l.id}`)
    }
  }
  for (const id of e2eGroupIds) await request.delete(`/api/admin/groups/${id}`)
}

function isE2E(title: string | null | undefined): boolean {
  return !!title && title.startsWith(E2E)
}

// Admin /admin/links group order (E2E groups only, in display order).
export async function adminGroupOrder(page: Page): Promise<string[]> {
  const titles = await page.locator('.group-block h3').allTextContents()
  return titles.map(t => t.trim()).filter(isE2E)
}

// Public profile group order (E2E groups only, in display order).
export async function publicGroupOrder(page: Page): Promise<string[]> {
  const titles = await page.locator('.link-group .group-title strong').allTextContents()
  return titles.map(t => t.trim()).filter(isE2E)
}

// ---- Minimal PNG encoder (solid-color RGBA) for upload fixtures. ----
export function makePng(width: number, height: number, rgb: [number, number, number] = [70, 110, 200]): Buffer {
  const bpp = 4
  const stride = width * bpp + 1
  const raw = Buffer.alloc(stride * height)
  for (let y = 0; y < height; y++) {
    const rowStart = y * stride
    raw[rowStart] = 0 // filter: none
    for (let x = 0; x < width; x++) {
      const o = rowStart + 1 + x * bpp
      raw[o] = rgb[0]
      raw[o + 1] = rgb[1]
      raw[o + 2] = rgb[2]
      raw[o + 3] = 255
    }
  }
  const idat = zlib.deflateSync(raw, { level: 6 })
  const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(width, 0)
  ihdr.writeUInt32BE(height, 4)
  ihdr[8] = 8 // bit depth
  ihdr[9] = 6 // colour type: RGBA
  return Buffer.concat([signature, pngChunk('IHDR', ihdr), pngChunk('IDAT', idat), pngChunk('IEND', Buffer.alloc(0))])
}

function pngChunk(type: string, payload: Buffer): Buffer {
  const typeBuf = Buffer.from(type, 'ascii')
  const len = Buffer.alloc(4)
  len.writeUInt32BE(payload.length, 0)
  const body = Buffer.concat([typeBuf, payload])
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(body), 0)
  return Buffer.concat([len, body, crc])
}

const CRC_TABLE = (() => {
  const table = new Uint32Array(256)
  for (let n = 0; n < 256; n++) {
    let c = n
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
    table[n] = c >>> 0
  }
  return table
})()

function crc32(buf: Buffer): number {
  let c = 0xffffffff
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8)
  return (c ^ 0xffffffff) >>> 0
}

// Load an image in the browser and return its intrinsic dimensions.
export async function imageSize(page: Page, src: string): Promise<{ width: number; height: number }> {
  return page.evaluate(async (url) => {
    const img = new Image()
    img.src = url
    await img.decode()
    return { width: img.naturalWidth, height: img.naturalHeight }
  }, src)
}
