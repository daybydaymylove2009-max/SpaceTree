/**
 * 原子性操作和回滚机制 - 专业级实现
 * @module composables/useAtomicOperations
 * @description 企业级事务性文件操作，支持原子性执行、完整回滚、持久化存储
 */

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getIndexedDB } from '../utils/indexedDB'
import { logger } from '../utils/logger'

import type { FileInfo } from '../types'

// 操作类型
export type OperationType = 'delete' | 'move' | 'rename'

// 操作状态
export type OperationStatus = 'pending' | 'executing' | 'completed' | 'failed' | 'rolled_back'

// 单个操作记录
export interface AtomicOperation {
  id: string
  type: OperationType
  status: OperationStatus
  file: FileInfo
  backupPath?: string
  originalPath: string
  targetPath?: string
  error?: string
  executedAt?: number
  rolledBackAt?: number
  retryCount: number
}

// 事务
export interface Transaction {
  id: string
  name: string
  operations: AtomicOperation[]
  status: 'pending' | 'executing' | 'committed' | 'rolled_back' | 'partial'
  createdAt: number
  committedAt?: number
  rolledBackAt?: number
  totalSpaceToFree: number
  actuallyFreed: number
  metadata?: {
    source: string
    userInitiated: boolean
    batchSize: number
  }
}

// 备份信息
interface BackupInfo {
  operationId: string
  originalPath: string
  backupPath: string
  size: number
}

// 事务统计
interface TransactionStats {
  total: number
  committed: number
  rolledBack: number
  partial: number
  totalFreed: number
  successRate: number
  averageDuration: number
}

const DB_STORE = 'transactions'
const MAX_RETRY_ATTEMPTS = 3
const RETRY_DELAY_MS = 1000

/**
 * 原子性操作组合式函数 - 专业级实现
 */
