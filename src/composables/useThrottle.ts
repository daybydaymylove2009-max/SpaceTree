/**
 * 专业级限流和防抖组合式函数
 * @module composables/useThrottle
 * @description 企业级性能优化工具，支持防抖、节流、缓存、记忆化
 */

import { ref, computed } from 'vue'
import type { Ref } from 'vue'

// ========== 防抖 ==========

export interface DebounceOptions {
  wait?: number
  immediate?: boolean
  maxWait?: number
}

export function useDebounce<T extends (...args: any[]) => any>(
  fn: T,
  options: DebounceOptions = {}
): {
  run: (...args: Parameters<T>) => void
  cancel: () => void
  flush: () => ReturnType<T> | undefined
  pending: Ref<boolean>
} {
  const { wait = 300, immediate = false, maxWait } = options

  let timeoutId: number | null = null
  let maxTimeoutId: number | null = null
  let lastArgs: Parameters<T> | null = null
  let lastCallTime: number | null = null
  let result: ReturnType<T>

  const pending = ref(false)

  function invokeFunc(): ReturnType<T> {
    const args = lastArgs!
    lastArgs = null
    lastCallTime = null
    result = fn(...args)
    return result
  }

  function startTimer(waitTime: number): number {
    return window.setTimeout(() => {
      if (maxWait !== undefined) {
        maxTimeoutId = null
      }
      const isInvoking = lastCallTime !== null
      if (isInvoking) {
        const timeSinceLastCall = Date.now() - lastCallTime!
        const timeWaiting = wait - timeSinceLastCall

        if (timeWaiting <= 0) {
          flush()
        } else {
          timeoutId = startTimer(timeWaiting)
        }
      } else {
        pending.value = false
      }
    }, waitTime)
  }

  function leadingEdge(): ReturnType<T> | undefined {
    lastCallTime = Date.now()
    timeoutId = startTimer(wait)
    return immediate ? invokeFunc() : undefined
  }

  function debounced(...args: Parameters<T>): void {
    lastArgs = args
    lastCallTime = Date.now()
    pending.value = true

    const isInvoking = timeoutId === null

    if (isInvoking) {
      leadingEdge()
    }

    if (maxWait !== undefined && maxTimeoutId === null) {
      maxTimeoutId = window.setTimeout(() => {
        flush()
      }, maxWait)
    }
  }

  function cancel(): void {
    if (timeoutId !== null) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
    if (maxTimeoutId !== null) {
      clearTimeout(maxTimeoutId)
      maxTimeoutId = null
    }
    lastArgs = null
    lastCallTime = null
    pending.value = false
  }

  function flush(): ReturnType<T> | undefined {
    if (timeoutId !== null) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
    if (maxTimeoutId !== null) {
      clearTimeout(maxTimeoutId)
      maxTimeoutId = null
    }
    if (lastArgs) {
      return invokeFunc()
    }
    return result
  }

  return {
    run: debounced,
    cancel,
    flush,
    pending
  }
}

// ========== 节流 ==========

export interface ThrottleOptions {
  wait?: number
  leading?: boolean
  trailing?: boolean
}

