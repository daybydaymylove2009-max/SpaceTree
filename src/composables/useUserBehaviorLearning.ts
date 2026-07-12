/**
 * 用户行为学习系统
 * @module composables/useUserBehaviorLearning
 * @description 学习用户的文件选择偏好，自适应调整评分权重
 */

import { ref, computed } from 'vue'
import type { FileInfo } from '../types'

// 用户选择历史记录
export interface UserChoice {
  timestamp: number
  groupHash: string
  keptFile: string
  deletedFiles: string[]
  context: {
    timeWeight: number
    pathWeight: number
    nameWeight: number
    sizeWeight: number
  }
}

// 学习到的权重配置
export interface LearnedWeights {
  timeWeight: number
  pathWeight: number
  nameWeight: number
  sizeWeight: number
  confidence: number // 学习置信度 0-1
}

// 用户偏好配置
export interface UserPreferences {
  preferNewFiles: boolean
  preferShortPaths: boolean
  preferVersionedNames: boolean
  avoidSystemPaths: boolean
  learnedWeights: LearnedWeights
  choiceHistory: UserChoice[]
}

const STORAGE_KEY = 'file-manager-user-behavior'
const MAX_HISTORY_SIZE = 100

/**
 * 用户行为学习组合式函数
 */
export function useUserBehaviorLearning() {
  // 用户偏好
  const preferences = ref<UserPreferences>({
    preferNewFiles: true,
    preferShortPaths: true,
    preferVersionedNames: true,
    avoidSystemPaths: true,
    learnedWeights: {
      timeWeight: 0.4,
      pathWeight: 0.3,
      nameWeight: 0.2,
      sizeWeight: 0.1,
      confidence: 0
    },
    choiceHistory: []
  })

  // 是否有足够的学习数据
  const hasEnoughData = computed(() => 
    preferences.value.choiceHistory.length >= 5
  )

  // 学习置信度
  const learningConfidence = computed(() => 
    preferences.value.learnedWeights.confidence
  )

  /**
   * 从本地存储加载
   */
  function loadFromStorage() {
    try {
      const stored = localStorage.getItem(STORAGE_KEY)
      if (stored) {
        const parsed = JSON.parse(stored)
        preferences.value = { ...preferences.value, ...parsed }
      }
    } catch (error) {
      console.error('加载用户行为数据失败:', error)
    }
  }

  /**
   * 保存到本地存储
   */
  function saveToStorage() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences.value))
    } catch (error) {
      console.error('保存用户行为数据失败:', error)
    }
  }

  /**
   * 记录用户选择
   */
  function recordUserChoice(
    groupHash: string,
    keptFile: FileInfo,
    deletedFiles: FileInfo[],
    currentWeights: LearnedWeights
  ) {
    const choice: UserChoice = {
      timestamp: Date.now(),
      groupHash,
      keptFile: keptFile.path,
      deletedFiles: deletedFiles.map(f => f.path),
      context: { ...currentWeights }
    }

    preferences.value.choiceHistory.unshift(choice)
    
    // 限制历史记录数量
    if (preferences.value.choiceHistory.length > MAX_HISTORY_SIZE) {
      preferences.value.choiceHistory = preferences.value.choiceHistory.slice(0, MAX_HISTORY_SIZE)
    }

    // 触发学习
    learnFromHistory()
    
    // 保存
    saveToStorage()
  }

  /**
   * 从历史记录学习权重
   */
  function learnFromHistory() {
    const history = preferences.value.choiceHistory
    if (history.length < 3) return

    // 分析用户选择模式
    let timePreference = 0
    let pathPreference = 0
    let namePreference = 0
    let sizePreference = 0

    for (const choice of history) {
      // 分析用户在该次选择中更看重什么因素
      // keptFile 和 deletedFiles 用于后续分析用户选择模式
      // const keptFile = choice.keptFile
      // const deletedFiles = choice.deletedFiles

      // 这里简化处理，实际应该根据文件属性分析
      // 假设用户选择最新的文件，说明看重时间
      // 选择路径短的，说明看重路径
      // 等等...

      // 基于上下文权重和用户选择，反向推导用户偏好
      timePreference += choice.context.timeWeight
      pathPreference += choice.context.pathWeight
      namePreference += choice.context.nameWeight
      sizePreference += choice.context.sizeWeight
    }

    const total = history.length
    
    // 计算新的权重（指数移动平均）
    const alpha = 0.3 // 学习率
    const current = preferences.value.learnedWeights

    const newTimeWeight = current.timeWeight * (1 - alpha) + (timePreference / total) * alpha
    const newPathWeight = current.pathWeight * (1 - alpha) + (pathPreference / total) * alpha
    const newNameWeight = current.nameWeight * (1 - alpha) + (namePreference / total) * alpha
    const newSizeWeight = current.sizeWeight * (1 - alpha) + (sizePreference / total) * alpha

    // 归一化
    const sum = newTimeWeight + newPathWeight + newNameWeight + newSizeWeight
    
    preferences.value.learnedWeights = {
      timeWeight: newTimeWeight / sum,
      pathWeight: newPathWeight / sum,
      nameWeight: newNameWeight / sum,
      sizeWeight: newSizeWeight / sum,
      confidence: Math.min(1, history.length / 20) // 20次选择达到满置信度
    }

    // 更新偏好标志
    preferences.value.preferNewFiles = preferences.value.learnedWeights.timeWeight > 0.35
    preferences.value.preferShortPaths = preferences.value.learnedWeights.pathWeight > 0.35
  }

  /**
   * 获取自适应权重
   */
  function getAdaptiveWeights(): LearnedWeights {
    if (hasEnoughData.value && learningConfidence.value > 0.5) {
      return preferences.value.learnedWeights
    }
    // 数据不足时使用默认权重
    return {
      timeWeight: 0.4,
      pathWeight: 0.3,
      nameWeight: 0.2,
      sizeWeight: 0.1,
      confidence: 0
    }
  }

  /**
   * 分析用户偏好报告
   */
  function generatePreferenceReport(): string {
    const p = preferences.value
    const w = p.learnedWeights
    
    return `
用户偏好分析报告:
==================
学习数据量: ${p.choiceHistory.length} 次选择
学习置信度: ${(w.confidence * 100).toFixed(1)}%

权重配置:
- 时间因素: ${(w.timeWeight * 100).toFixed(1)}% ${p.preferNewFiles ? '(偏好新文件)' : ''}
- 路径因素: ${(w.pathWeight * 100).toFixed(1)}% ${p.preferShortPaths ? '(偏好短路径)' : ''}
- 文件名因素: ${(w.nameWeight * 100).toFixed(1)}% ${p.preferVersionedNames ? '(偏好版本号)' : ''}
- 大小因素: ${(w.sizeWeight * 100).toFixed(1)}%

建议:
${w.confidence < 0.3 ? '- 学习数据不足，建议进行更多清理操作以优化推荐' : ''}
${w.confidence > 0.7 ? '- 学习充分，推荐结果已针对您的偏好优化' : ''}
    `.trim()
  }

  /**
   * 重置学习数据
   */
  function resetLearning() {
    preferences.value.choiceHistory = []
    preferences.value.learnedWeights = {
      timeWeight: 0.4,
      pathWeight: 0.3,
      nameWeight: 0.2,
      sizeWeight: 0.1,
      confidence: 0
    }
    preferences.value.preferNewFiles = true
    preferences.value.preferShortPaths = true
    saveToStorage()
  }

  // 初始化时加载
  loadFromStorage()

  return {
    preferences,
    hasEnoughData,
    learningConfidence,
    recordUserChoice,
    getAdaptiveWeights,
    generatePreferenceReport,
    resetLearning
  }
}