export function useAtomicOperations() {
  const transactions = ref<Transaction[]>([])
  const currentTransaction = ref<Transaction | null>(null)
  const isProcessing = ref(false)
  const operationProgress = ref(0)
  const db = getIndexedDB()

  /**
   * 生成唯一ID
   */
  function generateId(): string {
    return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}-${crypto.randomUUID().slice(0, 8)}`
  }

  /**
   * 创建新事务
   */
  function createTransaction(
    name: string,
    files: FileInfo[],
    metadata?: Transaction['metadata']
  ): Transaction {
    const operations: AtomicOperation[] = files.map(file => ({
      id: generateId(),
      type: 'delete',
      status: 'pending',
      file,
      originalPath: file.path,
      targetPath: undefined,
      retryCount: 0
    }))

    const transaction: Transaction = {
      id: generateId(),
      name,
      operations,
      status: 'pending',
      createdAt: Date.now(),
      totalSpaceToFree: files.reduce((sum, f) => sum + f.size, 0),
      actuallyFreed: 0,
      metadata
    }

    currentTransaction.value = transaction

    logger.info('创建事务', 'AtomicOperations', {
      transactionId: transaction.id,
      operationCount: operations.length,
      totalSpace: transaction.totalSpaceToFree
    })

    return transaction
  }

  /**
   * 创建备份
   */
  async function createBackup(operation: AtomicOperation): Promise<BackupInfo | null> {
    try {
      // 创建临时备份目录
      const backupDir = await invoke('get_temp_dir') as string
      const timestamp = Date.now()
      const backupPath = `${backupDir}/file_manager_backup_${operation.id}_${timestamp}_${operation.file.filename}`

      logger.debug('创建备份', 'AtomicOperations', {
        operationId: operation.id,
        source: operation.originalPath,
        backup: backupPath
      })

      // 复制文件到备份位置
      await invoke('copy_file', {
        source: operation.originalPath,
        destination: backupPath
      })

      logger.info('备份创建成功', 'AtomicOperations', {
        operationId: operation.id,
        backupPath
      })

      return {
        operationId: operation.id,
        originalPath: operation.originalPath,
        backupPath,
        size: operation.file.size
      }
    } catch (error) {
      logger.error('创建备份失败', 'AtomicOperations', { operationId: operation.id }, error as Error)
      return null
    }
  }

  /**
   * 恢复备份
   */
  async function restoreBackup(backupPath: string, originalPath: string): Promise<boolean> {
    try {
      logger.debug('恢复备份', 'AtomicOperations', { backupPath, originalPath })

      await invoke('copy_file', {
        source: backupPath,
        destination: originalPath
      })

      logger.info('备份恢复成功', 'AtomicOperations', { originalPath })
      return true
    } catch (error) {
      logger.error('恢复备份失败', 'AtomicOperations', { backupPath, originalPath }, error as Error)
      return false
    }
  }

  /**
   * 删除备份
   */
  async function deleteBackup(backupPath: string): Promise<boolean> {
    try {
      await invoke('delete_file_permanent', { path: backupPath })
      logger.debug('备份删除成功', 'AtomicOperations', { backupPath })
      return true
    } catch (error) {
      logger.warn('删除备份失败', 'AtomicOperations', { backupPath }, error as Error)
      return false
    }
  }

  /**
   * 执行单个操作（带重试机制）
   */
  async function executeOperation(
    operation: AtomicOperation,
    createBackups: boolean = true
  ): Promise<{ success: boolean; error?: string; retryable: boolean }> {
    operation.status = 'executing'
    operation.executedAt = Date.now()

    try {
      // 1. 创建备份（如果需要）
      if (createBackups) {
        const backup = await createBackup(operation)
        if (backup) {
          operation.backupPath = backup.backupPath
        }
      }

      // 2. 执行操作
      switch (operation.type) {
        case 'delete':
          await invoke('delete_file', {
            path: operation.originalPath,
            move_to_recycle_bin: true
          })
          break
        case 'move':
          if (operation.targetPath) {
            await invoke('move_file', {
              source: operation.originalPath,
              destination: operation.targetPath
            })
          }
          break
        case 'rename':
          if (operation.targetPath) {
            await invoke('rename_file', {
              path: operation.originalPath,
              new_name: operation.targetPath
            })
          }
          break
      }

      operation.status = 'completed'
      logger.debug('操作执行成功', 'AtomicOperations', { operationId: operation.id, type: operation.type })
      return { success: true, retryable: false }
    } catch (error) {
      operation.status = 'failed'
      operation.error = String(error)

      // 判断是否可以重试
      const retryable = operation.retryCount < MAX_RETRY_ATTEMPTS && isRetryableError(error)

      logger.error(
        '操作执行失败',
        'AtomicOperations',
        { operationId: operation.id, retryCount: operation.retryCount, retryable },
        error as Error
      )

      return { success: false, error: String(error), retryable }
    }
  }

  /**
   * 判断错误是否可重试
   */
  function isRetryableError(error: any): boolean {
    const message = String(error).toLowerCase()
    return (
      message.includes('busy') ||
      message.includes('locked') ||
      message.includes('temporarily') ||
      message.includes('timeout') ||
      message.includes('network')
    )
  }

  /**
   * 重试操作
   */
  async function retryOperation(
    operation: AtomicOperation,
    createBackups: boolean
  ): Promise<{ success: boolean; error?: string; retryable: boolean }> {
    operation.retryCount++
    logger.info(`重试操作 (${operation.retryCount}/${MAX_RETRY_ATTEMPTS})`, 'AtomicOperations', {
      operationId: operation.id
    })

    // 延迟重试
    await new Promise(resolve => setTimeout(resolve, RETRY_DELAY_MS * operation.retryCount))

    const result = await executeOperation(operation, createBackups)

    if (!result.success && result.retryable && operation.retryCount < MAX_RETRY_ATTEMPTS) {
      return retryOperation(operation, createBackups)
    }

    return { success: result.success, error: result.error, retryable: result.retryable }
  }

  /**
   * 回滚单个操作
   */
  async function rollbackOperation(operation: AtomicOperation): Promise<boolean> {
    if (operation.status !== 'completed') {
      return true // 未执行的操作无需回滚
    }

    logger.info('回滚操作', 'AtomicOperations', {
      operationId: operation.id,
      type: operation.type
    })

    try {
      switch (operation.type) {
        case 'delete':
          // 从备份恢复
          if (operation.backupPath) {
            const restored = await restoreBackup(
              operation.backupPath,
              operation.originalPath
            )
            if (restored) {
              await deleteBackup(operation.backupPath)
            }
            return restored
          }
          return false

        case 'move':
          // 移回原始位置
          if (operation.targetPath) {
            await invoke('move_file', {
              source: operation.targetPath,
              destination: operation.originalPath
            })
          }
          return true

        case 'rename':
          // 恢复原名
          if (operation.targetPath) {
            await invoke('rename_file', {
              path: operation.targetPath,
              new_name: operation.originalPath
            })
          }
          return true

        default:
          return false
      }
    } catch (error) {
      logger.error('回滚操作失败', 'AtomicOperations', { operationId: operation.id }, error as Error)
      return false
    } finally {
      operation.status = 'rolled_back'
      operation.rolledBackAt = Date.now()
    }
  }

  /**
   * 执行事务（原子性）
   */
  async function executeTransaction(
    transaction: Transaction,
    options: {
      createBackups?: boolean
      stopOnError?: boolean
      onProgress?: (completed: number, total: number, currentFile: string) => void
      onOperationComplete?: (operation: AtomicOperation, success: boolean) => void
    } = {}
  ): Promise<{
    success: boolean
    completed: number
    failed: number
    rolledBack: number
    errors: string[]
    transaction: Transaction
  }> {
    const {
      createBackups = true,
      stopOnError = false,
      onProgress,
      onOperationComplete
    } = options

    isProcessing.value = true
    transaction.status = 'executing'
    operationProgress.value = 0

    const errors: string[] = []
    let completed = 0
    let failed = 0

    logger.info('开始执行事务', 'AtomicOperations', {
      transactionId: transaction.id,
      operationCount: transaction.operations.length,
      stopOnError
    })

    // 第一阶段：执行所有操作
    for (let i = 0; i < transaction.operations.length; i++) {
      const operation = transaction.operations[i]

      let result = await executeOperation(operation, createBackups)

      // 如果失败且可重试，进行重试
      if (!result.success && result.retryable) {
        result = await retryOperation(operation, createBackups)
      }

      if (result.success) {
        completed++
        transaction.actuallyFreed += operation.file.size
      } else {
        failed++
        errors.push(`${operation.file.filename}: ${result.error}`)

        if (stopOnError) {
          logger.warn('事务因错误停止', 'AtomicOperations', {
            transactionId: transaction.id,
            failedAt: i + 1
          })
          break
        }
      }

      onOperationComplete?.(operation, result.success)
      onProgress?.(completed + failed, transaction.operations.length, operation.file.filename)
      operationProgress.value = Math.round(((completed + failed) / transaction.operations.length) * 100)
    }

    // 第二阶段：如果设置了stopOnError且有失败，回滚所有操作
    if (stopOnError && failed > 0) {
      logger.info('开始回滚事务', 'AtomicOperations', { transactionId: transaction.id })
      const rollbackResult = await rollbackTransaction(transaction)

      return {
        success: false,
        completed: 0,
        failed: transaction.operations.length,
        rolledBack: rollbackResult.rolledBack,
        errors: [...errors, `事务已回滚 (${rollbackResult.rolledBack}/${rollbackResult.rolledBack + rollbackResult.failed})`],
        transaction
      }
    }

    // 确定事务状态
    if (failed === 0) {
      transaction.status = 'committed'
      transaction.committedAt = Date.now()
      logger.info('事务提交成功', 'AtomicOperations', {
        transactionId: transaction.id,
        duration: transaction.committedAt - transaction.createdAt,
        freed: transaction.actuallyFreed
      })
    } else if (completed === 0) {
      transaction.status = 'rolled_back'
      transaction.rolledBackAt = Date.now()
    } else {
      transaction.status = 'partial'
      logger.warn('事务部分完成', 'AtomicOperations', {
        transactionId: transaction.id,
        completed,
        failed
      })
    }

    // 保存事务记录到 IndexedDB
    await saveTransaction(transaction)

    isProcessing.value = false
    currentTransaction.value = null
    operationProgress.value = 0

    return {
      success: failed === 0,
      completed,
      failed,
      rolledBack: 0,
      errors,
      transaction
    }
  }

  /**
   * 回滚整个事务
   */
  async function rollbackTransaction(transaction: Transaction): Promise<{
    success: boolean
    rolledBack: number
    failed: number
  }> {
    logger.info('回滚整个事务', 'AtomicOperations', {
      transactionId: transaction.id,
      operationCount: transaction.operations.length
    })

    let rolledBack = 0
    let failed = 0

    for (const operation of transaction.operations) {
      if (operation.status === 'completed') {
        const success = await rollbackOperation(operation)
        if (success) {
          rolledBack++
        } else {
          failed++
        }
      }
    }

    transaction.status = 'rolled_back'
    transaction.rolledBackAt = Date.now()
    await saveTransaction(transaction)

    logger.info('事务回滚完成', 'AtomicOperations', {
      transactionId: transaction.id,
      rolledBack,
      failed
    })

    return {
      success: failed === 0,
      rolledBack,
      failed
    }
  }

  /**
   * 保存事务记录到 IndexedDB
   */
  async function saveTransaction(transaction: Transaction): Promise<void> {
    try {
      await db.put(DB_STORE, transaction)

      // 更新内存中的列表
      const index = transactions.value.findIndex(t => t.id === transaction.id)
      if (index >= 0) {
        transactions.value[index] = transaction
      } else {
        transactions.value.unshift(transaction)
      }

      // 限制内存中的事务数量
      if (transactions.value.length > 50) {
        transactions.value = transactions.value.slice(0, 50)
      }
    } catch (error) {
      logger.error('保存事务记录失败', 'AtomicOperations', { transactionId: transaction.id }, error as Error)
    }
  }

  /**
   * 加载事务记录
   */
  async function loadTransactions(limit: number = 50): Promise<void> {
    try {
      const allTransactions = await db.getAll<Transaction>(DB_STORE)
      transactions.value = allTransactions
        .sort((a, b) => b.createdAt - a.createdAt)
        .slice(0, limit)

      logger.info('加载事务记录', 'AtomicOperations', { count: transactions.value.length })
    } catch (error) {
      logger.error('加载事务记录失败', 'AtomicOperations', {}, error as Error)
    }
  }

  /**
   * 获取事务统计
   */
  async function getTransactionStats(): Promise<TransactionStats> {
    const allTransactions = await db.getAll<Transaction>(DB_STORE)

    const total = allTransactions.length
    const committed = allTransactions.filter(t => t.status === 'committed').length
    const rolledBack = allTransactions.filter(t => t.status === 'rolled_back').length
    const partial = allTransactions.filter(t => t.status === 'partial').length

    const totalFreed = allTransactions.reduce((sum, t) => sum + t.actuallyFreed, 0)

    const completedTransactions = allTransactions.filter(t => t.committedAt || t.rolledBackAt)
    const averageDuration = completedTransactions.length > 0
      ? completedTransactions.reduce((sum, t) => {
          const end = t.committedAt || t.rolledBackAt || t.createdAt
          return sum + (end - t.createdAt)
        }, 0) / completedTransactions.length
      : 0

    return {
      total,
      committed,
      rolledBack,
      partial,
      totalFreed,
      successRate: total > 0 ? (committed / total) * 100 : 0,
      averageDuration
    }
  }

  /**
   * 清理旧事务记录
   */
  async function cleanupOldTransactions(keepCount: number = 20): Promise<number> {
    const allTransactions = await db.getAll<Transaction>(DB_STORE)
    const sorted = allTransactions.sort((a, b) => b.createdAt - a.createdAt)

    if (sorted.length <= keepCount) return 0

    const toDelete = sorted.slice(keepCount)
    let deleted = 0

    for (const transaction of toDelete) {
      try {
        await db.delete(DB_STORE, transaction.id)
        deleted++
      } catch (error) {
        logger.warn('删除旧事务记录失败', 'AtomicOperations', { transactionId: transaction.id }, error as Error)
      }
    }

    // 清理相关备份文件
    for (const transaction of toDelete) {
      for (const operation of transaction.operations) {
        if (operation.backupPath) {
          await deleteBackup(operation.backupPath)
        }
      }
    }

    logger.info('清理旧事务记录', 'AtomicOperations', { deleted, kept: keepCount })
    return deleted
  }

  /**
   * 导出事务记录
   */
  async function exportTransactions(format: 'json' | 'csv' = 'json'): Promise<string> {
    const allTransactions = await db.getAll<Transaction>(DB_STORE)

    if (format === 'csv') {
      const headers = ['id', 'name', 'status', 'createdAt', 'completedAt', 'operations', 'freed']
      const rows = allTransactions.map(t => [
        t.id,
        t.name,
        t.status,
        new Date(t.createdAt).toISOString(),
        t.committedAt ? new Date(t.committedAt).toISOString() : '',
        t.operations.length,
        t.actuallyFreed
      ])
      return [headers.join(','), ...rows.map(r => r.join(','))].join('\n')
    }

    return JSON.stringify(allTransactions, null, 2)
  }

  // 初始化加载
  loadTransactions()

  return {
    transactions,
    currentTransaction,
    isProcessing,
    operationProgress,
    createTransaction,
    executeTransaction,
    rollbackTransaction,
    rollbackOperation,
    getTransactionStats,
    cleanupOldTransactions,
    exportTransactions,
    loadTransactions
  }
}
