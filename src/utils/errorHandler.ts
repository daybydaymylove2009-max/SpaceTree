/**
 * 文件管理系统 - 专业级错误处理工具库
 * @module utils/errorHandler
 * @description 企业级错误处理，支持分类、恢复、上报、日志集成
 */

import { ElMessage, ElMessageBox } from 'element-plus'
import { logger } from './logger'

/**
 * 错误级别枚举
 */
export enum ErrorLevel {
  INFO = 'info',
  WARNING = 'warning',
  ERROR = 'error',
  FATAL = 'fatal'
}

/**
 * 错误分类枚举
 */
export enum ErrorCategory {
  SYSTEM = 'system',
  NETWORK = 'network',
  DATABASE = 'database',
  FILE_SYSTEM = 'file_system',
  VALIDATION = 'validation',
  PERMISSION = 'permission',
  RESOURCE = 'resource',
  UNKNOWN = 'unknown'
}

/**
 * 错误恢复策略
 */
export enum RecoveryStrategy {
  RETRY = 'retry',
  ROLLBACK = 'rollback',
  SKIP = 'skip',
  ABORT = 'abort',
  FALLBACK = 'fallback'
}

/**
 * 结构化错误对象
 */
export interface StructuredError {
  message: string
  code: string
  level: ErrorLevel
  category: ErrorCategory
  details?: string
  timestamp: number
  context?: string
  recoverable: boolean
  recoveryStrategy?: RecoveryStrategy
  originalError?: any
  stack?: string
}

/**
 * 应用错误类
 */
export class AppError extends Error {
  public level: ErrorLevel
  public code: string
  public category: ErrorCategory
  public details?: string
  public recoverable: boolean
  public recoveryStrategy?: RecoveryStrategy
  public timestamp: number
  public context?: string

  constructor(
    message: string,
    code: string,
    level: ErrorLevel = ErrorLevel.ERROR,
    category: ErrorCategory = ErrorCategory.UNKNOWN,
    options: {
      details?: string
      recoverable?: boolean
      recoveryStrategy?: RecoveryStrategy
      context?: string
      originalError?: any
    } = {}
  ) {
    super(message)
    this.name = 'AppError'
    this.code = code
    this.level = level
    this.category = category
    this.timestamp = Date.now()
    this.details = options.details
    this.recoverable = options.recoverable ?? false
    this.recoveryStrategy = options.recoveryStrategy
    this.context = options.context

    if (options.originalError?.stack) {
      this.stack = options.originalError.stack
    }

    // 记录到日志系统
    this.log()
  }

  private log() {
    const logData = {
      code: this.code,
      category: this.category,
      recoverable: this.recoverable,
      context: this.context,
      details: this.details
    }

    switch (this.level) {
      case ErrorLevel.INFO:
        logger.info(this.message, this.context, logData)
        break
      case ErrorLevel.WARNING:
        logger.warn(this.message, this.context, logData, this)
        break
      case ErrorLevel.ERROR:
        logger.error(this.message, this.context, logData, this)
        break
      case ErrorLevel.FATAL:
        logger.fatal(this.message, this.context, logData, this)
        break
    }
  }

  toStructured(): StructuredError {
    return {
      message: this.message,
      code: this.code,
      level: this.level,
      category: this.category,
      details: this.details,
      timestamp: this.timestamp,
      context: this.context,
      recoverable: this.recoverable,
      recoveryStrategy: this.recoveryStrategy,
      stack: this.stack
    }
  }
}

/**
 * 错误消息映射表（支持国际化）
 */
const errorMessages: Record<string, { zh: string; en: string }> = {
  'PATH_NOT_SAFE': { zh: '路径不安全，操作被拒绝', en: 'Path is not safe, operation denied' },
  'FILE_LOCKED': { zh: '文件被锁定，无法操作', en: 'File is locked' },
  'DB_ERROR': { zh: '数据库操作失败', en: 'Database operation failed' },
  'DB_CONNECTION_LOST': { zh: '数据库连接丢失', en: 'Database connection lost' },
  'HASH_ERROR': { zh: '文件哈希计算失败', en: 'File hash calculation failed' },
  'SCAN_ERROR': { zh: '文件扫描失败', en: 'File scan failed' },
  'SCAN_INTERRUPTED': { zh: '扫描被中断', en: 'Scan interrupted' },
  'EXPORT_ERROR': { zh: '导出失败', en: 'Export failed' },
  'DELETE_ERROR': { zh: '删除文件失败', en: 'Delete file failed' },
  'DELETE_PARTIAL': { zh: '部分文件删除失败', en: 'Partial delete failed' },
  'PERMISSION_DENIED': { zh: '权限不足，请检查文件权限', en: 'Permission denied' },
  'FILE_NOT_FOUND': { zh: '文件不存在', en: 'File not found' },
  'FILE_TOO_LARGE': { zh: '文件过大', en: 'File too large' },
  'NETWORK_ERROR': { zh: '网络连接失败', en: 'Network error' },
  'TIMEOUT': { zh: '操作超时', en: 'Operation timeout' },
  'VALIDATION_ERROR': { zh: '数据验证失败', en: 'Validation error' },
  'RESOURCE_EXHAUSTED': { zh: '系统资源不足', en: 'Resource exhausted' },
  'UNKNOWN_ERROR': { zh: '未知错误', en: 'Unknown error' }
}

