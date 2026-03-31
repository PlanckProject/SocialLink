import { DEFAULT_THEME, type Theme } from '~/app.config'

export const PRESET_THEMES: Theme[] = [
  DEFAULT_THEME,
  {
    ...DEFAULT_THEME,
    name: 'Daylight',
    colors: { background: '#f7f3ee', surface: '#ffffff', surface_alt: '#f0e9df', text: '#171412', text_muted: '#746b61', primary: '#2563eb', primary_contrast: '#ffffff', accent: '#f97316', border: 'rgba(23,20,18,0.12)' },
    background: {
      ...DEFAULT_THEME.background,
      type: 'gradient',
      value: '#f7f3ee',
      gradient: 'linear-gradient(160deg,#fff7ed 0%,#eef2ff 100%)',
      image: null,
      overlay: 'rgba(255,255,255,0.25)',
      shapes: { ...DEFAULT_THEME.background.shapes, opacity: 0.18, seed: 31 }
    },
    button: { ...DEFAULT_THEME.button, variant: 'soft', shadow: '0 10px 30px rgba(37,99,235,0.12)' }
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
      shapes: { ...DEFAULT_THEME.background.shapes, opacity: 0.34, seed: 73 }
    }
  }
]

export const cloneTheme = (theme: Theme): Theme => JSON.parse(JSON.stringify(theme)) as Theme
