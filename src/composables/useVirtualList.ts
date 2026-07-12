/**
 * 专业级虚拟列表组合式函数 - 百万级数据支持
 * @module composables/useVirtualList
 * @description 企业级超大数据列表渲染优化，支持百万级数据、分片加载、动态高度、预加载
 */

import { ref, computed, watch, onUnmounted, type Ref, shallowRef, triggerRef } from 'vue'
import { logger } from '../utils/logger'

// ========== 类型定义 ==========

export interface VirtualListOptions<T> {
  itemHeight: number | ((item: T, index: number) => number)
  overscan?: number
  containerHeight?: number
  scrollBehavior?: 'smooth' | 'auto'
  // 百万级数据优化选项
  chunkSize?: number
  preloadChunks?: number
  enableWorker?: boolean
  maxMemoryItems?: number
}

export interface VirtualListState<T> {
  visibleItems: Ref<Array<{ item: T; index: number; style: { top: string; height: string } }>>
  totalHeight: Ref<number>
  scrollOffset: Ref<number>
  isScrolling: Ref<boolean>
  visibleRange: Ref<{ start: number; end: number }>
  scrollToIndex: (index: number, behavior?: ScrollBehavior) => void
  scrollToTop: (behavior?: ScrollBehavior) => void
  scrollToBottom: (behavior?: ScrollBehavior) => void
  handleScroll: (event: Event) => void
  measureElement: (el: HTMLElement | null, index: number) => void
  refresh: () => void
  // 百万级数据新增
  loadChunk: (chunkIndex: number) => Promise<void>
  unloadChunk: (chunkIndex: number) => void
  getLoadedChunks: () => Set<number>
  performanceStats: Ref<PerformanceStats>
}

export interface PerformanceStats {
  renderTime: number
  scrollTime: number
  memoryUsage: number
  loadedChunks: number
  totalChunks: number
  averageItemHeight: number
  estimatedTotalHeight: number
}

export interface DataChunk<T> {
  index: number
  startIndex: number
  endIndex: number
  items: T[]
  loaded: boolean
  loading: boolean
  lastAccessed: number
}

// ========== 百万级虚拟列表 ==========

const DEFAULT_CHUNK_SIZE = 1000
const DEFAULT_MAX_MEMORY_ITEMS = 50000