export function useThrottle<T extends (...args: any[]) => any>(
  fn: T,
  options: ThrottleOptions = {}
): {
  run: (...args: Parameters<T>) => void
  cancel: () => void
  flush: () => ReturnType<T> | undefined
  pending: Ref<boolean>
} {
  const { wait = 300, leading = true, trailing = true } = options

  let timeoutId: number | null = null
  let previous = 0
  let lastArgs: Parameters<T> | null = null
  let result: ReturnType<T>

  const pending = ref(false)

  function invokeFunc(time: number): ReturnType<T> {
    const args = lastArgs!
    lastArgs = null
    previous = time
    result = fn(...args)
    return result
  }

  function leadingEdge(time: number): ReturnType<T> | undefined {
    previous = time
    timeoutId = window.setTimeout(timerExpired, wait)
    return leading ? invokeFunc(time) : result
  }

  function remainingWait(time: number): number {
    const timeSinceLastCall = time - previous
    return wait - timeSinceLastCall
  }

  function shouldInvoke(time: number): boolean {
    const timeSinceLastCall = time - previous
    return previous === 0 || timeSinceLastCall >= wait
  }

  function timerExpired(): void {
    const time = Date.now()
    if (shouldInvoke(time)) {
      return trailingEdge(time)
    }
    timeoutId = window.setTimeout(timerExpired, remainingWait(time))
  }

  function trailingEdge(time: number): ReturnType<T> | undefined {
    timeoutId = null
    if (trailing && lastArgs) {
      return invokeFunc(time)
    }
    lastArgs = null
    pending.value = false
    return result
  }

  function throttled(...args: Parameters<T>): void {
    const time = Date.now()
    const isInvoking = shouldInvoke(time)
    lastArgs = args
    pending.value = true

    if (isInvoking) {
      if (timeoutId === null) {
        return leadingEdge(time)
      }
    }

    if (timeoutId === null) {
      timeoutId = window.setTimeout(timerExpired, wait)
    }
  }

  function cancel(): void {
    if (timeoutId !== null) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
    previous = 0
    lastArgs = null
    pending.value = false
  }

  function flush(): ReturnType<T> | undefined {
    if (timeoutId !== null) {
      clearTimeout(timeoutId)
      timeoutId = null
    }
    if (lastArgs) {
      return invokeFunc(Date.now())
    }
    return result
  }

  return {
    run: throttled,
    cancel,
    flush,
    pending
  }
}

// ========== 请求缓存 ==========

export interface CacheOptions {
  maxAge?: number
  maxSize?: number
  keyGenerator?: (...args: any[]) => string
}

interface CacheEntry<T> {
  value: T
  timestamp: number
  hits: number
}

export function useRequestCache<T extends (...args: any[]) => Promise<any>>(
  fn: T,
  options: CacheOptions = {}
): {
  run: (...args: Parameters<T>) => Promise<ReturnType<T>>
  clear: () => void
  getStats: () => { size: number; hitRate: number; entries: Array<{ key: string; hits: number; age: number }> }
  invalidate: (keyPattern: RegExp) => void
} {
  const { maxAge = 60000, maxSize = 100, keyGenerator = (...args: any[]) => JSON.stringify(args) } = options

  const cache = new Map<string, CacheEntry<ReturnType<T>>>()
  let hits = 0
  let misses = 0

  async function cached(...args: Parameters<T>): Promise<ReturnType<T>> {
    const key = String(keyGenerator(...args))
    const now = Date.now()
    const entry = cache.get(key)

    if (entry && now - entry.timestamp < maxAge) {
      entry.hits++
      hits++
      return entry.value
    }

    misses++

    // 清理过期缓存
    if (entry) {
      cache.delete(key)
    }

    // 检查容量
    if (cache.size >= maxSize) {
      const oldestKey = cache.keys().next().value
      if (oldestKey !== undefined) {
        cache.delete(String(oldestKey))
      }
    }

    const value = await fn(...args)
    cache.set(key, { value, timestamp: now, hits: 1 })
    return value
  }

  function clear(): void {
    cache.clear()
    hits = 0
    misses = 0
  }

  function getStats() {
    const total = hits + misses
    const entries = Array.from(cache.entries()).map(([key, entry]) => ({
      key,
      hits: entry.hits,
      age: Date.now() - entry.timestamp
    }))

    return {
      size: cache.size,
      hitRate: total > 0 ? hits / total : 0,
      entries: entries.sort((a, b) => b.hits - a.hits)
    }
  }

  function invalidate(keyPattern: RegExp): void {
    for (const key of cache.keys()) {
      if (keyPattern.test(key)) {
        cache.delete(key)
      }
    }
  }

  return {
    run: cached,
    clear,
    getStats,
    invalidate
  }
}

// ========== 记忆化 ==========

export function useMemoize<T extends (...args: any[]) => any>(
  fn: T,
  options: { keyGenerator?: (...args: any[]) => string } = {}
): {
  call: (...args: Parameters<T>) => ReturnType<T>
  clear: () => void
  getCache: () => Map<string, ReturnType<T>>
} {
  const { keyGenerator = (...args: any[]) => JSON.stringify(args) } = options
  const cache = new Map<string, ReturnType<T>>()

  function memoized(...args: Parameters<T>): ReturnType<T> {
    const key = String(keyGenerator(...args))

    if (cache.has(key)) {
      return cache.get(key)!
    }

    const result = fn(...args)
    cache.set(key, result)
    return result
  }

  function clear(): void {
    cache.clear()
  }

  function getCache(): Map<string, ReturnType<T>> {
    return new Map(cache)
  }

  return {
    call: memoized,
    clear,
    getCache
  }
}

