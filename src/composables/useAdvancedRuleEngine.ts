/**
 * 高级规则引擎
 * @module composposables/useAdvancedRuleEngine
 * @description 支持复杂条件表达式的专业级规则引擎
 */

import { ref, computed } from 'vue'
import type { FileInfo } from '../types'

// 规则条件操作符
export type ConditionOperator = 
  | 'equals'           // 等于
  | 'notEquals'        // 不等于
  | 'contains'         // 包含
  | 'notContains'      // 不包含
  | 'startsWith'       // 开头是
  | 'endsWith'         // 结尾是
  | 'greaterThan'      // 大于
  | 'lessThan'         // 小于
  | 'greaterOrEqual'   // 大于等于
  | 'lessOrEqual'      // 小于等于
  | 'regex'            // 正则匹配
  | 'in'               // 在列表中
  | 'notIn'            // 不在列表中
  | 'between'          // 在范围内

// 逻辑操作符
export type LogicalOperator = 'AND' | 'OR' | 'NOT'

// 规则条件
export interface RuleCondition {
  id: string
  field: 'filename' | 'path' | 'extension' | 'size' | 'modifiedAt' | 'createdAt' | 'content'
  operator: ConditionOperator
  value: any
  value2?: any  // 用于 between 操作符
  caseSensitive?: boolean
}

// 复杂条件组
export interface ConditionGroup {
  id: string
  operator: LogicalOperator
  conditions: (RuleCondition | ConditionGroup)[]
}

// 规则动作
export type RuleAction = 'keep' | 'delete' | 'flag' | 'score'

// 高级规则
export interface AdvancedRule {
  id: string
  name: string
  description: string
  enabled: boolean
  priority: number  // 1-100，数字越大优先级越高
  condition: ConditionGroup
  action: RuleAction
  actionParams?: {
    score?: number           // 用于 score 动作
    flagColor?: string       // 用于 flag 动作
    flagMessage?: string     // 用于 flag 动作
  }
}

// 规则执行结果
export interface RuleExecutionResult {
  rule: AdvancedRule
  matched: boolean
  action: RuleAction
  score?: number
  flags?: string[]
  reason: string
}

// 预定义规则模板
export const ruleTemplates: AdvancedRule[] = [
  {
    id: 'template-keep-latest',
    name: '保留最新文件',
    description: '在重复文件中保留修改时间最新的',
    enabled: true,
    priority: 80,
    condition: {
      id: 'cond-1',
      operator: 'AND',
      conditions: [{
        id: 'cond-1-1',
        field: 'modifiedAt',
        operator: 'greaterThan',
        value: 'relative:max'  // 特殊值：相对于组内最大值
      }]
    },
    action: 'keep',
    actionParams: { score: 90 }
  },
  {
    id: 'template-avoid-backup',
    name: '避免备份文件',
    description: '标记包含backup、copy等关键词的文件为低优先级',
    enabled: true,
    priority: 70,
    condition: {
      id: 'cond-2',
      operator: 'OR',
      conditions: [
        {
          id: 'cond-2-1',
          field: 'filename',
          operator: 'regex',
          value: '(?i)(backup|copy|副本|复制|bak)'
        },
        {
          id: 'cond-2-2',
          field: 'path',
          operator: 'regex',
          value: '(?i)(backup|copy|副本|复制)'
        }
      ]
    },
    action: 'score',
    actionParams: { score: -20 }
  },
  {
    id: 'template-prefer-root',
    name: '偏好根目录文件',
    description: '路径越短（越接近根目录）的文件优先级越高',
    enabled: true,
    priority: 60,
    condition: {
      id: 'cond-3',
      operator: 'AND',
      conditions: [{
        id: 'cond-3-1',
        field: 'path',
        operator: 'lessThan',
        value: 'relative:avg'  // 路径长度小于平均值
      }]
    },
    action: 'score',
    actionParams: { score: 15 }
  },
  {
    id: 'template-protect-system',
    name: '保护系统文件',
    description: '标记系统目录和系统文件为危险',
    enabled: true,
    priority: 100,  // 最高优先级
    condition: {
      id: 'cond-4',
      operator: 'OR',
      conditions: [
        {
          id: 'cond-4-1',
          field: 'path',
          operator: 'regex',
          value: '(?i)(\\\\Windows\\\\|\\\\Program Files\\\\|\\\\System32\\\\|\\\\SysWOW64\\\\)'
        },
        {
          id: 'cond-4-2',
          field: 'extension',
          operator: 'in',
          value: ['sys', 'dll', 'exe', 'msi', 'reg']
        }
      ]
    },
    action: 'flag',
    actionParams: { flagColor: 'red', flagMessage: '系统文件，不建议删除' }
  }
]

/**
 * 高级规则引擎组合式函数
 */