/**
 * 获取本地化错误消息
 */
export function getErrorMessage(code: string, lang: 'zh' | 'en' = 'zh'): string {
  return errorMessages[code]?.[lang] || errorMessages['UNKNOWN_ERROR'][lang]
}

/**
 * 错误分类器
 */
export function classifyError(error: any): ErrorCategory {
  if (!error) return ErrorCategory.UNKNOWN

  const message = String(error.message || error).toLowerCase()

  if (message.includes('network') || message.includes('connection') || message.includes('timeout')) {
    return ErrorCategory.NETWORK
  }
  if (message.includes('database') || message.includes('db') || message.includes('sql')) {
    return ErrorCategory.DATABASE
  }
  if (message.includes('file') || message.includes('path') || message.includes('directory')) {
    return ErrorCategory.FILE_SYSTEM
  }
  if (message.includes('permission') || message.includes('access') || message.includes('denied')) {
    return ErrorCategory.PERMISSION
  }
  if (message.includes('validation') || message.includes('invalid') || message.includes('required')) {
    return ErrorCategory.VALIDATION
  }
  if (message.includes('memory') || message.includes('resource') || message.includes('disk')) {
    return ErrorCategory.RESOURCE
  }

  return ErrorCategory.UNKNOWN
}

/**
 * 处理错误并显示用户友好的提示
 * @param error - 错误对象
 * @param context - 错误上下文信息
 * @param options - 处理选项
 */
export function handleError(
  error: unknown,
  context?: string,
  options: {
    showMessage?: boolean
    recoverable?: boolean
    recoveryStrategy?: RecoveryStrategy
    fallback?: () => void
  } = {}
): AppError {
  const { showMessage = true, recoverable = false, recoveryStrategy, fallback } = options

  let appError: AppError

  if (error instanceof AppError) {
    appError = error
  } else if (error instanceof Error) {
    const category = classifyError(error)
    appError = new AppError(
      error.message,
      'UNKNOWN_ERROR',
      ErrorLevel.ERROR,
      category,
      {
        context,
        recoverable,
        recoveryStrategy,
        originalError: error
      }
    )
  } else {
    const message = String(error)
    appError = new AppError(
      message,
      'UNKNOWN_ERROR',
      ErrorLevel.ERROR,
      ErrorCategory.UNKNOWN,
      { context, recoverable, recoveryStrategy }
    )
  }

  // 显示用户提示
  if (showMessage) {
    displayErrorMessage(appError)
  }

  // 执行恢复策略
  if (recoverable && recoveryStrategy) {
    executeRecovery(appError, recoveryStrategy, fallback)
  }

  return appError
}

/**
 * 显示错误消息
 */
function displayErrorMessage(error: AppError): void {
  const displayMessage = getErrorMessage(error.code) || error.message

  switch (error.level) {
    case ErrorLevel.INFO:
      ElMessage.info(displayMessage)
      break
    case ErrorLevel.WARNING:
      ElMessage.warning(displayMessage)
      break
    case ErrorLevel.ERROR:
      ElMessage.error(displayMessage)
      break
    case ErrorLevel.FATAL:
      ElMessageBox.alert(displayMessage, '严重错误', {
        type: 'error',
        confirmButtonText: '确定',
        dangerouslyUseHTMLString: true,
        message: error.details
          ? `<div>
              <p>${displayMessage}</p>
              <p style="color: #909399; font-size: 12px; margin-top: 8px;">${error.details}</p>
            </div>`
          : displayMessage
      })
      break
  }
}

/**
 * 执行恢复策略
 */
