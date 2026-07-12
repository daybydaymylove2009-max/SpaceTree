import { ref } from 'vue'

const enableGlassEffect = ref(true)

/**
 * 渲染性能与视觉特效管理组合式函数
 */
export function usePerformance() {
  /**
   * 应用视觉特效状态到 DOM
   */
  function applyPerformanceSettings(enabled: boolean) {
    enableGlassEffect.value = enabled
    localStorage.setItem('enable-glass-effect', enabled ? 'true' : 'false')
    
    if (enabled) {
      document.documentElement.classList.remove('perf-low')
    } else {
      document.documentElement.classList.add('perf-low')
    }
  }

  /**
   * 性能基准测试与初始化
   */
  function initPerformance() {
    const saved = localStorage.getItem('enable-glass-effect')
    if (saved !== null) {
      applyPerformanceSettings(saved === 'true')
    } else {
      // 首次运行：进行微型帧率检测
      if (typeof window === 'undefined' || !window.requestAnimationFrame) {
        applyPerformanceSettings(true)
        return
      }

      let frameCount = 0
      const start = performance.now()
      
      function checkFrame() {
        frameCount++
        if (performance.now() - start < 100) {
          requestAnimationFrame(checkFrame)
        } else {
          // 计算 100ms 内的帧率，乘以 10 估算实际 FPS
          const fps = frameCount * 10
          console.log(`[Performance Benchmark] WebView 测得帧率为: ${fps} FPS`)
          
          // 如果帧率低于 45 帧，通常说明是没有显卡加速的低配置环境，或者旧版 WebView2，智能退化特效
          if (fps < 45) {
            console.warn('[Performance Benchmark] 帧率低于 45，智能设为低端显示模式（无毛玻璃效果）')
            applyPerformanceSettings(false)
          } else {
            applyPerformanceSettings(true)
          }
        }
      }
      
      requestAnimationFrame(checkFrame)
    }
  }

  return {
    enableGlassEffect,
    setGlassEffect: applyPerformanceSettings,
    initPerformance
  }
}
