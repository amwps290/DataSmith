const STORAGE_KEYS = {
  SQL_HISTORY: 'sql_history',
  REDIS_HISTORY: 'redis_command_history',
  SAVED_QUERIES: 'saved_queries',
  CODE_SNIPPETS: 'sql-snippets',
  APP_SETTINGS: 'app_settings',
  QUERY_CATEGORIES: 'query_categories',
} as const

export function getStorageItem<T>(key: string, defaultValue: T): T {
  try {
    const item = localStorage.getItem(key)
    return item ? JSON.parse(item) : defaultValue
  } catch { return defaultValue }
}

export function setStorageItem(key: string, value: unknown): void {
  localStorage.setItem(key, JSON.stringify(value))
}

export function removeStorageItem(key: string): void {
  localStorage.removeItem(key)
}

export { STORAGE_KEYS }
