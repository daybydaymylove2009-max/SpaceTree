/**
 * 批量操作组合式函数
 * @module composables/useBatchOperations
 * @description 提供文件批量选择、删除、移动等功能
 */

import { ref, computed } from 'vue'
import type { FileInfo } from '../types'
import { useFileOperations } from './useFileOperations'
import { confirmAction, showSuccess, showError } from '../utils/errorHandler'

/**
 * 批量操作选项
 */
export interface BatchOperationsOptions {
  dbPath: string
  onOperationComplete?: () => void
}

/**
 * 批量操作组合式函数
 */
export function useBatchOperations(options: BatchOperationsOptions) {
  const { dbPath, onOperationComplete } = options
  const { deleteFile } = useFileOperations({ dbPath })

  // 选中的文件
  const selectedFiles = ref<FileInfo[]>([])
  
  // 是否处于批量选择模式
  const isBatchMode = ref(false)

  // 是否有选中文件
  const hasSelection = computed(() => selectedFiles.value.length > 0)

  // 选中文件数量
  const selectionCount = computed(() => selectedFiles.value.length)

  /**
   * 进入批量选择模式
   */
  function enterBatchMode() {
    isBatchMode.value = true
    selectedFiles.value = []
  }

  /**
   * 退出批量选择模式
   */
  function exitBatchMode() {
    isBatchMode.value = false
    selectedFiles.value = []
  }

  /**
   * 切换文件选中状态
   */
  function toggleSelection(file: FileInfo) {
    const index = selectedFiles.value.findIndex(f => f.path === file.path)
    if (index > -1) {
      selectedFiles.value.splice(index, 1)
    } else {
      selectedFiles.value.push(file)
    }
  }

  /**
   * 检查文件是否被选中
   */
  function isSelected(file: FileInfo): boolean {
    return selectedFiles.value.some(f => f.path === file.path)
  }

  /**
   * 全选文件
   */
  function selectAll(files: FileInfo[]) {
    selectedFiles.value = [...files]
  }

  /**
   * 取消全选
   */
  function deselectAll() {
    selectedFiles.value = []
  }

  /**
   * 批量删除文件
   */
  async function batchDelete(): Promise<number> {
    if (selectedFiles.value.length === 0) {
      showError('请先选择要删除的文件')
      return 0
    }

    const confirmed = await confirmAction(
      `确定要删除选中的 ${selectedFiles.value.length} 个文件吗？`,
      '确认批量删除',
      'warning'
    )

    if (!confirmed) return 0

    let deletedCount = 0
    const failedFiles: string[] = []

    for (const file of selectedFiles.value) {
      try {
        const success = await deleteFile(file)
        if (success) {
          deletedCount++
        } else {
          failedFiles.push(file.filename)
        }
      } catch (error) {
        failedFiles.push(file.filename)
      }
    }

    if (deletedCount > 0) {
      showSuccess(`成功删除 ${deletedCount} 个文件`)
      selectedFiles.value = []
      onOperationComplete?.()
    }

    if (failedFiles.length > 0) {
      showError(`${failedFiles.length} 个文件删除失败`)
    }

    return deletedCount
  }

  /**
   * 智能选择（保留策略）
   * @param files - 文件列表
   * @param strategy - 保留策略
   */
  function smartSelect(files: FileInfo[], strategy: 'newest' | 'oldest' | 'shortest-path' | 'longest-path') {
    if (files.length <= 1) return

    let filesToDelete: FileInfo[] = []

    switch (strategy) {
      case 'newest':
        // 保留最新的，删除其他的
        const sortedByDate = [...files].sort((a, b) => 
          new Date(b.modified_at).getTime() - new Date(a.modified_at).getTime()
        )
        filesToDelete = sortedByDate.slice(1)
        break

      case 'oldest':
        // 保留最旧的，删除其他的
        const sortedByDateAsc = [...files].sort((a, b) => 
          new Date(a.modified_at).getTime() - new Date(b.modified_at).getTime()
        )
        filesToDelete = sortedByDateAsc.slice(1)
        break

      case 'shortest-path':
        // 保留路径最短的，删除其他的
        const sortedByPathLength = [...files].sort((a, b) => a.path.length - b.path.length)
        filesToDelete = sortedByPathLength.slice(1)
        break

      case 'longest-path':
        // 保留路径最长的，删除其他的
        const sortedByPathLengthDesc = [...files].sort((a, b) => b.path.length - a.path.length)
        filesToDelete = sortedByPathLengthDesc.slice(1)
        break
    }

    selectedFiles.value = filesToDelete
  }

  return {
    selectedFiles,
    isBatchMode,
    hasSelection,
    selectionCount,
    enterBatchMode,
    exitBatchMode,
    toggleSelection,
    isSelected,
    selectAll,
    deselectAll,
    batchDelete,
    smartSelect
  }
}
