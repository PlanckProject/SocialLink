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
  }
  branding: {
    site_name: string
    logo: string | null
    favicon: string
    footer_text: string
    show_footer: boolean
  }
  features: {
    show_view_count: boolean
    show_click_count: boolean
    show_cover_photo: boolean
  }
}

export const DEFAULT_THEME: Theme = {
  name: 'Midnight',
  colors: { background: '#0b0b0f', surface: '#15151d', surface_alt: '#1d1d27', text: '#f5f5f7', text_muted: '#a1a1aa', primary: '#7c3aed', primary_contrast: '#ffffff', accent: '#22d3ee', border: 'rgba(255,255,255,0.08)' },
  fonts: { heading: "'Inter', system-ui, sans-serif", body: "'Inter', system-ui, sans-serif", google_fonts: ['Inter:wght@400;500;600;700'] },
  radius: {
    background: '20%',
    avatar: '50%',
    social_icon: '50%'
  },
  layout: { max_width: '620px', cover_height: '340px', align: 'center' },
  button: { variant: 'glass', shadow: '0 8px 24px rgba(0,0,0,0.28)', hover_lift: true },
  background: {
    type: 'gradient',
    value: '#0b0b0f',
    gradient: 'linear-gradient(160deg,#1e1b4b 0%,#0b0b0f 60%)',
    image: null,
    overlay: 'rgba(0,0,0,0.35)',
    shapes: { enabled: true, count: 6, opacity: 0.28, blur: 90, min_size: 180, max_size: 420, seed: 17 }
  },
  effects: { cover_fade: true, cover_parallax: true },
  branding: { site_name: 'SocialLink', logo: null, favicon: '/favicon.ico', footer_text: 'Made with <a href="https://github.com/PlanckProject/SocialLink" target="_blank" rel="noopener noreferrer">SocialLink</a>', show_footer: true },
  features: { show_view_count: false, show_click_count: false, show_cover_photo: true }
}

export default defineAppConfig({ theme: DEFAULT_THEME })