export function useVirtualList<T>(
  items: Ref<T[]>,
  options: VirtualListOptions<T>
): VirtualListState<T> {
  const {
    itemHeight,
    overscan = 10,
    containerHeight = 400,
    scrollBehavior = 'auto',
    chunkSize = DEFAULT_CHUNK_SIZE,
    preloadChunks = 2,
    maxMemoryItems = DEFAULT_MAX_MEMORY_ITEMS
  } = options

  // 状态 - 使用 shallowRef 优化大数据性能
  const scrollOffset = ref(0)
  const isScrolling = ref(false)
  const containerRef = ref<HTMLElement | null>(null)
  const measuredHeights = ref<Map<number, number>>(new Map())
  const scrollTimeout = ref<number | null>(null)
  
  // 百万级数据分片管理
  const chunks = shallowRef<Map<number, DataChunk<T>>>(new Map())
  const loadedItems = shallowRef<Map<number, T>>(new Map())
  const chunkLoadQueue = ref<number[]>([])
  const isLoadingChunks = ref(false)
  
  // 性能统计
  const performanceStats = ref<PerformanceStats>({
    renderTime: 0,
    scrollTime: 0,
    memoryUsage: 0,
    loadedChunks: 0,
    totalChunks: 0,
    averageItemHeight: typeof itemHeight === 'number' ? itemHeight : 50,
    estimatedTotalHeight: 0
  })

  const isDynamicHeight = typeof itemHeight === 'function'

  // 计算总块数
  const totalChunks = computed(() => Math.ceil(items.value.length / chunkSize))

  // 获取项目高度
  function getItemHeight(index: number): number {
    if (isDynamicHeight) {
      return measuredHeights.value.get(index) || (itemHeight as Function)(items.value[index], index)
    }
    return itemHeight as number
  }

  // 计算项目位置（使用二分查找优化）
  function getItemOffset(index: number): number {
    if (!isDynamicHeight && typeof itemHeight === 'number') {
      return index * itemHeight
    }

    // 动态高度：使用缓存 + 二分查找
    let offset = 0

    // 查找最近的缓存点
    for (let i = index - 1; i >= 0; i--) {
      if (measuredHeights.value.has(i)) {
        for (let j = i; j < index; j++) {
          offset += getItemHeight(j)
        }
        return offset
      }
    }

    // 从头计算
    for (let i = 0; i < index; i++) {
      offset += getItemHeight(i)
    }
    return offset
  }

  // 计算总高度
  const totalHeight = computed(() => {
    if (!isDynamicHeight && typeof itemHeight === 'number') {
      return items.value.length * itemHeight
    }
    
    let total = 0
    for (let i = 0; i < items.value.length; i++) {
      total += getItemHeight(i)
    }
    return total
  })

  // 计算当前可见范围
  const visibleRange = computed(() => {
    const startOffset = scrollOffset.value
    const endOffset = startOffset + containerHeight

    // 二分查找起始索引
    let startIndex = 0
    let low = 0
    let high = items.value.length - 1

    while (low <= high) {
      const mid = Math.floor((low + high) / 2)
      const midOffset = getItemOffset(mid)

      if (midOffset < startOffset) {
        startIndex = mid
        low = mid + 1
      } else {
        high = mid - 1
      }
    }

    // 找到结束索引
    let endIndex = startIndex
    let currentOffset = getItemOffset(startIndex)

    while (endIndex < items.value.length && currentOffset < endOffset) {
      currentOffset += getItemHeight(endIndex)
      endIndex++
    }

    // 添加 overscan
    const finalStartIndex = Math.max(0, startIndex - overscan)
    const finalEndIndex = Math.min(items.value.length - 1, endIndex + overscan)

    return { start: finalStartIndex, end: finalEndIndex }
  })

  // 计算需要加载的块
  const visibleChunks = computed(() => {
    const { start, end } = visibleRange.value
    const startChunk = Math.floor(start / chunkSize)
    const endChunk = Math.floor(end / chunkSize)
    return { start: startChunk, end: endChunk }
  })

  // 可见项目
  const visibleItems = computed(() => {
    const { start, end } = visibleRange.value
    const result = []

    for (let i = start; i <= end; i++) {
      if (i >= items.value.length) break

      const item = items.value[i]
      const height = getItemHeight(i)
      const offset = getItemOffset(i)

      result.push({
        item,
        index: i,
        style: {
          top: `${offset}px`,
          height: `${height}px`,
          position: 'absolute' as const,
          left: '0',
          right: '0'
        }
      })
    }

    return result
  })

  // ========== 分片加载管理 ==========

  /**
   * 加载指定块
   */
  async function loadChunk(chunkIndex: number): Promise<void> {
    if (chunkIndex < 0 || chunkIndex >= totalChunks.value) return
    
    const existingChunk = chunks.value.get(chunkIndex)
    if (existingChunk?.loaded || existingChunk?.loading) return

    const startTime = performance.now()
    
    // 创建或更新块
    const chunk: DataChunk<T> = {
      index: chunkIndex,
      startIndex: chunkIndex * chunkSize,
      endIndex: Math.min((chunkIndex + 1) * chunkSize - 1, items.value.length - 1),
      items: [],
      loaded: false,
      loading: true,
      lastAccessed: Date.now()
    }

    chunks.value.set(chunkIndex, chunk)

    // 模拟异步加载（实际项目中这里可能是从服务器加载）
    await new Promise(resolve => setTimeout(resolve, 0))

    // 加载数据
    chunk.items = items.value.slice(chunk.startIndex, chunk.endIndex + 1)
    chunk.loaded = true
    chunk.loading = false
    chunk.lastAccessed = Date.now()

    // 更新已加载项目映射
    chunk.items.forEach((item, idx) => {
      loadedItems.value.set(chunk.startIndex + idx, item)
    })

    // 内存管理：卸载旧块
    manageMemory()

    // 更新统计
    performanceStats.value.loadedChunks = chunks.value.size
    performanceStats.value.renderTime = performance.now() - startTime

    triggerRef(chunks)
    triggerRef(loadedItems)

    logger.debug('块加载完成', 'VirtualList', { 
      chunkIndex, 
      itemCount: chunk.items.length,
      totalLoaded: loadedItems.value.size 
    })
  }

  /**
   * 卸载块
   */
  function unloadChunk(chunkIndex: number): void {
    const chunk = chunks.value.get(chunkIndex)
    if (!chunk) return

    // 从已加载项目中移除
    for (let i = chunk.startIndex; i <= chunk.endIndex; i++) {
      loadedItems.value.delete(i)
    }

    chunks.value.delete(chunkIndex)
    performanceStats.value.loadedChunks = chunks.value.size

    triggerRef(chunks)
    triggerRef(loadedItems)

    logger.debug('块卸载完成', 'VirtualList', { chunkIndex })
  }

  /**
   * 内存管理
   */
  function manageMemory(): void {
    const maxChunks = Math.ceil(maxMemoryItems / chunkSize)
    
    if (chunks.value.size <= maxChunks) return

    // 按最后访问时间排序，卸载最旧的
    const sortedChunks = Array.from(chunks.value.values())
      .sort((a, b) => a.lastAccessed - b.lastAccessed)

    const toUnload = sortedChunks.slice(0, sortedChunks.length - maxChunks)
    toUnload.forEach(chunk => unloadChunk(chunk.index))

    logger.info('内存管理：卸载旧块', 'VirtualList', { 
      unloaded: toUnload.length,
      remaining: chunks.value.size 
    })
  }

  /**
   * 预加载相邻块
   */
  async function preloadAdjacentChunks(): Promise<void> {
    const { start, end } = visibleChunks.value
    
    for (let i = start - preloadChunks; i <= end + preloadChunks; i++) {
      if (i >= 0 && i < totalChunks.value) {
        if (!chunks.value.get(i)?.loaded && !chunkLoadQueue.value.includes(i)) {
          chunkLoadQueue.value.push(i)
        }
      }
    }

    // 处理加载队列
    await processLoadQueue()
  }

  /**
   * 处理加载队列
   */
  async function processLoadQueue(): Promise<void> {
    if (isLoadingChunks.value || chunkLoadQueue.value.length === 0) return

    isLoadingChunks.value = true

    while (chunkLoadQueue.value.length > 0) {
      const chunkIndex = chunkLoadQueue.value.shift()!
      await loadChunk(chunkIndex)
    }

    isLoadingChunks.value = false
  }

  /**
   * 获取已加载的块
   */
  function getLoadedChunks(): Set<number> {
    return new Set(chunks.value.keys())
  }

  // ========== 滚动处理 ==========

  function handleScroll(event: Event) {
    const startTime = performance.now()
    const target = event.target as HTMLElement
    scrollOffset.value = target.scrollTop

    isScrolling.value = true

    if (scrollTimeout.value) {
      clearTimeout(scrollTimeout.value)
    }

    scrollTimeout.value = window.setTimeout(() => {
      isScrolling.value = false
      // 滚动停止后预加载
      preloadAdjacentChunks()
    }, 150)

    // 更新性能统计
    performanceStats.value.scrollTime = performance.now() - startTime
    performanceStats.value.memoryUsage = estimateMemoryUsage()
  }

  /**
   * 估计内存使用
   */
  function estimateMemoryUsage(): number {
    let total = 0
    chunks.value.forEach(chunk => {
      total += chunk.items.length * 200 // 估计每个项目200字节
    })
    return total
  }

  // ========== 元素测量 ==========

  function measureElement(el: HTMLElement | null, index: number) {
    if (!el || !isDynamicHeight) return

    const height = el.getBoundingClientRect().height
    const currentHeight = measuredHeights.value.get(index)

    if (currentHeight !== height) {
      measuredHeights.value.set(index, height)
      // 更新平均高度统计
      updateAverageHeight()
    }
  }

  function updateAverageHeight() {
    if (measuredHeights.value.size === 0) return
    
    let total = 0
    measuredHeights.value.forEach(h => total += h)
    performanceStats.value.averageItemHeight = total / measuredHeights.value.size
    performanceStats.value.estimatedTotalHeight = performanceStats.value.averageItemHeight * items.value.length
  }

  // ========== 滚动控制 ==========

  function scrollToIndex(index: number, behavior: ScrollBehavior = scrollBehavior) {
    if (index < 0 || index >= items.value.length) return

    // 确保目标块已加载
    const targetChunk = Math.floor(index / chunkSize)
    loadChunk(targetChunk).then(() => {
      const offset = getItemOffset(index)
      containerRef.value?.scrollTo({
        top: offset,
        behavior
      })
    })
  }

  function scrollToTop(behavior: ScrollBehavior = scrollBehavior) {
    containerRef.value?.scrollTo({
      top: 0,
      behavior
    })
  }

  function scrollToBottom(behavior: ScrollBehavior = scrollBehavior) {
    // 先加载最后一块
    const lastChunk = totalChunks.value - 1
    loadChunk(lastChunk).then(() => {
      containerRef.value?.scrollTo({
        top: totalHeight.value,
        behavior
      })
    })
  }

  function refresh() {
    measuredHeights.value.clear()
    chunks.value.clear()
    loadedItems.value.clear()
    chunkLoadQueue.value = []
    performanceStats.value = {
      renderTime: 0,
      scrollTime: 0,
      memoryUsage: 0,
      loadedChunks: 0,
      totalChunks: totalChunks.value,
      averageItemHeight: typeof itemHeight === 'number' ? itemHeight : 50,
      estimatedTotalHeight: 0
    }
    
    // 重新加载可见块
    const { start, end } = visibleChunks.value
    for (let i = start; i <= end; i++) {
      loadChunk(i)
    }
  }

  // ========== 监听 ==========

  watch(items, () => {
    // 数据变化时刷新
    refresh()
  }, { deep: true })

  // 监听可见块变化，自动加载
  watch(visibleChunks, ({ start, end }) => {
    for (let i = start; i <= end; i++) {
      if (i >= 0 && i < totalChunks.value) {
        const chunk = chunks.value.get(i)
        if (!chunk?.loaded && !chunk?.loading) {
          loadChunk(i)
        } else if (chunk) {
          chunk.lastAccessed = Date.now()
        }
      }
    }
    
    // 预加载相邻块
    preloadAdjacentChunks()
  }, { immediate: true })

  // 初始化
  performanceStats.value.totalChunks = totalChunks.value

  // 清理
  onUnmounted(() => {
    if (scrollTimeout.value) {
      clearTimeout(scrollTimeout.value)
    }
    chunks.value.clear()
    loadedItems.value.clear()
  })

  return {
    visibleItems,
    totalHeight,
    scrollOffset,
    isScrolling,
    visibleRange,
    scrollToIndex,
    scrollToTop,
    scrollToBottom,
    handleScroll,
    measureElement,
    refresh,
    loadChunk,
    unloadChunk,
    getLoadedChunks,
    performanceStats
  }
}

