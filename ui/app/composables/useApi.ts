export const useApiHeaders = () => {
  const headers: Record<string, string> = {}
  if (import.meta.server) {
    const requestHeaders = useRequestHeaders(['cookie'])
    if (requestHeaders.cookie) headers.cookie = requestHeaders.cookie
  }
  return headers
}

interface ApiResponse<T> {
  request_id: string
  status: number
  message: string
  success: boolean
  data: T
}

function isApiResponse<T>(value: unknown): value is ApiResponse<T> {
  if (!value || typeof value !== 'object') return false
  const response = value as Record<string, unknown>
  return typeof response.request_id === 'string'
    && typeof response.status === 'number'
    && typeof response.message === 'string'
    && typeof response.success === 'boolean'
    && 'data' in response
}

export async function apiFetch<T>(url: string, options: Record<string, any> = {}): Promise<T> {
  const config = useRuntimeConfig()
  const response = await $fetch<ApiResponse<T> | T>(url, {
    baseURL: String(config.public.apiBase || ''),
    credentials: 'include',
    ...options,
    headers: { ...useApiHeaders(), ...(options.headers || {}) }
  })
  return isApiResponse<T>(response) ? response.data : response
}

export function useApiFetch<T>(url: string | (() => string), options: Record<string, any> = {}) {
  const config = useRuntimeConfig()
  return useFetch<ApiResponse<T> | T, unknown, T>(url, {
    baseURL: String(config.public.apiBase || ''),
    credentials: 'include',
    ...options,
    headers: { ...useApiHeaders(), ...(options.headers || {}) },
    transform: (response: ApiResponse<T> | T) =>
      isApiResponse<T>(response) ? response.data : response
  })
}
