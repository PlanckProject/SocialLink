import { DEFAULT_THEME, type Theme } from '~/app.config'
import { deepMerge } from '~/stores/config'

export const PRESET_THEMES: Theme[] = [
  DEFAULT_THEME,
  {
    ...DEFAULT_THEME,
    name: 'Verdant',
    colors: { background: '#d4e2d7', surface: '#dcecdf', surface_alt: '#cfe0d3', text: '#1b2a22', text_muted: '#546760', primary: '#4e9d83', primary_contrast: '#ffffff', accent: '#8a5a34', border: 'rgba(48,82,64,0.20)' },
    fonts: { ...DEFAULT_THEME.fonts },
    button: { variant: 'glass', shadow: '0 8px 22px rgba(26,44,35,0.10)', hover_lift: true },
    background: {
      ...DEFAULT_THEME.background,
      type: 'gradient',
      value: '#d4e2d7',
      gradient: 'radial-gradient(120% 90% at 15% -8%,#c6e3ce 0%,transparent 48%),radial-gradient(120% 92% at 85% 4%,#d6cbad 0%,transparent 46%),radial-gradient(130% 100% at 50% 110%,#cae6da 0%,transparent 55%),#d4e2d7',
      image: null,
      overlay: 'rgba(246,251,247,0.05)',
      shapes: { ...DEFAULT_THEME.background.shapes, opacity: 0.16, blur: 100, seed: 44 }
    },
    effects: { ...DEFAULT_THEME.effects, glass: true, glass_opacity: 60, glass_blur: 22 }
  },
  {
    ...DEFAULT_THEME,
    name: 'Aurora',
    colors: { ...DEFAULT_THEME.colors, primary: '#10b981', accent: '#f472b6', surface: 'rgba(16,24,39,0.72)', surface_alt: 'rgba(31,41,55,0.82)' },
    background: {
      ...DEFAULT_THEME.background,
      type: 'gradient',
      value: '#020617',
      gradient: 'radial-gradient(circle at 20% 10%,#134e4a 0%,transparent 35%),radial-gradient(circle at 80% 0%,#831843 0%,transparent 28%),#020617',
      image: null,
      overlay: 'rgba(2,6,23,0.42)',
      shapes: { ...DEFAULT_THEME.background.shapes, opacity: 0.22, seed: 73 }
    }
  },
  {
    ...DEFAULT_THEME,
    name: 'Midnight',
    colors: { background: '#0b0b0f', surface: '#15151d', surface_alt: '#1d1d27', text: '#f5f5f7', text_muted: '#a1a1aa', primary: '#7c3aed', primary_contrast: '#ffffff', accent: '#22d3ee', border: 'rgba(255,255,255,0.08)' },
    button: { variant: 'glass', shadow: '0 12px 32px rgba(4,2,20,0.42)', hover_lift: true },
    background: {
      ...DEFAULT_THEME.background,
      type: 'gradient',
      value: '#0b0b0f',
      gradient: 'linear-gradient(165deg,#241d52 0%,#120f26 52%,#08070c 100%)',
      image: null,
      overlay: 'rgba(0,0,0,0.35)',
      shapes: { ...DEFAULT_THEME.background.shapes, opacity: 0.2, blur: 96, seed: 17 }
    },
    effects: { ...DEFAULT_THEME.effects, glass: false, glass_opacity: 72, glass_blur: 18 }
  },
  {
    ...DEFAULT_THEME,
    name: 'Bubblegum',
    colors: { background: '#ffd9ec', surface: '#fff0f7', surface_alt: '#ffe3f1', text: '#4a1533', text_muted: '#9c5a80', primary: '#ec4899', primary_contrast: '#ffffff', accent: '#0ea5e9', border: 'rgba(190,60,120,0.18)' },
    button: { variant: 'glass', shadow: '0 8px 22px rgba(214,51,132,0.15)', hover_lift: true },
    background: {
      ...DEFAULT_THEME.background,
      type: 'gradient',
      value: '#ffd9ec',
      gradient: 'radial-gradient(120% 90% at 15% -8%,#ffc4e2 0%,transparent 48%),radial-gradient(120% 92% at 85% 4%,#cfe4ff 0%,transparent 46%),radial-gradient(130% 100% at 50% 110%,#ffd4ef 0%,transparent 55%),#ffd9ec',
      image: null,
      overlay: 'rgba(255,247,251,0.05)',
      shapes: { ...DEFAULT_THEME.background.shapes, opacity: 0.18, blur: 100, seed: 88 }
    },
    effects: { ...DEFAULT_THEME.effects, glass: true, glass_opacity: 64, glass_blur: 20 }
  }
]

// Backfill any fields a stored theme predates (e.g. effects.glass_*, ungrouped)
// from DEFAULT_THEME, then JSON-copy so the result shares no references with the
// defaults constant (the admin editor mutates it via v-model).
export const cloneTheme = (theme: Theme): Theme => JSON.parse(JSON.stringify(deepMerge(DEFAULT_THEME, theme))) as Theme