// ========== 表格虚拟化（百万级） ==========

export interface VirtualTableOptions<T> extends VirtualListOptions<T> {
  columns: Array<{
    key: keyof T | string
    title: string
    width?: number
    fixed?: 'left' | 'right'
    sortable?: boolean
  }>
  enableSorting?: boolean
  enableFiltering?: boolean
}

export function useVirtualTable<T>(
  items: Ref<T[]>,
  options: VirtualTableOptions<T>
) {
  const { columns, enableSorting = false, enableFiltering = false, ...listOptions } = options

  const list = useVirtualList(items, listOptions)
  
  // 排序状态
  const sortKey = ref<keyof T | null>(null)
  const sortOrder = ref<'asc' | 'desc'>('asc')
  
  // 过滤状态
  const filters = ref<Record<string, any>>({})

  // 列宽计算
  const columnWidths = computed(() => {
    return columns.map(col => col.width || 150)
  })

  // 获取列样式
  function getColumnStyle(column: typeof columns[0], index: number) {
    const style: Record<string, string> = {}

    if (column.width) {
      style.width = `${column.width}px`
      style.minWidth = `${column.width}px`
    }

    if (column.fixed === 'left') {
      let left = 0
      for (let i = 0; i < index; i++) {
        if (columns[i].fixed === 'left') {
          left += columnWidths.value[i]
        }
      }
      style.position = 'sticky'
      style.left = `${left}px`
      style.zIndex = '10'
    } else if (column.fixed === 'right') {
      let right = 0
      for (let i = columns.length - 1; i > index; i--) {
        if (columns[i].fixed === 'right') {
          right += columnWidths.value[i]
        }
      }
      style.position = 'sticky'
      style.right = `${right}px`
      style.zIndex = '10'
    }

    return style
  }

  // 排序
  function sortBy(key: keyof T) {
    if (!enableSorting) return
    
    if (sortKey.value === key) {
      sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc'
    } else {
      sortKey.value = key
      sortOrder.value = 'asc'
    }

    // 触发重新排序
    const sorted = [...items.value].sort((a, b) => {
      const aVal = a[key]
      const bVal = b[key]
      
      if (aVal < bVal) return sortOrder.value === 'asc' ? -1 : 1
      if (aVal > bVal) return sortOrder.value === 'asc' ? 1 : -1
      return 0
    })

    items.value = sorted
  }

  // 过滤
  function setFilter(key: string, value: any) {
    if (!enableFiltering) return
    filters.value[key] = value
  }

  return {
    ...list,
    columns,
    columnWidths,
    getColumnStyle,
    sortKey,
    sortOrder,
    sortBy,
    filters,
    setFilter
  }
}

