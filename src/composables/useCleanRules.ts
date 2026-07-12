/**
 * 智能清理规则引擎
 * @module composables/useCleanRules
 * @description 提供专业的重复文件清理规则、安全检查和智能决策
 */

import { ref, computed } from 'vue'
import type { FileInfo, DuplicateGroup } from '../types'
// formatSize 导入保留供后续使用

// 清理规则类型
export type CleanRuleType = 
  | 'keep-newest' 
  | 'keep-oldest' 
  | 'keep-shortest-path'
  | 'keep-longest-path'
  | 'keep-largest'
  | 'keep-smallest'
  | 'keep-by-location'
  | 'keep-by-pattern'
  | 'smart-score'

// 规则优先级
export type RulePriority = 'high' | 'medium' | 'low'

// 清理规则配置
export interface CleanRule {
  id: string
  type: CleanRuleType
  name: string
  description: string
  enabled: boolean
  priority: RulePriority
  params?: Record<string, any>
}

// 安全检查结果
export interface SafetyCheckResult {
  passed: boolean
  level: 'safe' | 'warning' | 'danger'
  message: string
  details: string[]
}

// 文件评分结果
export interface FileScore {
  file: FileInfo
  score: number
  reasons: string[]
  warnings: string[]
}

// 清理方案
export interface CleanPlan {
  group: DuplicateGroup
  keepFile: FileScore
  deleteFiles: FileScore[]
  safety: SafetyCheckResult
  spaceToFree: number
  confidence: number // 0-100
}

// 默认规则集
export const defaultRules: CleanRule[] = [
  {
    id: 'rule-1',
    type: 'keep-newest',
    name: '保留最新版本',
    description: '保留修改时间最新的文件，删除旧版本',
    enabled: true,
    priority: 'high'
  },
  {
    id: 'rule-2',
    type: 'keep-shortest-path',
    name: '保留根目录文件',
    description: '保留路径最短的文件（通常更接近根目录，更易于访问）',
    enabled: true,
    priority: 'medium'
  },
  {
    id: 'rule-3',
    type: 'smart-score',
    name: '智能评分系统',
    description: '综合文件名、路径、修改时间等多维度评分',
    enabled: true,
    priority: 'high',
    params: {
      timeWeight: 0.4,
      pathWeight: 0.3,
      nameWeight: 0.2,
      sizeWeight: 0.1
    }
  }
]

// 危险路径模式
const dangerousPatterns = [
  /\\Windows\\/i,
  /\\Program Files\\/i,
  /\\ProgramData\\/i,
  /\\System32\\/i,
  /\\SysWOW64\\/i,
  /\\Users\\[^\\]+\\AppData\\Local\\Temp\\/i,
  /\\\.git\\/i,
  /\\node_modules\\/i,
  /\\vendor\\/i
]

// 系统文件扩展名
const systemExtensions = ['sys', 'dll', 'exe', 'bat', 'cmd', 'msi', 'reg']

/**
 * 清理规则引擎组合式函数
 */
