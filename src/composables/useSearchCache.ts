/**
 * 搜索缓存组合式函数
 * @module composables/useSearchCache
 * @description 专业级搜索缓存管理，支持LRU淘汰、过期时间、持久化
 */

import { ref, computed, type Ref } from 'vue'
import { getIndexedDB } from '../utils/indexedDB'
import { logger } from '../utils/logger'

export interface SearchCacheEntry<T> {
  id: string
  query: string
  filters: any
  results: T[]
  total: number
  timestamp: number
  hits: number
  expiresAt: number
}

export interface SearchCacheOptions {
  maxSize?: number
  ttl?: number
  persist?: boolean
}

const DEFAULT_OPTIONS: SearchCacheOptions = {
  maxSize: 100,
  ttl: 5 * 60 * 1000, // 5分钟
  persist: true
}

export function useSearchCache<T>(options: SearchCacheOptions = {}) {
  const { maxSize, ttl, persist } = { ...DEFAULT_OPTIONS, ...options }
  const db = getIndexedDB()
  const DB_STORE = 'searchCache'

  // 使用普通 Map 而不是 ref(Map) 来避免类型问题
  const cacheMap = new Map<string, SearchCacheEntry<T>>()
  const cache = ref<SearchCacheEntry<T>[]>([]) as Ref<SearchCacheEntry<T>[]>
  const isLoading = ref(false)

  const size = computed(() => cacheMap.size)
  const entries = computed(() => Array.from(cacheMap.values()))

  /**
   * 生成缓存键
   */
  function generateKey(query: string, filters: any): string {
    return btoa(JSON.stringify({ query, filters })).slice(0, 50)
  }

  /**
   * 从IndexedDB加载缓存
   */
  async function loadFromDB(): Promise<void> {
    if (!persist) return

    isLoading.value = true
    try {
      const dbEntries = await db.getAll<SearchCacheEntry<T>>(DB_STORE)
      const now = Date.now()
      let loaded = 0
      let expired = 0

      for (const entry of dbEntries) {
        if (entry.expiresAt > now) {
          cacheMap.set(entry.id, entry)
          loaded++
        } else {
          expired++
          await db.delete(DB_STORE, entry.id)
        }
      }
      cache.value = Array.from(cacheMap.values())

      logger.info('搜索缓存加载完成', 'SearchCache', { loaded, expired })
    } catch (error) {
      logger.error('加载搜索缓存失败', 'SearchCache', {}, error as Error)
    } finally {
      isLoading.value = false
    }
  }

  /**
   * 保存到IndexedDB
   */
  async function saveToDB(entry: SearchCacheEntry<T>): Promise<void> {
    if (!persist) return

    try {
      await db.put(DB_STORE, entry)
    } catch (error) {
      logger.error('保存搜索缓存失败', 'SearchCache', { entryId: entry.id }, error as Error)
    }
  }

  /**
   * 获取缓存
   */
  function get(query: string, filters: any): SearchCacheEntry<T> | null {
    const key = generateKey(query, filters)
    const entry = cacheMap.get(key)

    if (!entry) return null

    // 检查过期
    if (entry.expiresAt < Date.now()) {
      cacheMap.delete(key)
      if (persist) {
        db.delete(DB_STORE, key).catch(() => {})
      }
      return null
    }

    // 更新命中次数
    entry.hits++
    return entry
  }

  /**
   * 设置缓存
   */
  async function set(query: string, filters: any, results: T[], total: number): Promise<void> {
    const key = generateKey(query, filters)

    // LRU淘汰
    if (cacheMap.size >= maxSize! && !cacheMap.has(key)) {
      const oldestKey = cacheMap.keys().next().value
      if (oldestKey !== undefined) {
        cacheMap.delete(oldestKey)
        if (persist) {
          await db.delete(DB_STORE, oldestKey)
        }
      }
    }

    const entry: SearchCacheEntry<T> = {
      id: key,
      query,
      filters,
      results,
      total,
      timestamp: Date.now(),
      hits: 1,
      expiresAt: Date.now() + ttl!
    }

    cacheMap.set(key, entry)
    cache.value = Array.from(cacheMap.values())
    await saveToDB(entry)
  }

  /**
   * 清除缓存
   */
  async function clear(): Promise<void> {
    cacheMap.clear()
    cache.value = []

    if (persist) {
      try {
        await db.clear(DB_STORE)
        logger.info('搜索缓存已清除', 'SearchCache')
      } catch (error) {
        logger.error('清除搜索缓存失败', 'SearchCache', {}, error as Error)
      }
    }
  }

  /**
   * 使缓存失效
   */
  async function invalidate(pattern?: RegExp): Promise<void> {
    if (!pattern) {
      await clear()
      return
    }

    for (const [key, entry] of cacheMap.entries()) {
      if (pattern.test(entry.query) || pattern.test(JSON.stringify(entry.filters))) {
        cacheMap.delete(key)
        if (persist) {
          await db.delete(DB_STORE, key)
        }
      }
    }
    cache.value = Array.from(cacheMap.values())
  }

  /**
   * 获取统计
   */
  function getStats() {
    const cacheEntries = Array.from(cacheMap.values())
    const totalHits = cacheEntries.reduce((sum, e) => sum + e.hits, 0)

    return {
      size: cacheMap.size,
      totalHits,
      averageHits: cacheEntries.length > 0 ? totalHits / cacheEntries.length : 0,
      oldestEntry: cacheEntries.length > 0 ? Math.min(...cacheEntries.map(e => e.timestamp)) : null,
      newestEntry: cacheEntries.length > 0 ? Math.max(...cacheEntries.map(e => e.timestamp)) : null
    }
  }

  // 初始化加载
  loadFromDB()

  return {
    cache: entries,
    size,
    isLoading,
    get,
    set,
    clear,
    invalidate,
    getStats
  }
}
