import type { Theme } from '~/app.config'

/// Client-side event bus that broadcasts a public-config/theme change so the
/// whole UI can refresh the moment the admin saves or activates a theme.
export function useConfigBus() {
  return useEventBus<Theme>('config:updated')
}
