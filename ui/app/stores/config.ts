import { defineStore } from 'pinia'
import type { Theme } from '~/app.config'

type Mode = 'single' | 'multi'
interface PublicConfig { mode: Mode; features: { registration_enabled: boolean }; theme?: Partial<Theme> }
interface ThemeOverride { key: string; theme: Theme }

const isRecord = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value)
const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T

export function deepMerge<T>(base: T, override?: Partial<T>): T {
  if (!override) return copy(base)
  const output: Record<string, unknown> = { ...(base as Record<string, unknown>) }
  for (const [key, value] of Object.entries(override as Record<string, unknown>)) {
    const current = output[key]
    output[key] = isRecord(current) && isRecord(value) ? deepMerge(current, value) : value
  }
  return output as T
}

export const useConfigStore = defineStore('config', () => {
  const appConfig = useAppConfig()
  const resolvedThemeState = useState<Theme>('resolved-theme', () => deepMerge(appConfig.theme as Theme))
  const themeOverrideState = useState<ThemeOverride | null>('theme-override', () => null)
  const theme = computed(() => themeOverrideState.value?.theme || resolvedThemeState.value)
  const mode = useState<Mode>('site-mode', () => 'single')
  const registration_enabled = useState<boolean>('registration-enabled', () => false)
  const loaded = ref(false)

  async function loadPublicConfig(force = false) {
    if (loaded.value && !force) return
    try {
      const config = await apiFetch<PublicConfig>('/api/config')
      mode.value = config.mode || 'single'
      registration_enabled.value = !!config.features?.registration_enabled
      resolvedThemeState.value = deepMerge(appConfig.theme as Theme, config.theme as Partial<Theme>)
    } catch {
      if (!loaded.value) resolvedThemeState.value = deepMerge(appConfig.theme as Theme)
    } finally {
      loaded.value = true
    }
  }

  async function reloadPublicConfig() { await loadPublicConfig(true) }

  function setTheme(theme: Theme) { resolvedThemeState.value = deepMerge(appConfig.theme as Theme, theme) }
  function setThemeOverride(key: string, theme: Theme) {
    themeOverrideState.value = { key, theme: deepMerge(appConfig.theme as Theme, theme) }
  }
  function clearThemeOverride(key: string) {
    if (themeOverrideState.value?.key === key) themeOverrideState.value = null
  }

  return {
    mode,
    registration_enabled,
    theme,
    loaded,
    loadPublicConfig,
    reloadPublicConfig,
    setTheme,
    setThemeOverride,
    clearThemeOverride
  }
})
