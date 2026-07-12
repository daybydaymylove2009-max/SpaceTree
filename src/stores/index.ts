/**
 * 文件管理系统 - Pinia Store
 * @module stores/index
 * @description 全局状态管理，使用 Pinia 实现
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  FileInfo,
  DuplicateGroup,
  DuplicateClassification,
  ScanConfig,
  ScanProgress,
  MenuKey,
  DuplicateType,
  AppSettings
} from '../types'
import { handleError } from '../utils/errorHandler'
import { invoke } from '@tauri-apps/api/core'

/**
 * 应用主 Store
 * @store useAppStore
 */
export const useAppStore = defineStore('app', () => {
  // ========== State ==========

  /** 当前激活的菜单 */
  const activeMenu = ref<MenuKey>('scan')

  /** 数据库路径 */
  const dbPath = ref('')

  /** 重复文件分类结果 */
  const duplicateClassification = ref<DuplicateClassification | null>(null)

  /** 数据库中是否有文件数据 */
  const hasDatabaseFiles = ref<boolean | null>(null)

  /** 是否正在加载 */
  const isLoading = ref(false)

  /** 当前操作进度 */
  const currentProgress = ref(0)

  /** 应用设置 */
  const settings = ref<AppSettings>({
    theme: 'auto',
    language: 'zh-CN',
    auto_scan: false,
    default_scan_config: {
      max_depth: 10,
      include_extensions: [],
      exclude_extensions: ['tmp', 'log', 'cache'],
      min_size: 0,
      max_size: 0,
      exclude_hidden: true,
      exclude_system: true,
      exclude_patterns: ['node_modules', '.git', 'target', 'dist'],
      hash_algorithm: 'xxhash3'
    },
    notification_enabled: true,
    delete_confirmation: true
  })

  // ========== Getters ==========

  /** 是否有重复文件数据 */
  const hasDuplicates = computed(() => {
    if (!duplicateClassification.value) return false
    return (
      duplicateClassification.value.complete_duplicates.length > 0 ||
      duplicateClassification.value.name_duplicates.length > 0 ||
      duplicateClassification.value.content_duplicates.length > 0
    )
  })

  /** 重复文件总数 */
  const totalDuplicateGroups = computed(() => {
    if (!duplicateClassification.value) return 0
    return (
      duplicateClassification.value.complete_duplicates.length +
      duplicateClassification.value.name_duplicates.length +
      duplicateClassification.value.content_duplicates.length
    )
  })

  /** 获取指定类型的重复文件 */
  const getDuplicatesByType = (type: DuplicateType): DuplicateGroup[] => {
    if (!duplicateClassification.value) return []
    switch (type) {
      case 'complete':
        return duplicateClassification.value.complete_duplicates
      case 'name':
        return duplicateClassification.value.name_duplicates
      case 'content':
        return duplicateClassification.value.content_duplicates
      default:
        return []
    }
  }

  // ========== Actions ==========

  /**
   * 设置当前菜单
   * @param menu - 菜单键值
   */
  function setActiveMenu(menu: MenuKey): void {
    activeMenu.value = menu
  }

  /**
   * 设置数据库路径
   * @param path - 数据库路径
   */
  function setDbPath(path: string): void {
    dbPath.value = path
  }

  /**
   * 设置重复文件分类结果
   * @param classification - 分类结果
   */
  function setDuplicateClassification(classification: DuplicateClassification | null): void {
    duplicateClassification.value = classification
  }

  /**
   * 检查数据库是否有文件
   */
  async function checkDatabaseFiles(): Promise<boolean> {
    if (!dbPath.value) return false

    isLoading.value = true
    try {
      const hasFiles = await invoke('check_database_has_files', { db_path: dbPath.value }) as boolean
      hasDatabaseFiles.value = hasFiles
      return hasFiles
    } catch (error) {
      handleError(error, '检查数据库失败')
      hasDatabaseFiles.value = false
      return false
    } finally {
      isLoading.value = false
    }
  }

  /**
   * 查询重复文件
   */
  async function queryDuplicates(): Promise<void> {
    if (!dbPath.value) {
      handleError(new Error('数据库路径未设置'))
      return
    }

    isLoading.value = true
    try {
      const result = await invoke('find_duplicates', { db_path: dbPath.value }) as DuplicateClassification
      duplicateClassification.value = result
    } catch (error) {
      handleError(error, '查询重复文件失败')
      duplicateClassification.value = {
        complete_duplicates: [],
        name_duplicates: [],
        content_duplicates: []
      }
    } finally {
      isLoading.value = false
    }
  }

  /**
   * 删除文件
   * @param fileInfo - 文件信息
   */
  async function deleteFile(fileInfo: FileInfo): Promise<boolean> {
    if (!dbPath.value || !fileInfo.path) return false

    try {
      await invoke('delete_file', {
        path: fileInfo.path,
        db_path: dbPath.value,
        allowed_roots: null
      })

      // 从当前列表中移除已删除的文件
      if (duplicateClassification.value) {
        const removeFromGroup = (groups: DuplicateGroup[]) => {
          groups.forEach(group => {
            const index = group.files.findIndex(f => f.path === fileInfo.path)
            if (index > -1) {
              group.files.splice(index, 1)
            }
          })
        }

        removeFromGroup(duplicateClassification.value.complete_duplicates)
        removeFromGroup(duplicateClassification.value.name_duplicates)
        removeFromGroup(duplicateClassification.value.content_duplicates)
      }

      return true
    } catch (error) {
      handleError(error, '删除文件失败')
      return false
    }
  }

  /**
   * 打开文件
   * @param filePath - 文件路径
   */
  async function openFile(filePath: string): Promise<void> {
    try {
      await invoke('open_file', { path: filePath })
    } catch (error) {
      handleError(error, '打开文件失败')
    }
  }

  /**
   * 打开文件所在文件夹
   * @param filePath - 文件路径
   */
  async function openFolder(filePath: string): Promise<void> {
    try {
      await invoke('show_in_folder', { path: filePath })
    } catch (error) {
      handleError(error, '打开文件夹失败')
    }
  }

  /**
   * 更新设置
   * @param newSettings - 新设置
   */
  function updateSettings(newSettings: Partial<AppSettings>): void {
    settings.value = { ...settings.value, ...newSettings }
  }

  /**
   * 重置状态
   */
  function resetState(): void {
    duplicateClassification.value = null
    hasDatabaseFiles.value = null
    currentProgress.value = 0
  }

  return {
    // State
    activeMenu,
    dbPath,
    duplicateClassification,
    hasDatabaseFiles,
    isLoading,
    currentProgress,
    settings,

    // Getters
    hasDuplicates,
    totalDuplicateGroups,
    getDuplicatesByType,

    // Actions
    setActiveMenu,
    setDbPath,
    setDuplicateClassification,
    checkDatabaseFiles,
    queryDuplicates,
    deleteFile,
    openFile,
    openFolder,
    updateSettings,
    resetState
  }
})

