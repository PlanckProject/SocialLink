import type { ComputedRef, Ref } from 'vue'
import type { Theme } from '~/app.config'

const kebab = (value: string) => value.replace(/[A-Z]/g, match => `-${match.toLowerCase()}`)
const cssEscapeValue = (value: string) => value.replace(/[<>]/g, '')
const MAX_BACKGROUND_SHAPES = 12
const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value))
const finite = (value: unknown, fallback: number) => {
  const number = Number(value)
  return Number.isFinite(number) ? number : fallback
}

function seededRandom(seed: number) {
  let state = seed >>> 0
  return () => {
    state += 0x6D2B79F5
    let value = state
    value = Math.imul(value ^ (value >>> 15), value | 1)
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61)
    return ((value ^ (value >>> 14)) >>> 0) / 4294967296
  }
}

function backgroundShapeCssVars(theme: Theme) {
  const settings = theme.background.shapes
  const count = clamp(Math.trunc(finite(settings.count, 0)), 0, MAX_BACKGROUND_SHAPES)
  const minSize = clamp(finite(settings.min_size, 180), 40, 1200)
  const maxSize = clamp(finite(settings.max_size, 420), minSize, 1600)
  const random = seededRandom(Math.trunc(finite(settings.seed, 17)))
  const vars: Record<string, string> = {
    '--background-shape-opacity': String(clamp(finite(settings.opacity, 0.28), 0, 1)),
    '--background-shape-blur': `${clamp(finite(settings.blur, 90), 0, 240)}px`
  }

  for (let index = 0; index < MAX_BACKGROUND_SHAPES; index++) {
    const prefix = `--background-shape-${index + 1}`
    const visible = settings.enabled && index < count
    vars[`${prefix}-display`] = visible ? 'block' : 'none'
    if (!visible) continue

    const size = minSize + ((maxSize - minSize) * random())
    const height = size * (0.68 + (random() * 0.64))
    vars[`${prefix}-left`] = `${Math.round((-6 + (random() * 112)) * 100) / 100}%`
    vars[`${prefix}-top`] = `${Math.round((-6 + (random() * 112)) * 100) / 100}%`
    vars[`${prefix}-width`] = `${Math.round(size)}px`
    vars[`${prefix}-height`] = `${Math.round(height)}px`
    vars[`${prefix}-radius`] = Array.from({ length: 4 }, () => `${Math.round(38 + (random() * 34))}%`).join(' ')
    vars[`${prefix}-color`] = index % 2 === 0 ? theme.colors.primary : theme.colors.accent
    vars[`${prefix}-rotation`] = `${Math.round(random() * 360)}deg`
  }

  return vars
}

const radiusSetting = (value: string, fallback: number, max = 50) => {
  const parsed = Number.parseFloat(String(value).trim())
  return clamp(Number.isFinite(parsed) ? parsed : fallback, 0, max)
}

export function themeToCssVars(theme: Theme) {
  const radiusSettings = {
    link: radiusSetting(theme.radius.link, 22),
    linkIcon: radiusSetting(theme.radius.link_icon, 14),
    background: radiusSetting(theme.radius.background, 20, 20),
    avatar: radiusSetting(theme.radius.avatar, 50),
    socialIcon: radiusSetting(theme.radius.social_icon, 50)
  }
  const radius = Object.fromEntries(
    Object.entries(radiusSettings).map(([key, value]) => [key, value >= 50 ? '999px' : `${value}px`])
  )
  const vars: Record<string, string> = {
    '--color-background': theme.colors.background,
    '--color-surface': theme.colors.surface,
    '--color-surface-alt': theme.colors.surface_alt,
    '--color-text': theme.colors.text,
    '--color-text-muted': theme.colors.text_muted,
    '--color-primary': theme.colors.primary,
    '--color-primary-contrast': theme.colors.primary_contrast,
    '--color-accent': theme.colors.accent,
    '--color-border': theme.colors.border,
    '--font-heading': theme.fonts.heading,
    '--font-body': theme.fonts.body,
    '--layout-max-width': theme.layout.max_width,
    '--layout-spacing': theme.layout.spacing,
    '--layout-align': theme.layout.align,
    '--cover-height': theme.layout.cover_height,
    '--button-shadow': theme.button.shadow,
    '--background-value': theme.background.value,
    '--background-gradient': theme.background.type === 'gradient' ? theme.background.gradient : `linear-gradient(${theme.background.value}, ${theme.background.value})`,
    '--background-image': theme.background.type === 'image' && theme.background.image ? `url(${theme.background.image})` : 'none',
    '--background-overlay': theme.background.overlay
  }

  Object.assign(vars, backgroundShapeCssVars(theme))
  for (const [key, value] of Object.entries(radius)) vars[`--radius-${kebab(key)}`] = value
  return vars
}

export function varsToStyle(vars: Record<string, string>) {
  return Object.entries(vars).map(([key, value]) => `${key}:${cssEscapeValue(value)};`).join('')
}

export function useTheme(themeOverride?: Ref<Theme> | ComputedRef<Theme>) {
  const configStore = useConfigStore()
  const theme = computed(() => themeOverride?.value || configStore.theme)
  const cssVars = computed(() => themeToCssVars(theme.value))
  const fontLinks = computed(() => theme.value.fonts.google_fonts.map(font => ({
    rel: 'stylesheet',
    href: `https://fonts.googleapis.com/css2?family=${encodeURIComponent(font).replace(/%3A/g, ':').replace(/%3B/g, ';').replace(/%40/g, '@')}&display=swap`
  })))

  useHead(() => ({
    titleTemplate: title => title ? `${title} · ${theme.value.branding.site_name}` : theme.value.branding.site_name,
    link: [{ rel: 'icon', href: theme.value.branding.favicon }, ...fontLinks.value],
    style: [{ key: 'theme-vars', innerHTML: `:root{${varsToStyle(cssVars.value)}}` }]
  }))

  return { theme, cssVars, styleText: computed(() => varsToStyle(cssVars.value)) }
}
