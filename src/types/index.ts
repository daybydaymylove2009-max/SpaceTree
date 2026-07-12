/**
 * 文件管理系统 - 公共类型定义
 * @module types/index
 * @description 定义系统中使用的所有数据类型和接口
 */

/**
 * 文件信息接口
 * @interface FileInfo
 */
export interface FileInfo {
  /** 文件ID */
  id: number
  /** 文件完整路径 */
  path: string
  /** 文件名 */
  filename: string
  /** 文件大小（字节） */
  size: number
  /** 文件哈希值 */
  hash: string | null
  /** 哈希算法 */
  hash_algorithm: string
  /** 创建时间 */
  created_at: string
  /** 修改时间 */
  modified_at: string
  /** 文件扩展名 */
  file_extension: string
}

/**
 * 重复文件组接口
 * @interface DuplicateGroup
 */
export interface DuplicateGroup {
  /** 组哈希值 */
  hash: string
  /** 组内文件列表 */
  files: FileInfo[]
  /** 总大小 */
  total_size: number
  /** 浪费的空间 */
  wasted_space: number
}

/**
 * 重复文件分类结果
 * @interface DuplicateClassification
 */
export interface DuplicateClassification {
  /** 完全相同（文件名与哈希都相同） */
  complete_duplicates: DuplicateGroup[]
  /** 名称相同（文件名相同，哈希不同） */
  name_duplicates: DuplicateGroup[]
  /** 内容相同（哈希相同，文件名不同） */
  content_duplicates: DuplicateGroup[]
}

/**
 * 扫描配置接口
 * @interface ScanConfig
 */
export interface ScanConfig {
  /** 最大扫描深度 */
  max_depth: number
  /** 包含的文件扩展名 */
  include_extensions: string[]
  /** 排除的文件扩展名 */
  exclude_extensions: string[]
  /** 最小文件大小 */
  min_size: number
  /** 最大文件大小 */
  max_size: number
  /** 是否排除隐藏文件 */
  exclude_hidden: boolean
  /** 是否排除系统文件 */
  exclude_system: boolean
  /** 排除模式 */
  exclude_patterns: string[]
  /** 哈希算法 */
  hash_algorithm: 'xxhash3' | 'xxhash64' | 'blake3' | 'sha256' | 'sha512' | 'md5'
}

/**
 * 扫描进度接口
 * @interface ScanProgress
 */
export interface ScanProgress {
  /** 总文件数 */
  total_files: number
  /** 已处理文件数 */
  processed_files: number
  /** 当前文件 */
  current_file: string
  /** 进度百分比 */
  percentage: number
  /** 是否暂停 */
  is_paused: boolean
  /** 是否停止 */
  is_stopped: boolean
  /** 已扫描目录数 */
  scanned_directories: number
  /** 当前目录 */
  current_directory: string
  /** 最近扫描的文件 */
  recent_files: string[]
}

/**
 * 扫描历史记录接口
 * @interface ScanHistory
 */
export interface ScanHistory {
  id: number
  scan_mode: 'full' | 'incremental'
  directories: string[]
  total_files: number
  duplicate_groups: number
  duplicate_files: number
  wasted_space: number
  duration_seconds: number
  status: 'completed' | 'interrupted'
  scan_time: string
}

/**
 * 分析报告摘要接口
 * @interface AnalysisSummary
 */
export interface AnalysisSummary {
  total_files_analyzed: number
  total_duplicate_groups: number
  total_duplicate_files: number
  total_wasted_space: number
  top_insight: string
  recommendations: string[]
  compliance_score?: number
  risk_level?: string
}

/**
 * 文件类型详情接口
 * @interface FileTypeDetail
 */
export interface FileTypeDetail {
  extension: string
  file_count: number
  duplicate_count: number
  total_size: number
  wasted_space: number
  percentage: number
}

/**
 * 大小范围详情接口
 * @interface SizeRangeDetail
 */