/**
 * 扫描状态 Store
 * @store useScanStore
 */
export const useScanStore = defineStore('scan', () => {
  // ========== State ==========

  /** 是否正在扫描 */
  const isScanning = ref(false)

  /** 是否暂停 */
  const isPaused = ref(false)

  /** 扫描进度 */
  const progress = ref<ScanProgress>({
    total_files: 0,
    processed_files: 0,
    current_file: '',
    percentage: 0,
    is_paused: false,
    is_stopped: false,
    scanned_directories: 0,
    current_directory: '',
    recent_files: []
  })

  /** 扫描配置 */
  const scanConfig = ref<ScanConfig>({
    max_depth: 10,
    include_extensions: [],
    exclude_extensions: ['tmp', 'log', 'cache'],
    min_size: 0,
    max_size: 0,
    exclude_hidden: true,
    exclude_system: true,
    exclude_patterns: ['node_modules', '.git', 'target', 'dist'],
    hash_algorithm: 'xxhash3'
  })

  /** 扫描目录列表 */
  const directories = ref<string[]>([])

  // ========== Getters ==========

  /** 扫描百分比 */
  const scanPercentage = computed(() => {
    if (progress.value.total_files === 0) return 0
    return Math.round((progress.value.processed_files / progress.value.total_files) * 100)
  })

  /** 预计剩余时间 */
  const estimatedTimeRemaining = computed(() => {
    if (progress.value.percentage <= 0) return '计算中...'
    // 简化计算，实际需要根据历史速度估算
    return '计算中...'
  })

  // ========== Actions ==========

  /**
   * 开始扫描
   */
  function startScan(): void {
    isScanning.value = true
    isPaused.value = false
  }

  /**
   * 暂停扫描
   */
  function pauseScan(): void {
    isPaused.value = true
  }

  /**
   * 恢复扫描
   */
  function resumeScan(): void {
    isPaused.value = false
  }

  /**
   * 停止扫描
   */
  function stopScan(): void {
    isScanning.value = false
    isPaused.value = false
  }

  /**
   * 更新进度
   * @param newProgress - 新进度
   */
  function updateProgress(newProgress: Partial<ScanProgress>): void {
    progress.value = { ...progress.value, ...newProgress }
  }

  /**
   * 添加扫描目录
   * @param dir - 目录路径
   */
  function addDirectory(dir: string): void {
    if (!directories.value.includes(dir)) {
      directories.value.push(dir)
    }
  }

  /**
   * 移除扫描目录
   * @param index - 目录索引
   */
  function removeDirectory(index: number): void {
    directories.value.splice(index, 1)
  }

  /**
   * 清空扫描目录
   */
  function clearDirectories(): void {
    directories.value = []
  }

  /**
   * 更新扫描配置
   * @param config - 新配置
   */
  function updateScanConfig(config: Partial<ScanConfig>): void {
    scanConfig.value = { ...scanConfig.value, ...config }
  }

  /**
   * 重置扫描状态
   */
  function resetScan(): void {
    isScanning.value = false
    isPaused.value = false
    progress.value = {
      total_files: 0,
      processed_files: 0,
      current_file: '',
      percentage: 0,
      is_paused: false,
      is_stopped: false,
      scanned_directories: 0,
      current_directory: '',
      recent_files: []
    }
  }

  return {
    // State
    isScanning,
    isPaused,
    progress,
    scanConfig,
    directories,

    // Getters
    scanPercentage,
    estimatedTimeRemaining,

    // Actions
    startScan,
    pauseScan,
    resumeScan,
    stopScan,
    updateProgress,
    addDirectory,
    removeDirectory,
    clearDirectories,
    updateScanConfig,
    resetScan
  }
})
