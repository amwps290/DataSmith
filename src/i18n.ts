import { createI18n } from 'vue-i18n'
import zhCN from './locales/zh-CN.json'
import enUS from './locales/en-US.json'
import type { Language } from '@/stores/app'

const i18n = createI18n({
  legacy: false, // 使用 Composition API
  locale: localStorage.getItem('language') || (navigator.language.startsWith('zh') ? 'zh-CN' : 'en-US'),
  fallbackLocale: 'en-US',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
  },
})

/**
 * 类型安全的语言切换函数
 */
export function setLocale(locale: Language) {
  // @ts-ignore - 内部切换由于 vue-i18n 类型复杂暂用 ignore
  i18n.global.locale.value = locale
}

export default i18n
