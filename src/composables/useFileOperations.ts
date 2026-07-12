/**
 * 文件操作组合式函数 - 专业级实现
 * @module composables/useFileOperations
 * @description 企业级文件操作，支持进度报告、取消机制、批量处理、并发控制
 */

import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { logger } from '../utils/logger'
import {
  handleError,
  confirmAction,
  showSuccess,
  AppError,
  ErrorLevel,
  ErrorCategory
} from '../utils/errorHandler'
import { useAsyncQueue } from './useThrottle'
import type { FileInfo } from '../types'

/**
 * 文件操作选项
 */
export interface FileOperationsOptions {
  /** 数据库路径 */
  dbPath: string
  /** 删除成功后回调 */
  onDeleteSuccess?: () => void
  /** 并发数 */
  concurrency?: number
  /** 是否启用重试 */
  enableRetry?: boolean
  /** 最大重试次数 */
  maxRetries?: number
}

/**
 * 操作进度
 */
export interface OperationProgress {
  current: number
  total: number
  currentFile: string
  percentage: number
  status: 'idle' | 'running' | 'paused' | 'completed' | 'cancelled' | 'error'
  errors: Array<{ file: string; error: string }>
}

/**
 * 文件操作结果
 */
export interface FileOperationResult {
  success: boolean
  file: FileInfo
  error?: string
  duration: number
}

/**
 * 文件操作组合式函数 - 专业级实现
 * @param options - 配置选项
 * @returns 文件操作方法
 */