export function useCleanRules() {
  const rules = ref<CleanRule[]>([...defaultRules])
  const isAnalyzing = ref(false)

  // 启用的规则
  const enabledRules = computed(() => 
    rules.value.filter(r => r.enabled).sort((a, b) => {
      const priorityOrder = { high: 0, medium: 1, low: 2 }
      return priorityOrder[a.priority] - priorityOrder[b.priority]
    })
  )

  /**
   * 执行安全检查
   */
  function performSafetyCheck(group: DuplicateGroup): SafetyCheckResult {
    const issues: string[] = []
    let level: SafetyCheckResult['level'] = 'safe'

    for (const file of group.files) {
      // 检查危险路径
      for (const pattern of dangerousPatterns) {
        if (pattern.test(file.path)) {
          issues.push(`文件位于系统目录: ${file.filename}`)
          level = 'danger'
          break
        }
      }

      // 检查系统文件
      if (systemExtensions.includes(file.file_extension.toLowerCase())) {
        issues.push(`系统文件: ${file.filename}`)
        level = level === 'safe' ? 'warning' : level
      }

      // 检查文件锁定
      if (file.size === 0) {
        issues.push(`空文件: ${file.filename}`)
      }
    }

    // 检查组内文件数量
    if (group.files.length < 2) {
      issues.push('文件组内文件数量不足')
      level = 'danger'
    }

    return {
      passed: level !== 'danger',
      level,
      message: level === 'safe' ? '安全检查通过' : 
               level === 'warning' ? '存在警告，建议谨慎操作' : 
               '存在危险，不建议清理',
      details: issues
    }
  }

  /**
   * 计算文件评分（智能评分算法）
   */
  function calculateFileScore(file: FileInfo, allFiles: FileInfo[]): FileScore {
    let score = 50 // 基础分
    const reasons: string[] = []
    const warnings: string[] = []

    // 1. 时间因素（40%）- 越新越好
    const now = Date.now()
    const fileTime = new Date(file.modified_at).getTime()
    const timeDiff = now - fileTime
    const daysOld = timeDiff / (1000 * 60 * 60 * 24)
    
    if (daysOld < 7) {
      score += 20
      reasons.push('最近7天内修改')
    } else if (daysOld < 30) {
      score += 10
      reasons.push('最近30天内修改')
    } else if (daysOld < 90) {
      score += 0
    } else {
      score -= 10
      warnings.push('超过90天未修改')
    }

    // 2. 路径因素（30%）- 越短越好（通常更接近根目录）
    const pathLength = file.path.length
    const avgPathLength = allFiles.reduce((sum, f) => sum + f.path.length, 0) / allFiles.length
    
    if (pathLength < avgPathLength * 0.8) {
      score += 15
      reasons.push('路径较短，易于访问')
    } else if (pathLength > avgPathLength * 1.2) {
      score -= 10
      warnings.push('路径较深')
    }

    // 3. 文件名因素（20%）
    const fileName = file.filename.toLowerCase()
    
    // 检查是否有版本号（通常更规范）
    if (/v?\d+(\.\d+)*|version|ver/i.test(fileName)) {
      score += 10
      reasons.push('文件名包含版本信息')
    }
    
    // 检查是否有描述性关键词
    if (/backup|copy|副本|复制/i.test(fileName)) {
      score -= 15
      warnings.push('文件名包含复制标记')
    }

    // 4. 文件大小因素（10%）- 在重复文件中，大小应该相同，这里作为辅助判断
    const sizes = allFiles.map(f => f.size)
    const maxSize = Math.max(...sizes)
    const minSize = Math.min(...sizes)
    
    if (maxSize !== minSize) {
      // 如果不是完全重复，按大小评分
      const sizeRatio = file.size / maxSize
      if (sizeRatio > 0.95) {
        score += 5
        reasons.push('文件完整度高')
      }
    }

    // 额外安全检查扣分
    for (const pattern of dangerousPatterns) {
      if (pattern.test(file.path)) {
        score -= 50
        warnings.push('位于系统目录')
        break
      }
    }

    // 确保分数在0-100之间
    score = Math.max(0, Math.min(100, score))

    return {
      file,
      score,
      reasons,
      warnings
    }
  }

  /**
   * 应用清理规则
   */
  function applyRules(group: DuplicateGroup): { keep: FileScore; delete: FileScore[] } {
    const files = group.files
    
    // 为所有文件评分
    const scoredFiles = files.map(f => calculateFileScore(f, files))
    
    // 按分数排序（高到低）
    scoredFiles.sort((a, b) => b.score - a.score)
    
    // 选择最高分文件保留
    const keepFile = scoredFiles[0]
    const deleteFiles = scoredFiles.slice(1)

    return { keep: keepFile, delete: deleteFiles }
  }

  /**
   * 生成清理方案
   */
  function generateCleanPlan(groups: DuplicateGroup[]): CleanPlan[] {
    isAnalyzing.value = true
    
    try {
      const plans: CleanPlan[] = []

      for (const group of groups) {
        // 安全检查
        const safety = performSafetyCheck(group)
        
        // 如果安全检查不通过，跳过
        if (!safety.passed) {
          continue
        }

        // 应用规则
        const { keep, delete: toDelete } = applyRules(group)

        // 计算释放空间
        const spaceToFree = toDelete.reduce((sum, f) => sum + f.file.size, 0)

        // 计算置信度
        const confidence = Math.min(100, keep.score + 20)

        plans.push({
          group,
          keepFile: keep,
          deleteFiles: toDelete,
          safety,
          spaceToFree,
          confidence
        })
      }

      // 按释放空间排序
      plans.sort((a, b) => b.spaceToFree - a.spaceToFree)

      return plans
    } finally {
      isAnalyzing.value = false
    }
  }

  /**
   * 添加自定义规则
   */
  function addRule(rule: Omit<CleanRule, 'id'>): CleanRule {
    const newRule: CleanRule = {
      ...rule,
      id: `rule-${Date.now()}`
    }
    rules.value.push(newRule)
    return newRule
  }

  /**
   * 删除规则
   */
  function removeRule(ruleId: string) {
    rules.value = rules.value.filter(r => r.id !== ruleId)
  }

  /**
   * 更新规则
   */
  function updateRule(ruleId: string, updates: Partial<CleanRule>) {
    const index = rules.value.findIndex(r => r.id === ruleId)
    if (index > -1) {
      rules.value[index] = { ...rules.value[index], ...updates }
    }
  }

  /**
   * 重置为默认规则
   */
  function resetRules() {
    rules.value = [...defaultRules]
  }

  return {
    rules,
    enabledRules,
    isAnalyzing,
    performSafetyCheck,
    calculateFileScore,
    generateCleanPlan,
    addRule,
    removeRule,
    updateRule,
    resetRules
  }
}
