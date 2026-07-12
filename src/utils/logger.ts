/**
 * 专业级日志系统
 * @module utils/logger
 * @description 企业级日志管理，支持分级、持久化、上报、轮转
 */

export enum LogLevel {
  DEBUG = 0,
  INFO = 1,
  WARN = 2,
  ERROR = 3,
  FATAL = 4
}

export interface LogEntry {
  id: string
  timestamp: number
  level: LogLevel
  levelName: string
  message: string
  context?: string
  data?: any
  stack?: string
  userAgent?: string
  url?: string
}

export interface LoggerConfig {
  minLevel: LogLevel
  maxEntries: number
  enableConsole: boolean
  enablePersistence: boolean
  enableRemote: boolean
  remoteUrl?: string
  batchSize: number
  flushInterval: number
}

const DEFAULT_CONFIG: LoggerConfig = {
  minLevel: LogLevel.DEBUG,
  maxEntries: 10000,
  enableConsole: true,
  enablePersistence: true,
  enableRemote: false,
  batchSize: 100,
  flushInterval: 30000
}

const LOG_LEVEL_NAMES: Record<LogLevel, string> = {
  [LogLevel.DEBUG]: 'DEBUG',
  [LogLevel.INFO]: 'INFO',
  [LogLevel.WARN]: 'WARN',
  [LogLevel.ERROR]: 'ERROR',
  [LogLevel.FATAL]: 'FATAL'
}

const STORAGE_KEY = 'file-manager-logs-v1'
const STORAGE_INDEX_KEY = 'file-manager-logs-index'

class ProfessionalLogger {
  private config: LoggerConfig
  private logBuffer: LogEntry[] = []
  private flushTimer: number | null = null
  private isInitialized = false

  constructor(config: Partial<LoggerConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
    this.init()
  }

  private async init() {
    if (this.isInitialized) return

    // 启动定时刷新
    if (this.config.enablePersistence) {
      this.startFlushTimer()
    }

    // 页面卸载前保存
    window.addEventListener('beforeunload', () => {
      this.flush()
    })

    this.isInitialized = true
  }

  private generateId(): string {
    return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
  }

  private createEntry(
    level: LogLevel,
    message: string,
    context?: string,
    data?: any,
    error?: Error
  ): LogEntry {
    return {
      id: this.generateId(),
      timestamp: Date.now(),
      level,
      levelName: LOG_LEVEL_NAMES[level],
      message,
      context,
      data,
      stack: error?.stack,
      userAgent: navigator.userAgent,
      url: window.location.href
    }
  }

  private shouldLog(level: LogLevel): boolean {
    return level >= this.config.minLevel
  }

  private async persist(entry: LogEntry) {
    if (!this.config.enablePersistence) return

    this.logBuffer.push(entry)

    // 缓冲区满时立即刷新
    if (this.logBuffer.length >= this.config.batchSize) {
      await this.flush()
    }
  }

  private async flush() {
    if (this.logBuffer.length === 0) return

    try {
      const entries = [...this.logBuffer]
      this.logBuffer = []

      // 获取现有日志
      const existingLogs = await this.getStoredLogs()
      const allLogs = [...entries, ...existingLogs]

      // 限制日志数量，保留最新的
      const trimmedLogs = allLogs.slice(0, this.config.maxEntries)

      // 分批存储避免超出存储限制
      const chunks = this.chunkArray(trimmedLogs, 100)
      for (let i = 0; i < chunks.length; i++) {
        localStorage.setItem(`${STORAGE_KEY}-${i}`, JSON.stringify(chunks[i]))
      }

      // 更新索引
      localStorage.setItem(STORAGE_INDEX_KEY, JSON.stringify({
        chunkCount: chunks.length,
        totalEntries: trimmedLogs.length,
        lastUpdate: Date.now()
      }))

      // 远程上报
      if (this.config.enableRemote && this.config.remoteUrl) {
        await this.reportToRemote(entries)
      }
    } catch (error) {
      console.error('日志持久化失败:', error)
    }
  }

  private chunkArray<T>(array: T[], size: number): T[][] {
    const chunks: T[][] = []
    for (let i = 0; i < array.length; i += size) {
      chunks.push(array.slice(i, i + size))
    }
    return chunks
  }

  private async getStoredLogs(): Promise<LogEntry[]> {
    try {
      const index = localStorage.getItem(STORAGE_INDEX_KEY)
      if (!index) return []

      const { chunkCount } = JSON.parse(index)
      const logs: LogEntry[] = []

      for (let i = 0; i < chunkCount; i++) {
        const chunk = localStorage.getItem(`${STORAGE_KEY}-${i}`)
        if (chunk) {
          logs.push(...JSON.parse(chunk))
        }
      }

      return logs
    } catch {
      return []
    }
  }