export function useFileOperations(options: FileOperationsOptions) {
  const {
    dbPath,
    onDeleteSuccess,
    concurrency = 3,
    enableRetry = true,
    maxRetries = 3
  } = options

  // 状态
  const isProcessing = ref(false)
  const isCancelled = ref(false)
  const isPaused = ref(false)
  const progress = ref<OperationProgress>({
    current: 0,
    total: 0,
    currentFile: '',
    percentage: 0,
    status: 'idle',
    errors: []
  })

  // 异步队列
  const queue = useAsyncQueue({
    concurrency,
    retry: enableRetry ? maxRetries : 0,
    retryDelay: 1000
  })

  // 计算属性
  const canCancel = computed(() => progress.value.status === 'running')
  const canPause = computed(() => progress.value.status === 'running')
  const canResume = computed(() => progress.value.status === 'paused')

  /**
   * 检查是否被取消
   */
  function checkCancelled(): boolean {
    if (isCancelled.value) {
      logger.info('操作已被用户取消', 'FileOperations')
      return true
    }
    return false
  }

  /**
   * 等待恢复
   */
  async function waitForResume(): Promise<void> {
    while (isPaused.value && !isCancelled.value) {
      await new Promise(resolve => setTimeout(resolve, 100))
    }
  }

  /**
   * 更新进度
   */
  function updateProgress(
    current: number,
    total: number,
    currentFile: string,
    status?: OperationProgress['status']
  ) {
    progress.value = {
      ...progress.value,
      current,
      total,
      currentFile,
      percentage: total > 0 ? Math.round((current / total) * 100) : 0,
      status: status || progress.value.status
    }
  }

  /**
   * 记录错误
   */
  function recordError(file: FileInfo, error: any) {
    const errorInfo = { file: file.filename, error: String(error) }
    progress.value.errors.push(errorInfo)
    logger.error('文件操作失败', 'FileOperations', { file: file.path }, error)
  }

  /**
   * 打开文件
   * @param filePath - 文件路径
   */
  async function openFile(filePath: string): Promise<boolean> {
    if (!filePath) {
      handleError(
        new AppError('文件路径无效', 'VALIDATION_ERROR', ErrorLevel.WARNING, ErrorCategory.VALIDATION),
        '打开文件'
      )
      return false
    }

    try {
      logger.info('打开文件', 'FileOperations', { path: filePath })
      await invoke('open_file', { path: filePath })
      return true
    } catch (error) {
      handleError(error, '打开文件')
      return false
    }
  }

  /**
   * 打开文件所在文件夹
   * @param filePath - 文件路径
   */
  async function openFolder(filePath: string): Promise<boolean> {
    if (!filePath) {
      handleError(
        new AppError('文件路径无效', 'VALIDATION_ERROR', ErrorLevel.WARNING, ErrorCategory.VALIDATION),
        '打开文件夹'
      )
      return false
    }

    try {
      logger.info('打开文件夹', 'FileOperations', { path: filePath })
      await invoke('show_in_folder', { path: filePath })
      return true
    } catch (error) {
      handleError(error, '打开文件夹')
      return false
    }
  }

  /**
   * 删除单个文件
   * @param fileInfo - 文件信息
   * @param skipConfirm - 跳过确认
   */
  async function deleteFile(fileInfo: FileInfo, skipConfirm = false): Promise<FileOperationResult> {
    const startTime = Date.now()

    if (!fileInfo.path) {
      const error = new AppError(
        '文件路径无效',
        'VALIDATION_ERROR',
        ErrorLevel.WARNING,
        ErrorCategory.VALIDATION
      )
      handleError(error, '删除文件')
      return { success: false, file: fileInfo, error: error.message, duration: 0 }
    }

    // 确认删除
    if (!skipConfirm) {
      const confirmed = await confirmAction(
        `确定要删除文件 "${fileInfo.filename}" 吗？`,
        '确认删除',
        'warning'
      )
      if (!confirmed) {
        return { success: false, file: fileInfo, error: '用户取消', duration: 0 }
      }
    }

    try {
      logger.info('删除文件', 'FileOperations', { path: fileInfo.path })

      await invoke('delete_file', {
        path: fileInfo.path,
        db_path: dbPath,
        allowed_roots: null
      })

      showSuccess('文件删除成功')
      onDeleteSuccess?.()

      return {
        success: true,
        file: fileInfo,
        duration: Date.now() - startTime
      }
    } catch (error) {
      handleError(error, '删除文件')
      return {
        success: false,
        file: fileInfo,
        error: String(error),
        duration: Date.now() - startTime
      }
    }
  }

  /**
   * 批量删除文件 - 专业级实现
   * @param fileInfos - 文件信息列表
   * @param options - 批量操作选项
   */
  async function batchDeleteFiles(
    fileInfos: FileInfo[],
    options: {
      skipConfirm?: boolean
      stopOnError?: boolean
      onProgress?: (progress: OperationProgress) => void
    } = {}
  ): Promise<{
    success: boolean
    completed: number
    failed: number
    results: FileOperationResult[]
  }> {
    const { skipConfirm = false, stopOnError = false, onProgress } = options

    if (fileInfos.length === 0) {
      return { success: true, completed: 0, failed: 0, results: [] }
    }

    // 批量确认
    if (!skipConfirm) {
      const confirmed = await confirmAction(
        `确定要删除选中的 ${fileInfos.length} 个文件吗？`,
        '确认批量删除',
        'warning'
      )
      if (!confirmed) {
        return { success: false, completed: 0, failed: 0, results: [] }
      }
    }

    // 初始化状态
    isProcessing.value = true
    isCancelled.value = false
    isPaused.value = false
    progress.value = {
      current: 0,
      total: fileInfos.length,
      currentFile: '',
      percentage: 0,
      status: 'running',
      errors: []
    }

    const results: FileOperationResult[] = []
    let completed = 0
    let failed = 0

    logger.info('开始批量删除', 'FileOperations', {
      count: fileInfos.length,
      stopOnError
    })

    try {
      for (let i = 0; i < fileInfos.length; i++) {
        // 检查是否被取消
        if (checkCancelled()) {
          progress.value.status = 'cancelled'
          break
        }

        // 等待恢复
        await waitForResume()

        const fileInfo = fileInfos[i]
        updateProgress(i, fileInfos.length, fileInfo.filename)
        onProgress?.(progress.value)

        // 执行删除
        const result = await deleteFile(fileInfo, true)
        results.push(result)

        if (result.success) {
          completed++
        } else {
          failed++
          recordError(fileInfo, result.error)

          if (stopOnError) {
            logger.warn('批量删除因错误停止', 'FileOperations', {
              failedAt: i + 1,
              error: result.error
            })
            break
          }
        }
      }

      // 完成
      const success = failed === 0
      progress.value.status = success ? 'completed' : failed === completed ? 'error' : 'completed'
      updateProgress(completed + failed, fileInfos.length, '', progress.value.status)
      onProgress?.(progress.value)

      if (completed > 0) {
        showSuccess(`成功删除 ${completed} 个文件`)
        onDeleteSuccess?.()
      }

      logger.info('批量删除完成', 'FileOperations', {
        completed,
        failed,
        cancelled: isCancelled.value
      })

      return { success, completed, failed, results }
    } finally {
      isProcessing.value = false
    }
  }

  /**
   * 使用队列的批量删除（支持并发控制）
   */
  async function batchDeleteWithQueue(
    fileInfos: FileInfo[],
    options: {
      skipConfirm?: boolean
      onProgress?: (progress: OperationProgress) => void
    } = {}
  ): Promise<{
    success: boolean
    completed: number
    failed: number
  }> {
    const { skipConfirm = false, onProgress } = options

    if (fileInfos.length === 0) {
      return { success: true, completed: 0, failed: 0 }
    }

    if (!skipConfirm) {
      const confirmed = await confirmAction(
        `确定要删除选中的 ${fileInfos.length} 个文件吗？`,
        '确认批量删除',
        'warning'
      )
      if (!confirmed) {
        return { success: false, completed: 0, failed: 0 }
      }
    }

    isProcessing.value = true
    isCancelled.value = false
    progress.value = {
      current: 0,
      total: fileInfos.length,
      currentFile: '',
      percentage: 0,
      status: 'running',
      errors: []
    }

    let completed = 0
    let failed = 0

    // 添加任务到队列
    const promises = fileInfos.map((fileInfo, index) =>
      queue.add(async () => {
        if (checkCancelled()) return

        updateProgress(index + 1, fileInfos.length, fileInfo.filename)
        onProgress?.(progress.value)

        try {
          await invoke('delete_file', {
            path: fileInfo.path,
            db_path: dbPath,
            allowed_roots: null
          })
          completed++
        } catch (error) {
          failed++
          recordError(fileInfo, error)
        }
      })
    )

    await Promise.all(promises)
    await queue.drain()

    progress.value.status = failed === 0 ? 'completed' : 'completed'
    isProcessing.value = false

    if (completed > 0) {
      showSuccess(`成功删除 ${completed} 个文件`)
      onDeleteSuccess?.()
    }

    return { success: failed === 0, completed, failed }
  }

  /**
   * 取消操作
   */
  function cancel() {
    if (canCancel.value) {
      isCancelled.value = true
      progress.value.status = 'cancelled'
      logger.info('用户取消操作', 'FileOperations')
    }
  }

  /**
   * 暂停操作
   */
  function pause() {
    if (canPause.value) {
      isPaused.value = true
      progress.value.status = 'paused'
      logger.info('用户暂停操作', 'FileOperations')
    }
  }

  /**
   * 恢复操作
   */
  function resume() {
    if (canResume.value) {
      isPaused.value = false
      progress.value.status = 'running'
      logger.info('用户恢复操作', 'FileOperations')
    }
  }

  /**
   * 重置状态
   */
  function reset() {
    isProcessing.value = false
    isCancelled.value = false
    isPaused.value = false
    progress.value = {
      current: 0,
      total: 0,
      currentFile: '',
      percentage: 0,
      status: 'idle',
      errors: []
    }
  }

  /**
   * 复制文件路径到剪贴板
   */
  async function copyPathToClipboard(path: string): Promise<boolean> {
    try {
      await navigator.clipboard.writeText(path)
      showSuccess('路径已复制到剪贴板')
      return true
    } catch (error) {
      handleError(error, '复制路径')
      return false
    }
  }

  /**
   * 获取文件信息
   */
  async function getFileInfo(path: string): Promise<FileInfo | null> {
    try {
      const info = await invoke('get_file_info', { path, db_path: dbPath })
      return info as FileInfo
    } catch (error) {
      handleError(error, '获取文件信息')
      return null
    }
  }

  /**
   * 移动文件
   */
  async function moveFile(
    sourcePath: string,
    destinationPath: string,
    skipConfirm = false
  ): Promise<boolean> {
    if (!skipConfirm) {
      const confirmed = await confirmAction(
        `确定要移动文件到 "${destinationPath}" 吗？`,
        '确认移动'
      )
      if (!confirmed) return false
    }

    try {
      logger.info('移动文件', 'FileOperations', { source: sourcePath, destination: destinationPath })
      await invoke('move_file', { source: sourcePath, destination: destinationPath })
      showSuccess('文件移动成功')
      return true
    } catch (error) {
      handleError(error, '移动文件')
      return false
    }
  }

  /**
   * 重命名文件
   */
  async function renameFile(
    path: string,
    newName: string,
    skipConfirm = false
  ): Promise<boolean> {
    if (!skipConfirm) {
      const confirmed = await confirmAction(
        `确定要将文件重命名为 "${newName}" 吗？`,
        '确认重命名'
      )
      if (!confirmed) return false
    }

    try {
      logger.info('重命名文件', 'FileOperations', { path, newName })
      await invoke('rename_file', { path, new_name: newName })
      showSuccess('文件重命名成功')
      return true
    } catch (error) {
      handleError(error, '重命名文件')
      return false
    }
  }

  return {
    // 状态
    isProcessing,
    isCancelled,
    isPaused,
    progress,
    canCancel,
    canPause,
    canResume,
    queueStats: queue.stats,

    // 基本操作
    openFile,
    openFolder,
    deleteFile,
    copyPathToClipboard,
    getFileInfo,
    moveFile,
    renameFile,

    // 批量操作
    batchDeleteFiles,
    batchDeleteWithQueue,

    // 控制方法
    cancel,
    pause,
    resume,
    reset
  }
}
