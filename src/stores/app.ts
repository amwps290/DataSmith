import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import i18n from '@/i18n'

export type Theme = 'light' | 'dark'
export type Language = 'zh-CN' | 'en-US'

export const useAppStore = defineStore('app', () => {
  // 主题
  const theme = ref<Theme>(localStorage.getItem('theme') as Theme || 'light')
  
  // 语言
  const language = ref<Language>(localStorage.getItem('language') as Language || 
    (navigator.language.startsWith('zh') ? 'zh-CN' : 'en-US'))

  // 侧边栏折叠状态
  const sidebarCollapsed = ref(false)

  // 监听主题变化并持久化
  watch(theme, (newTheme) => {
    localStorage.setItem('theme', newTheme)
  })

  // 监听语言变化并同步到 i18n 实例及持久化
  watch(language, (newLang) => {
    localStorage.setItem('language', newLang)
    // @ts-ignore
    i18n.global.locale.value = newLang
  }, { immediate: true })

  // 切换主题
  function toggleTheme() {
    theme.value = theme.value === 'light' ? 'dark' : 'light'
  }

  // 设置主题
  function setTheme(newTheme: Theme) {
    theme.value = newTheme
  }

  // 切换语言
  function toggleLanguage() {
    language.value = language.value === 'zh-CN' ? 'en-US' : 'zh-CN'
  }

  // 设置语言
  function setLanguage(newLang: Language) {
    language.value = newLang
  }

  // 切换侧边栏
  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  return {
    theme,
    language,
    sidebarCollapsed,
    toggleTheme,
    setTheme,
    toggleLanguage,
    setLanguage,
    toggleSidebar,
  }
})