async function executeRecovery(
  error: AppError,
  strategy: RecoveryStrategy,
  fallback?: () => void
): Promise<void> {
  logger.info(`尝试恢复策略: ${strategy}`, 'ErrorRecovery', { error: error.toStructured() })

  switch (strategy) {
    case RecoveryStrategy.RETRY:
      // 延迟重试
      await new Promise(resolve => setTimeout(resolve, 1000))
      break

    case RecoveryStrategy.FALLBACK:
      fallback?.()
      break

    case RecoveryStrategy.SKIP:
      // 跳过当前操作，继续执行
      break

    case RecoveryStrategy.ROLLBACK:
      // 回滚操作（需要外部实现）
      break

    case RecoveryStrategy.ABORT:
      // 中止操作
      break
  }
}

/**
 * 包装异步函数，自动处理错误
 * @param fn - 异步函数
 * @param context - 错误上下文
 * @param options - 错误处理选项
 * @returns 包装后的函数
 */
export function withErrorHandling<T extends (...args: any[]) => Promise<any>>(
  fn: T,
  context?: string,
  options?: Parameters<typeof handleError>[2]
): (...args: Parameters<T>) => Promise<ReturnType<T> | undefined> {
  return async (...args: Parameters<T>): Promise<ReturnType<T> | undefined> => {
    try {
      return await fn(...args)
    } catch (error) {
      handleError(error, context, options)
      return undefined
    }
  }
}

/**
 * 创建特定上下文的错误处理器
 */
export function createErrorHandler(context: string, defaultOptions?: Parameters<typeof handleError>[2]) {
  return {
    handle: (error: unknown, options?: Parameters<typeof handleError>[2]) =>
      handleError(error, context, { ...defaultOptions, ...options }),

    wrap: <T extends (...args: any[]) => Promise<any>>(fn: T) =>
      withErrorHandling(fn, context, defaultOptions),

    createError: (
      message: string,
      code: string,
      level?: ErrorLevel,
      category?: ErrorCategory,
      options?: ConstructorParameters<typeof AppError>[4]
    ) => new AppError(message, code, level, category, { ...options, context })
  }
}

/**
 * 确认对话框包装
 * @param message - 确认消息
 * @param title - 对话框标题
 * @param type - 对话框类型
 * @returns 是否确认
 */
export async function confirmAction(
  message: string,
  title: string = '确认操作',
  type: 'warning' | 'error' | 'info' = 'warning'
): Promise<boolean> {
  try {
    await ElMessageBox.confirm(message, title, {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type
    })
    return true
  } catch {
    return false
  }
}

/**
 * 批量操作错误收集器
 */
export class BatchErrorCollector {
  private errors: AppError[] = []
  private context: string

  constructor(context: string) {
    this.context = context
  }

  add(error: unknown, item?: string): void {
    const appError = error instanceof AppError
      ? error
      : new AppError(
          String(error),
          'UNKNOWN_ERROR',
          ErrorLevel.ERROR,
          ErrorCategory.UNKNOWN,
          { context: `${this.context}${item ? ` - ${item}` : ''}` }
        )
    this.errors.push(appError)
  }

  hasErrors(): boolean {
    return this.errors.length > 0
  }

  getErrors(): AppError[] {
    return [...this.errors]
  }

  getErrorSummary(): { total: number; byLevel: Record<string, number>; byCategory: Record<string, number> } {
    const byLevel: Record<string, number> = {}
    const byCategory: Record<string, number> = {}

    this.errors.forEach(err => {
      byLevel[err.level] = (byLevel[err.level] || 0) + 1
      byCategory[err.category] = (byCategory[err.category] || 0) + 1
    })

    return {
      total: this.errors.length,
      byLevel,
      byCategory
    }
  }

  clear(): void {
    this.errors = []
  }

  report(): void {
    if (this.errors.length === 0) return

    const summary = this.getErrorSummary()
    logger.warn(
      `批量操作错误报告: 共 ${summary.total} 个错误`,
      this.context,
      { summary, errors: this.errors.map(e => e.toStructured()) }
    )
  }
}

/**
 * 成功提示
 * @param message - 成功消息
 */
export function showSuccess(message: string): void {
  ElMessage.success(message)
  logger.info(message, 'UI')
}

/**
 * 信息提示
 * @param message - 信息消息
 */
export function showInfo(message: string): void {
  ElMessage.info(message)
  logger.info(message, 'UI')
}

/**
 * 警告提示
 * @param message - 警告消息
 */
export function showWarning(message: string): void {
  ElMessage.warning(message)
  logger.warn(message, 'UI')
}

/**
 * 错误提示
 * @param message - 错误消息
 */
export function showError(message: string): void {
  ElMessage.error(message)
  logger.error(message, 'UI')
}