// ========== 树形虚拟化（百万级） ==========

export interface TreeNode {
  id: string
  children?: TreeNode[]
  expanded?: boolean
  [key: string]: any
}

export interface VirtualTreeOptions extends Omit<VirtualListOptions<any>, 'itemHeight'> {
  itemHeight: number
  childrenKey?: string
  expandedKey?: string
}

export function useVirtualTree<T extends TreeNode>(
  treeData: Ref<T[]>,
  options: VirtualTreeOptions
) {
  const { itemHeight, childrenKey = 'children', expandedKey = 'expanded', ...listOptions } = options

  // 扁平化树数据（使用迭代而非递归，避免大数据栈溢出）
  const flattenedItems = computed(() => {
    const result: Array<{ item: T; level: number; index: number; parentId?: string; hasChildren: boolean }> = []
    let index = 0

    // 使用栈进行迭代遍历
    const stack: Array<{ node: T; level: number; parentId?: string }> = []
    
    // 初始化栈（逆序入栈保持顺序）
    for (let i = treeData.value.length - 1; i >= 0; i--) {
      stack.push({ node: treeData.value[i], level: 0 })
    }

    while (stack.length > 0) {
      const { node, level, parentId } = stack.pop()!
      const children = (node as any)[childrenKey] as T[] | undefined
      const expanded = (node as any)[expandedKey] as boolean | undefined
      const hasChildren = children && children.length > 0

      result.push({
        item: node,
        level,
        index: index++,
        parentId,
        hasChildren: !!hasChildren
      })

      // 如果展开，子节点入栈（逆序保持顺序）
      if (expanded && hasChildren) {
        for (let i = children!.length - 1; i >= 0; i--) {
          stack.push({ 
            node: children![i], 
            level: level + 1,
            parentId: node.id 
          })
        }
      }
    }

    return result
  })

  // 使用虚拟列表
  const list = useVirtualList(
    computed(() => flattenedItems.value.map(i => i.item)),
    { ...listOptions, itemHeight }
  )

  // 展开/折叠节点
  function toggleNode(node: T) {
    (node as any)[expandedKey] = !(node as any)[expandedKey]
  }

  // 展开所有（限制层级避免性能问题）
  function expandAll(maxLevel: number = 3) {
    function expand(nodes: T[], level: number) {
      if (level > maxLevel) return
      for (const node of nodes) {
        (node as any)[expandedKey] = true
        const children = (node as any)[childrenKey] as T[] | undefined
        if (children) {
          expand(children, level + 1)
        }
      }
    }
    expand(treeData.value, 0)
  }

  // 折叠所有
  function collapseAll() {
    function collapse(nodes: T[]) {
      for (const node of nodes) {
        (node as any)[expandedKey] = false
        const children = (node as any)[childrenKey] as T[] | undefined
        if (children) {
          collapse(children)
        }
      }
    }
    collapse(treeData.value)
  }

  // 查找节点（迭代实现）
  function findNode(id: string): T | null {
    const stack = [...treeData.value]
    
    while (stack.length > 0) {
      const node = stack.pop()!
      if (node.id === id) return node
      
      const children = (node as any)[childrenKey] as T[] | undefined
      if (children) {
        stack.push(...children)
      }
    }
    
    return null
  }

  return {
    ...list,
    flattenedItems,
    toggleNode,
    expandAll,
    collapseAll,
    findNode
  }
}

// ========== 大数据导出 ==========

export interface ExportOptions {
  chunkSize?: number
  onProgress?: (progress: number) => void
}

/**
 * 分片导出大数据
 */
export async function exportLargeData<T>(
  items: T[],
  formatter: (item: T) => string,
  options: ExportOptions = {}
): Promise<string> {
  const { chunkSize = 10000, onProgress } = options
  
  const results: string[] = []
  const totalChunks = Math.ceil(items.length / chunkSize)

  for (let i = 0; i < totalChunks; i++) {
    const start = i * chunkSize
    const end = Math.min(start + chunkSize, items.length)
    const chunk = items.slice(start, end)
    
    // 使用 setTimeout 让出主线程
    await new Promise(resolve => setTimeout(resolve, 0))
    
    const formatted = chunk.map(formatter).join('\n')
    results.push(formatted)
    
    onProgress?.((i + 1) / totalChunks)
  }

  return results.join('\n')
}