export function useAdvancedRuleEngine() {
  const rules = ref<AdvancedRule[]>([...ruleTemplates])
  const executionLog = ref<RuleExecutionResult[]>([])

  // 启用的规则，按优先级排序
  const activeRules = computed(() => 
    rules.value
      .filter(r => r.enabled)
      .sort((a, b) => b.priority - a.priority)
  )

  /**
   * 评估单个条件
   */
  function evaluateCondition(condition: RuleCondition, file: FileInfo, context?: any): boolean {
    let fieldValue: any

    // 获取字段值
    switch (condition.field) {
      case 'filename':
        fieldValue = file.filename
        break
      case 'path':
        fieldValue = file.path
        break
      case 'extension':
        fieldValue = file.file_extension
        break
      case 'size':
        fieldValue = file.size
        break
      case 'modifiedAt':
        fieldValue = new Date(file.modified_at).getTime()
        break
      case 'createdAt':
        fieldValue = new Date(file.created_at).getTime()
        break
      default:
        return false
    }

    // 处理特殊值
    let compareValue = condition.value
    if (typeof compareValue === 'string' && compareValue.startsWith('relative:')) {
      compareValue = context?.[compareValue.replace('relative:', '')]
    }

    // 执行比较
    switch (condition.operator) {
      case 'equals':
        return fieldValue === compareValue
      case 'notEquals':
        return fieldValue !== compareValue
      case 'contains':
        return String(fieldValue).includes(compareValue)
      case 'notContains':
        return !String(fieldValue).includes(compareValue)
      case 'startsWith':
        return String(fieldValue).startsWith(compareValue)
      case 'endsWith':
        return String(fieldValue).endsWith(compareValue)
      case 'greaterThan':
        return fieldValue > compareValue
      case 'lessThan':
        return fieldValue < compareValue
      case 'greaterOrEqual':
        return fieldValue >= compareValue
      case 'lessOrEqual':
        return fieldValue <= compareValue
      case 'regex':
        try {
          const regex = new RegExp(compareValue, condition.caseSensitive ? '' : 'i')
          return regex.test(String(fieldValue))
        } catch {
          return false
        }
      case 'in':
        return Array.isArray(compareValue) && compareValue.includes(fieldValue)
      case 'notIn':
        return Array.isArray(compareValue) && !compareValue.includes(fieldValue)
      case 'between':
        return fieldValue >= compareValue && fieldValue <= condition.value2
      default:
        return false
    }
  }

  /**
   * 评估条件组
   */
  function evaluateConditionGroup(
    group: ConditionGroup, 
    file: FileInfo, 
    context?: any
  ): boolean {
    const results = group.conditions.map(cond => {
      if ('conditions' in cond) {
        // 嵌套条件组
        return evaluateConditionGroup(cond, file, context)
      } else {
        // 单个条件
        return evaluateCondition(cond, file, context)
      }
    })

    switch (group.operator) {
      case 'AND':
        return results.every(r => r)
      case 'OR':
        return results.some(r => r)
      case 'NOT':
        return !results.some(r => r)
      default:
        return false
    }
  }

  /**
   * 执行单条规则
   */
  function executeRule(
    rule: AdvancedRule, 
    file: FileInfo, 
    context?: any
  ): RuleExecutionResult {
    const matched = evaluateConditionGroup(rule.condition, file, context)
    
    return {
      rule,
      matched,
      action: matched ? rule.action : 'keep',
      score: matched && rule.action === 'score' ? rule.actionParams?.score : undefined,
      flags: matched && rule.action === 'flag' ? [rule.actionParams?.flagMessage || ''] : undefined,
      reason: matched ? `匹配规则: ${rule.name}` : '未匹配'
    }
  }

  /**
   * 执行所有规则
   */
  function executeAllRules(
    file: FileInfo, 
    context?: any
  ): { finalScore: number; flags: string[]; reasons: string[] } {
    let score = 50 // 基础分
    const flags: string[] = []
    const reasons: string[] = []

    for (const rule of activeRules.value) {
      const result = executeRule(rule, file, context)
      
      if (result.matched) {
        if (result.score !== undefined) {
          score += result.score
        }
        if (result.flags) {
          flags.push(...result.flags)
        }
        reasons.push(result.reason)
      }
    }

    // 限制分数范围
    score = Math.max(0, Math.min(100, score))

    return { finalScore: score, flags: [...new Set(flags)], reasons }
  }

  /**
   * 添加自定义规则
   */
  function addRule(rule: Omit<AdvancedRule, 'id'>): AdvancedRule {
    const newRule: AdvancedRule = {
      ...rule,
      id: `rule-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
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
  function updateRule(ruleId: string, updates: Partial<AdvancedRule>) {
    const index = rules.value.findIndex(r => r.id === ruleId)
    if (index > -1) {
      rules.value[index] = { ...rules.value[index], ...updates }
    }
  }

  /**
   * 启用/禁用规则
   */
  function toggleRule(ruleId: string) {
    const rule = rules.value.find(r => r.id === ruleId)
    if (rule) {
      rule.enabled = !rule.enabled
    }
  }

  /**
   * 重置为默认规则
   */
  function resetRules() {
    rules.value = [...ruleTemplates]
  }

  /**
   * 验证规则有效性
   */
  function validateRule(rule: AdvancedRule): { valid: boolean; errors: string[] } {
    const errors: string[] = []

    if (!rule.name.trim()) {
      errors.push('规则名称不能为空')
    }

    if (rule.priority < 1 || rule.priority > 100) {
      errors.push('优先级必须在 1-100 之间')
    }

    // 验证条件
    function validateCondition(cond: RuleCondition | ConditionGroup, path: string) {
      if ('conditions' in cond) {
        if (cond.conditions.length === 0) {
          errors.push(`${path}: 条件组不能为空`)
        }
        cond.conditions.forEach((c, i) => validateCondition(c, `${path}.conditions[${i}]`))
      } else {
        if (!cond.field) {
          errors.push(`${path}: 字段不能为空`)
        }
        if (!cond.operator) {
          errors.push(`${path}: 操作符不能为空`)
        }
        if (cond.value === undefined || cond.value === null) {
          errors.push(`${path}: 值不能为空`)
        }
      }
    }

    validateCondition(rule.condition, 'condition')

    return { valid: errors.length === 0, errors }
  }

  return {
    rules,
    activeRules,
    executionLog,
    evaluateCondition,
    evaluateConditionGroup,
    executeRule,
    executeAllRules,
    addRule,
    removeRule,
    updateRule,
    toggleRule,
    resetRules,
    validateRule
  }
}
