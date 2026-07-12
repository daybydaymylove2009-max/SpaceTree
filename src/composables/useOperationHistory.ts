/**
 * 操作历史管理组合式函数
 * @module composables/useOperationHistory
 * @description 提供操作记录、撤销、重做功能
 */

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { FileInfo } from '../types'
import { showSuccess, showError } from '../utils/errorHandler'
import { useAppStore } from '../stores'

function getRecycleBinPath(dbPath: string): string {
  const lastSlash = Math.max(dbPath.lastIndexOf('/'), dbPath.lastIndexOf('\\'));
  if (lastSlash === -1) return 'recycle_bin';
  return dbPath.substring(0, lastSlash) + '/recycle_bin';
}

export interface Operation {
  id: string
  type: 'delete' | 'move' | 'rename'
  timestamp: number
  files: FileInfo[]
  details: {
    originalPaths: string[]
    newPaths?: string[]
    targetDir?: string
  }
  status: 'completed' | 'undone' | 'failed'
}

const operationHistory = ref<Operation[]>([])
const maxHistorySize = 50

/**
 * 生成唯一ID
 */
function generateId(): string {
  return Date.now().toString(36) + Math.random().toString(36).substr(2)
}

/**
 * 操作历史管理组合式函数
 */
export function useOperationHistory() {
  /**
   * 添加操作记录
   */
  function addOperation(
    type: Operation['type'],
    files: FileInfo[],
    details: Operation['details']
  ): Operation {
    const operation: Operation = {
      id: generateId(),
      type,
      timestamp: Date.now(),
      files,
      details,
      status: 'completed'
    }

    operationHistory.value.unshift(operation)

    // 限制历史记录数量
    if (operationHistory.value.length > maxHistorySize) {
      operationHistory.value = operationHistory.value.slice(0, maxHistorySize)
    }

    return operation
  }

  /**
   * 撤销删除操作
   */
  async function undoDelete(operation: Operation): Promise<boolean> {
    if (operation.type !== 'delete') {
      showError('该操作类型不支持撤销')
      return false
    }

    if (operation.status === 'undone') {
      showError('该操作已被撤销')
      return false
    }

    const appStore = useAppStore()
    const dbPath = appStore.dbPath
    if (!dbPath) {
      showError('未检测到有效的扫描数据库，无法进行历史恢复')
      return false
    }
    const recycleBinPath = getRecycleBinPath(dbPath)

    try {
      // 从回收站恢复文件
      for (const file of operation.files) {
        await invoke('restore_from_recycle_bin', {
          original_path: file.path,
          db_path: dbPath,
          recycle_bin_path: recycleBinPath
        })
      }

      operation.status = 'undone'
      showSuccess('文件已恢复')
      return true
    } catch (error) {
      showError(`恢复失败: ${error}`)
      return false
    }
  }

  /**
   * 清空历史记录
   */
  function clearHistory() {
    operationHistory.value = []
  }

  /**
   * 获取可撤销的操作
   */
  function getUndoableOperations(): Operation[] {
    return operationHistory.value.filter(op => op.status === 'completed')
  }

  return {
    operationHistory,
    addOperation,
    undoDelete,
    clearHistory,
    getUndoableOperations
  }
}
