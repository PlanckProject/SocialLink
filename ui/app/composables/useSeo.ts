import type { Ref } from 'vue'
import type { PublicProfileResponse } from '~/types/social'

function absoluteUrl(origin: string, url?: string | null): string | undefined {
  if (!url) return undefined
  if (/^https?:\/\//i.test(url)) return url
  return origin.replace(/\/$/, '') + (url.startsWith('/') ? url : `/${url}`)
}

/** Escapes `<` so JSON-LD can be safely inlined in a <script> tag. */
function safeJson(value: unknown): string {
  return JSON.stringify(value).replace(/</g, '\\u003c')
}

/**
 * SSR-rendered SEO for a public profile: title/description, Open Graph, Twitter
 * cards, canonical URL, lang and JSON-LD structured data (ProfilePage + Person +
 * ItemList of links). Driven by the profile data and the active theme branding.
 */
export function useProfileSeo(data: Ref<PublicProfileResponse | null>) {
  const config = useConfigStore()
  const requestUrl = useRequestURL()
  const origin = requestUrl.origin

  const profile = computed(() => data.value?.profile || null)
  const site_name = computed(() => config.branding.site_name || 'SocialLink')
  const title = computed(() => profile.value?.display_name || site_name.value)
  const description = computed(() => {
    const current = profile.value
    const text = current?.bio || `Explore ${title.value}'s links, socials and profiles.`
    return text.length > 300 ? `${text.slice(0, 297)}…` : text
  })
  const image = computed(() => absoluteUrl(origin, profile.value?.cover_url || profile.value?.avatar_url))
  const canonical = computed(() => origin.replace(/\/$/, '') + requestUrl.pathname)

  useSeoMeta({
    title: () => title.value,
    description: () => description.value,
    ogType: 'profile',
    ogTitle: () => title.value,
    ogDescription: () => description.value,
    ogUrl: () => canonical.value,
    ogSiteName: () => site_name.value,
    ogImage: () => image.value,
    twitterCard: 'summary_large_image',
    twitterTitle: () => title.value,
    twitterDescription: () => description.value,
    twitterImage: () => image.value,
    robots: 'index, follow'
  })

  const structuredData = computed(() => {
    const current = profile.value
    if (!current) return ''
    const links = [
      ...(data.value?.groups.flatMap(group => group.links) || []),
      ...(data.value?.ungrouped || [])
    ]
    const sameAs = (current.socials || [])
      .map(social => socialHref(social.platform, social.url))
      .filter(href => /^https?:\/\//i.test(href))

    const person: Record<string, unknown> = {
      '@type': 'Person',
      name: current.display_name,
      alternateName: current.username,
      url: canonical.value
    }
    if (current.bio) person.description = current.bio
    if (current.location) person.address = { '@type': 'PostalAddress', addressLocality: current.location }
    const avatar = absoluteUrl(origin, current.avatar_url)
    if (avatar) person.image = avatar
    if (sameAs.length) person.sameAs = sameAs

    const graph: Record<string, unknown>[] = [
      { '@type': 'ProfilePage', name: title.value, url: canonical.value, mainEntity: person }
    ]
    if (links.length) {
      graph.push({
        '@type': 'ItemList',
        name: `${title.value} — links`,
        itemListElement: links.map((link, index) => ({
          '@type': 'ListItem',
          position: index + 1,
          name: link.title,
          url: absoluteUrl(origin, `/api/l/${link.id}`)
        }))
      })
    }
    return safeJson({ '@context': 'https://schema.org', '@graph': graph })
  })

  useHead(() => ({
    htmlAttrs: { lang: 'en' },
    link: [{ rel: 'canonical', href: canonical.value }],
    script: structuredData.value
      ? [{ type: 'application/ld+json', innerHTML: structuredData.value, key: 'ld-profile' }]
      : []
  }))
}
