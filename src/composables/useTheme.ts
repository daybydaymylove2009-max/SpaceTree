/**
 * 主题管理组合式函数
 * @module composables/useTheme
 * @description 提供暗黑模式、主题切换功能
 */

import { ref, onMounted } from 'vue'

export type Theme = 'light' | 'dark' | 'auto'

const currentTheme = ref<Theme>('auto')
const isDark = ref(false)

/**
 * 主题管理组合式函数
 */
export function useTheme() {
  /**
   * 检测系统主题偏好
   */
  function detectSystemTheme(): boolean {
    if (typeof window === 'undefined') return false
    return window.matchMedia('(prefers-color-scheme: dark)').matches
  }

  /**
   * 应用主题到 DOM
   */
  function applyTheme(dark: boolean) {
    isDark.value = dark
    if (dark) {
      document.documentElement.classList.add('dark')
      document.documentElement.setAttribute('data-theme', 'dark')
    } else {
      document.documentElement.classList.remove('dark')
      document.documentElement.setAttribute('data-theme', 'light')
    }
  }

  /**
   * 设置主题
   */
  function setTheme(theme: Theme) {
    currentTheme.value = theme
    localStorage.setItem('app-theme', theme)

    if (theme === 'auto') {
      applyTheme(detectSystemTheme())
    } else {
      applyTheme(theme === 'dark')
    }
  }

  /**
   * 切换主题
   */
  function toggleTheme() {
    const newTheme = isDark.value ? 'light' : 'dark'
    setTheme(newTheme)
  }

  /**
   * 初始化主题
   */
  function initTheme() {
    const saved = localStorage.getItem('app-theme') as Theme | null
    if (saved) {
      setTheme(saved)
    } else {
      setTheme('auto')
    }

    // 监听系统主题变化
    if (typeof window !== 'undefined') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
      mediaQuery.addEventListener('change', (e) => {
        if (currentTheme.value === 'auto') {
          applyTheme(e.matches)
        }
      })
    }
  }

  onMounted(() => {
    initTheme()
  })

  return {
    currentTheme,
    isDark,
    setTheme,
    toggleTheme,
    initTheme
  }
}
