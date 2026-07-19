import type { GroupStyle } from '~/types/social'

export interface Theme {
  name: string
  colors: {
    background: string
    surface: string
    surface_alt: string
    text: string
    text_muted: string
    primary: string
    primary_contrast: string
    accent: string
    border: string
  }
  fonts: {
    heading: string
    body: string
    google_fonts: string[]
  }
  radius: {
    background: string
    avatar: string
    social_icon: string
  }
  layout: {
    max_width: string
    cover_height: string
    align: 'left' | 'center' | 'right'
  }
  button: {
    variant: 'solid' | 'outline' | 'glass' | 'soft'
    shadow: string
    hover_lift: boolean
  }
  background: {
    type: 'solid' | 'gradient' | 'image'
    value: string
    gradient: string
    image: string | null
    overlay: string
    shapes: {
      enabled: boolean
      count: number
      opacity: number
      blur: number
      min_size: number
      max_size: number
      seed: number
    }
  }
  effects: {
    cover_fade: boolean
    cover_parallax: boolean
    glass: boolean
    glass_opacity: number
    glass_blur: number
  }
  ungrouped: GroupStyle
  features: {
    show_view_count: boolean
    show_click_count: boolean
    show_cover_photo: boolean
  }
}

/** Site-identity branding. Owned by the profile (not the theme) so it stays
 *  unchanged when a theme is applied, saved or imported. */
export interface Branding {
  site_name: string
  logo: string | null
  favicon: string
  footer_text: string
  show_footer: boolean
}

export const DEFAULT_BRANDING: Branding = {
  site_name: 'SocialLink',
  logo: null,
  favicon: '/favicon.ico',
  footer_text: 'Made with <a href="https://github.com/PlanckProject/SocialLink" target="_blank" rel="noopener noreferrer">SocialLink</a>',
  show_footer: true
}

export const DEFAULT_THEME: Theme = {
  name: 'Tidewater',
  colors: { background: '#06121a', surface: '#0e2229', surface_alt: '#123039', text: '#e9f2f1', text_muted: '#93aead', primary: '#2f80c2', primary_contrast: '#ffffff', accent: '#15a58f', border: 'rgba(160,225,215,0.10)' },
  fonts: { heading: "'Inter', system-ui, sans-serif", body: "'Inter', system-ui, sans-serif", google_fonts: ['Inter:wght@400;500;600;700'] },
  radius: {
    background: '20%',
    avatar: '50%',
    social_icon: '50%'
  },
  layout: { max_width: '620px', cover_height: '340px', align: 'center' },
  button: { variant: 'glass', shadow: '0 12px 30px rgba(2,14,18,0.45)', hover_lift: true },
  background: {
    type: 'gradient',
    value: '#06121a',
    gradient: 'linear-gradient(170deg,#103a45 0%,#0b2732 52%,#06121a 100%)',
    image: null,
    overlay: 'rgba(3,12,16,0.34)',
    shapes: { enabled: true, count: 6, opacity: 0.18, blur: 100, min_size: 180, max_size: 440, seed: 29 }
  },
  effects: { cover_fade: true, cover_parallax: true, glass: false, glass_opacity: 72, glass_blur: 18 },
  ungrouped: { layout: 'list', link_radius: '50%', icon_radius: '50%', spacing: '12px', title_align: 'left' },
  features: { show_view_count: false, show_click_count: false, show_cover_photo: true }
}

export default defineAppConfig({ theme: DEFAULT_THEME })