export interface SizeRangeDetail {
  range_label: string
  min_bytes: number
  max_bytes: number
  file_count: number
  group_count: number
  total_size: number
  wasted_space: number
  percentage: number
}

/**
 * 大文件信息接口
 * @interface LargeFileInfo
 */
export interface LargeFileInfo {
  hash: string
  size: number
  file_count: number
  locations: string[]
  potential_savings: number
  modified_at: string
  filenames: string[]
}

/**
 * 目录统计接口
 * @interface DirectoryStats
 */
export interface DirectoryStats {
  directory: string
  duplicate_file_count: number
  duplicate_group_count: number
  wasted_space: number
  percentage_of_total: number
}

/**
 * 多维度分析报告接口
 * @interface MultiDimensionalReport
 */
export interface MultiDimensionalReport {
  by_path: {
    directory_tree: any[]
    top_duplicate_directories: DirectoryStats[]
    path_depth_distribution: any[]
    cross_directory_duplicates: any[]
  }
  by_type: {
    type_distribution: FileTypeDetail[]
    top_duplicate_extensions: any[]
    file_types?: FileTypeDetail[]
  }
  by_size: {
    size_ranges: SizeRangeDetail[]
    large_duplicate_files: LargeFileInfo[]
    size_efficiency_metrics: any
  }
  by_owner: any
  charts: {
    pie_charts: any
    bar_charts: any
  }
  summary: AnalysisSummary
  generated_at: string
}

/**
 * 清理影响分析结果接口
 * @interface CleanupImpactResult
 */
export interface CleanupImpactResult {
  files_to_delete: number
  space_to_free: number
  total_groups: number
  safety_score: number
  warnings: string[]
}

/**
 * 相似图片组接口
 * @interface SimilarImageGroup
 */
export interface SimilarImageGroup {
  hash: string
  files: FileInfo[]
  similarity: number
}

/**
 * 图片相似度配置接口
 * @interface ImageSimilarityConfig
 */
export interface ImageSimilarityConfig {
  algorithm: 'ahash' | 'dhash' | 'phash'
  threshold: number
}

/**
 * 回收站项目接口
 * @interface RecycleBinItem
 */
export interface RecycleBinItem {
  original_path: string
  deleted_at: string
  size: number
}

/**
 * 审计日志接口
 * @interface AuditLog
 */
export interface AuditLog {
  id: number
  timestamp: string
  event_type: string
  severity: 'critical' | 'high' | 'medium' | 'low' | 'info'
  resource: string
  action: string
  details: string
  result: string
}

/**
 * 应用设置接口
 * @interface AppSettings
 */
export interface AppSettings {
  theme: 'light' | 'dark' | 'auto'
  language: string
  auto_scan: boolean
  default_scan_config: ScanConfig
  notification_enabled: boolean
  delete_confirmation: boolean
}

/**
 * 菜单项类型
 * @type MenuKey
 */
export type MenuKey =
  | 'scan'
  | 'search'
  | 'directory'
  | 'duplicates'
  | 'image_archive'
  | 'analysis'
  | 'tools'
  | 'settings'
  | 'about'

/**
 * 重复文件类型
 * @type DuplicateType
 */
export type DuplicateType = 'complete' | 'name' | 'content'

/**
 * 哈希算法选项
 * @interface HashAlgorithmOption
 */
export interface HashAlgorithmOption {
  value: ScanConfig['hash_algorithm']
  label: string
  description: string
  speed: string
  security: string
  useCase: string
}

/**
 * 导出格式类型
 * @type ExportFormat
 */
export type ExportFormat = 'csv' | 'md' | 'json'

/**
 * 扫描模式类型
 * @type ScanMode
 */
export type ScanMode = 'full' | 'incremental'

/**
 * 扫描状态类型
 * @type ScanStatus
 */
export type ScanStatus = 'running' | 'paused' | 'stopped' | 'completed'