// ========== 异步队列 ==========

export interface QueueOptions {
  concurrency?: number
  retry?: number
  retryDelay?: number
}

export interface QueueTask<T> {
  id: string
  fn: () => Promise<T>
  resolve: (value: T) => void
  reject: (error: any) => void
  retries: number
}

export function useAsyncQueue(options: QueueOptions = {}) {
  const { concurrency = 3, retry = 3, retryDelay = 1000 } = options

  const queue: QueueTask<any>[] = []
  const running = ref(0)
  const completed = ref(0)
  const failed = ref(0)

  const isIdle = computed(() => running.value === 0 && queue.length === 0)
  const isBusy = computed(() => running.value >= concurrency)
  const progress = computed(() => {
    const total = completed.value + failed.value + queue.length + running.value
    return total > 0 ? (completed.value + failed.value) / total : 0
  })

  async function processNext(): Promise<void> {
    if (queue.length === 0 || running.value >= concurrency) return

    const task = queue.shift()!
    running.value++

    try {
      const result = await task.fn()
      task.resolve(result)
      completed.value++
    } catch (error) {
      if (task.retries < retry) {
        task.retries++
        await new Promise(resolve => setTimeout(resolve, retryDelay))
        queue.unshift(task)
      } else {
        task.reject(error)
        failed.value++
      }
    } finally {
      running.value--
      processNext()
    }
  }

  function add<T>(fn: () => Promise<T>): Promise<T> {
    return new Promise((resolve, reject) => {
      const task: QueueTask<T> = {
        id: `${Date.now()}-${Math.random()}`,
        fn,
        resolve,
        reject,
        retries: 0
      }
      queue.push(task)
      processNext()
    })
  }

  function clear(): void {
    queue.length = 0
  }

  async function drain(): Promise<void> {
    while (!isIdle.value) {
      await new Promise(resolve => setTimeout(resolve, 100))
    }
  }

  return {
    add,
    clear,
    drain,
    isIdle,
    isBusy,
    progress,
    stats: {
      running,
      completed,
      failed,
      pending: computed(() => queue.length)
    }
  }
}

// ========== 搜索优化 ==========

export interface SearchOptions {
  debounceMs?: number
  minChars?: number
  maxResults?: number
  highlightMatches?: boolean
}

export function useOptimizedSearch<T>(
  items: Ref<T[]>,
  searchFields: (keyof T)[],
  options: SearchOptions = {}
) {
  const {
    debounceMs = 300,
    minChars = 2,
    maxResults = 100,
    highlightMatches = true
  } = options

  const searchQuery = ref('')
  const isSearching = ref(false)
  const results = ref<T[]>([])

  function getFieldValue(item: T, field: keyof T): string {
    const value = item[field]
    return value ? String(value).toLowerCase() : ''
  }

  function highlightText(text: string, query: string): string {
    if (!highlightMatches || !query) return text
    const regex = new RegExp(`(${escapeRegExp(query)})`, 'gi')
    return text.replace(regex, '<mark>$1</mark>')
  }

  function escapeRegExp(string: string): string {
    return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  }

  const performSearch = useDebounce(() => {
    const query = searchQuery.value.trim().toLowerCase()

    if (query.length < minChars) {
      results.value = []
      isSearching.value = false
      return
    }

    isSearching.value = true

    const filtered = items.value.filter(item => {
      return searchFields.some(field => {
        const value = getFieldValue(item, field)
        return value.includes(query)
      })
    })

    results.value = filtered.slice(0, maxResults)
    isSearching.value = false
  }, { wait: debounceMs })

  function setQuery(query: string) {
    searchQuery.value = query
    performSearch.run()
  }

  return {
    query: searchQuery,
    isSearching,
    results,
    setQuery,
    highlightText
  }
}