  private async reportToRemote(entries: LogEntry[]) {
    if (!this.config.remoteUrl) return

    try {
      await fetch(this.config.remoteUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ logs: entries })
      })
    } catch (error) {
      console.error('日志上报失败:', error)
    }
  }

  private startFlushTimer() {
    if (this.flushTimer) {
      clearInterval(this.flushTimer)
    }
    this.flushTimer = window.setInterval(() => {
      this.flush()
    }, this.config.flushInterval)
  }

  private outputToConsole(entry: LogEntry) {
    if (!this.config.enableConsole) return

    const timestamp = new Date(entry.timestamp).toISOString()
    const prefix = `[${timestamp}] [${entry.levelName}]${entry.context ? ` [${entry.context}]` : ''}`

    switch (entry.level) {
      case LogLevel.DEBUG:
        console.debug(prefix, entry.message, entry.data || '')
        break
      case LogLevel.INFO:
        console.info(prefix, entry.message, entry.data || '')
        break
      case LogLevel.WARN:
        console.warn(prefix, entry.message, entry.data || '')
        break
      case LogLevel.ERROR:
      case LogLevel.FATAL:
        console.error(prefix, entry.message, entry.data || '', entry.stack || '')
        break
    }
  }

  // 公共API
  log(level: LogLevel, message: string, context?: string, data?: any, error?: Error) {
    if (!this.shouldLog(level)) return

    const entry = this.createEntry(level, message, context, data, error)
    this.outputToConsole(entry)
    this.persist(entry)
  }

  debug(message: string, context?: string, data?: any) {
    this.log(LogLevel.DEBUG, message, context, data)
  }

  info(message: string, context?: string, data?: any) {
    this.log(LogLevel.INFO, message, context, data)
  }

  warn(message: string, context?: string, data?: any, error?: Error) {
    this.log(LogLevel.WARN, message, context, data, error)
  }

  error(message: string, context?: string, data?: any, error?: Error) {
    this.log(LogLevel.ERROR, message, context, data, error)
  }

  fatal(message: string, context?: string, data?: any, error?: Error) {
    this.log(LogLevel.FATAL, message, context, data, error)
  }

  // 获取日志
  async getLogs(
    options: {
      level?: LogLevel
      startTime?: number
      endTime?: number
      context?: string
      limit?: number
    } = {}
  ): Promise<LogEntry[]> {
    const { level, startTime, endTime, context, limit = 100 } = options

    let logs = await this.getStoredLogs()

    // 应用过滤器
    if (level !== undefined) {
      logs = logs.filter(l => l.level >= level)
    }
    if (startTime) {
      logs = logs.filter(l => l.timestamp >= startTime)
    }
    if (endTime) {
      logs = logs.filter(l => l.timestamp <= endTime)
    }
    if (context) {
      logs = logs.filter(l => l.context === context)
    }

    // 按时间倒序
    logs.sort((a, b) => b.timestamp - a.timestamp)

    return logs.slice(0, limit)
  }

  // 导出日志
  async exportLogs(format: 'json' | 'csv' = 'json'): Promise<string> {
    const logs = await this.getLogs({ limit: this.config.maxEntries })

    if (format === 'csv') {
      const headers = ['timestamp', 'level', 'context', 'message', 'url']
      const rows = logs.map(l => [
        new Date(l.timestamp).toISOString(),
        l.levelName,
        l.context || '',
        l.message,
        l.url || ''
      ])
      return [headers.join(','), ...rows.map(r => r.join(','))].join('\n')
    }

    return JSON.stringify(logs, null, 2)
  }

  // 清空日志
  async clearLogs() {
    this.logBuffer = []

    const index = localStorage.getItem(STORAGE_INDEX_KEY)
    if (index) {
      const { chunkCount } = JSON.parse(index)
      for (let i = 0; i < chunkCount; i++) {
        localStorage.removeItem(`${STORAGE_KEY}-${i}`)
      }
      localStorage.removeItem(STORAGE_INDEX_KEY)
    }

    this.info('日志已清空', 'Logger')
  }

  // 获取统计
  async getStats() {
    const logs = await this.getStoredLogs()
    const stats = {
      total: logs.length,
      byLevel: {} as Record<string, number>,
      byContext: {} as Record<string, number>,
      timeRange: {
        oldest: logs.length > 0 ? Math.min(...logs.map(l => l.timestamp)) : null,
        newest: logs.length > 0 ? Math.max(...logs.map(l => l.timestamp)) : null
      }
    }

    logs.forEach(l => {
      stats.byLevel[l.levelName] = (stats.byLevel[l.levelName] || 0) + 1
      if (l.context) {
        stats.byContext[l.context] = (stats.byContext[l.context] || 0) + 1
      }
    })

    return stats
  }

  // 销毁
  destroy() {
    if (this.flushTimer) {
      clearInterval(this.flushTimer)
      this.flushTimer = null
    }
    this.flush()
  }
}

// 单例实例
let loggerInstance: ProfessionalLogger | null = null

export function getLogger(config?: Partial<LoggerConfig>): ProfessionalLogger {
  if (!loggerInstance) {
    loggerInstance = new ProfessionalLogger(config)
  }
  return loggerInstance
}

export function resetLogger() {
  if (loggerInstance) {
    loggerInstance.destroy()
    loggerInstance = null
  }
}

// 便捷导出
export const logger = getLogger()
