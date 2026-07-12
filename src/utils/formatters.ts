/**
 * 文件管理系统 - 格式化工具函数库
 * @module utils/formatters
 * @description 提供文件大小、日期等数据的格式化功能
 */

/**
 * 格式化文件大小
 * @param bytes - 文件字节数
 * @returns 格式化后的大小字符串 (如: "1.5 MB")
 * @example
 * formatSize(1024) // "1 KB"
 * formatSize(1536000) // "1.46 MB"
 */
export function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

/**
 * 格式化日期时间
 * @param dateStr - ISO 格式的日期字符串
 * @returns 本地化日期时间字符串 (如: "2024/01/15 14:30")
 * @example
 * formatDate('2024-01-15T14:30:00Z') // "2024/01/15 14:30"
 */
export function formatDate(dateStr: string): string {
  if (!dateStr) return '-'
  try {
    const date = new Date(dateStr)
    return date.toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    })
  } catch {
    return dateStr
  }
}

/**
 * 格式化日期 (仅日期部分)
 * @param dateStr - ISO 格式的日期字符串
 * @returns 本地化日期字符串 (如: "2024/01/15")
 */
export function formatDateOnly(dateStr: string): string {
  if (!dateStr) return '-'
  try {
    const date = new Date(dateStr)
    return date.toLocaleDateString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit'
    })
  } catch {
    return dateStr
  }
}

/**
 * 格式化时长
 * @param seconds - 秒数
 * @returns 格式化后的时长字符串 (如: "2小时30分钟")
 */
export function formatDuration(seconds: number): string {
  if (seconds < 60) {
    return seconds + '秒'
  } else if (seconds < 3600) {
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return secs > 0 ? `${mins}分${secs}秒` : `${mins}分`
  } else {
    const hours = Math.floor(seconds / 3600)
    const mins = Math.floor((seconds % 3600) / 60)
    return mins > 0 ? `${hours}小时${mins}分` : `${hours}小时`
  }
}

/**
 * 格式化文件数量
 * @param count - 文件数量
 * @returns 带千分位分隔符的数字字符串
 */
export function formatFileCount(count: number): string {
  return count.toLocaleString('zh-CN')
}

/**
 * 截断路径显示
 * @param path - 完整路径
 * @param maxLength - 最大显示长度
 * @returns 截断后的路径 (如: "C:\\...\\Documents\\file.txt")
 */
export function truncatePath(path: string, maxLength: number = 50): string {
  if (path.length <= maxLength) return path
  const start = path.substring(0, 15)
  const end = path.substring(path.length - (maxLength - 20))
  return `${start}...${end}`
}

/**
 * 获取文件扩展名
 * @param filename - 文件名
 * @returns 扩展名 (小写，不含点号)
 */
export function getFileExtension(filename: string): string {
  const lastDot = filename.lastIndexOf('.')
  return lastDot === -1 ? '' : filename.substring(lastDot + 1).toLowerCase()
}

/**
 * 获取文件名 (不含扩展名)
 * @param filename - 完整文件名
 * @returns 不含扩展名的文件名
 */
export function getFileNameWithoutExtension(filename: string): string {
  const lastDot = filename.lastIndexOf('.')
  return lastDot === -1 ? filename : filename.substring(0, lastDot)
}
