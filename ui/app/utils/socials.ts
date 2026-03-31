import { SOCIAL_ICON_CLASSES } from '~/utils/socialIcons'

export interface SocialPlatform {
  slug: string
  label: string
  aliases?: string[]
  domains?: string[]
}

/** Famous social apps offered in the profile editor (with brand icons). */
export const SOCIAL_PLATFORMS: SocialPlatform[] = [
  { slug: 'instagram', label: 'Instagram', domains: ['instagram.com'] },
  { slug: 'youtube', label: 'YouTube', domains: ['youtube.com', 'youtu.be'] },
  { slug: 'linkedin', label: 'LinkedIn', domains: ['linkedin.com'] },
  { slug: 'x', label: 'X (Twitter)', aliases: ['twitter'], domains: ['x.com', 'twitter.com'] },
  { slug: 'github', label: 'GitHub', domains: ['github.com'] },
  { slug: 'facebook', label: 'Facebook', domains: ['facebook.com', 'fb.com', 'fb.me'] },
  { slug: 'tiktok', label: 'TikTok', domains: ['tiktok.com'] },
  { slug: 'twitch', label: 'Twitch', domains: ['twitch.tv'] },
  { slug: 'discord', label: 'Discord', domains: ['discord.gg', 'discord.com'] },
  { slug: 'telegram', label: 'Telegram', domains: ['t.me', 'telegram.me'] },
  { slug: 'whatsapp', label: 'WhatsApp', domains: ['wa.me', 'whatsapp.com'] },
  { slug: 'spotify', label: 'Spotify', domains: ['spotify.com', 'open.spotify.com'] },
  { slug: 'reddit', label: 'Reddit', domains: ['reddit.com'] },
  { slug: 'pinterest', label: 'Pinterest', domains: ['pinterest.com'] },
  { slug: 'snapchat', label: 'Snapchat', domains: ['snapchat.com'] },
  { slug: 'medium', label: 'Medium', domains: ['medium.com'] },
  { slug: 'dribbble', label: 'Dribbble', domains: ['dribbble.com'] },
  { slug: 'behance', label: 'Behance', domains: ['behance.net'] },
  { slug: 'mastodon', label: 'Mastodon', domains: ['mastodon.social', 'mastodon.online'] },
  { slug: 'threads', label: 'Threads', domains: ['threads.net'] },
  { slug: 'substack', label: 'Substack', domains: ['substack.com'] },
  { slug: 'patreon', label: 'Patreon', domains: ['patreon.com'] },
  { slug: 'soundcloud', label: 'SoundCloud', domains: ['soundcloud.com'] },
  { slug: 'applemusic', label: 'Apple Music', aliases: ['apple-music', 'apple music'], domains: ['music.apple.com'] },
  { slug: 'bluesky', label: 'Bluesky', aliases: ['bsky'], domains: ['bsky.app'] },
  { slug: 'email', label: 'Email', aliases: ['mail', 'e-mail'], domains: [] },
  { slug: 'website', label: 'Website', aliases: ['web', 'link', 'globe', 'home', 'other'], domains: [] }
]

const BY_KEY = new Map<string, SocialPlatform>()
for (const platform of SOCIAL_PLATFORMS) {
  BY_KEY.set(platform.slug, platform)
  for (const alias of platform.aliases || []) BY_KEY.set(alias, platform)
}

const WEBSITE = BY_KEY.get('website') as SocialPlatform
const EMAIL = BY_KEY.get('email') as SocialPlatform

/** Resolves a platform from an explicit name and/or a URL (brand auto-detect). */
export function detectPlatform(platform?: string | null, url?: string | null): SocialPlatform {
  const key = (platform || '').trim().toLowerCase()
  if (key && BY_KEY.has(key)) return BY_KEY.get(key) as SocialPlatform
  const normalized = key.replace(/[^a-z0-9]/g, '')
  if (normalized && BY_KEY.has(normalized)) return BY_KEY.get(normalized) as SocialPlatform

  const raw = (url || '').trim().toLowerCase()
  if (key === 'email' || raw.startsWith('mailto:') || (!raw.includes('/') && raw.includes('@'))) return EMAIL

  if (raw) {
    let host = ''
    try {
      host = new URL(raw.includes('://') ? raw : `https://${raw}`).hostname.replace(/^www\./, '')
    } catch { host = '' }
    if (host) {
      for (const platformCandidate of SOCIAL_PLATFORMS) {
        if ((platformCandidate.domains || []).some(domain => host === domain || host.endsWith(`.${domain}`))) {
          return platformCandidate
        }
      }
    }
  }
  return WEBSITE
}

export function socialIconClass(slug: string): string {
  return SOCIAL_ICON_CLASSES[slug] || SOCIAL_ICON_CLASSES.website
}

export function socialLabel(platform?: string | null, url?: string | null): string {
  return detectPlatform(platform, url).label
}

/** Builds a safe outbound href, adding https:// or mailto: as needed. */
export function socialHref(platform?: string | null, url?: string | null): string {
  const value = (url || '').trim()
  if (!value) return '#'
  if (/^(https?:\/\/|mailto:)/i.test(value)) return value
  const resolved = detectPlatform(platform, url)
  if (resolved.slug === 'email' && value.includes('@')) return `mailto:${value}`
  return `https://${value}`
}
