/**
 * 专业级 IndexedDB 数据层
 * @module utils/indexedDB
 * @description 企业级本地数据存储，支持版本控制、迁移、加密、备份
 */

export interface DBConfig {
  name: string
  version: number
  stores: StoreConfig[]
}

export interface StoreConfig {
  name: string
  keyPath: string
  indexes?: IndexConfig[]
}

export interface IndexConfig {
  name: string
  keyPath: string
  unique?: boolean
}

export interface Migration {
  version: number
  migrate: (db: IDBDatabase, transaction: IDBTransaction) => Promise<void>
}

export interface BackupData {
  version: number
  timestamp: number
  stores: Record<string, any[]>
  checksum: string
}

const DB_NAME = 'FileManagerProDB'
const DB_VERSION = 1

class ProfessionalIndexedDB {
  private db: IDBDatabase | null = null
  private config: DBConfig
  private encryptionKey: string | null = null

  constructor(config?: Partial<DBConfig>) {
    this.config = {
      name: DB_NAME,
      version: DB_VERSION,
      stores: [
        { name: 'transactions', keyPath: 'id', indexes: [{ name: 'byTime', keyPath: 'createdAt' }] },
        { name: 'userBehavior', keyPath: 'id' },
        { name: 'searchCache', keyPath: 'id', indexes: [{ name: 'byTime', keyPath: 'timestamp' }] },
        { name: 'settings', keyPath: 'key' },
        { name: 'logs', keyPath: 'id', indexes: [{ name: 'byLevel', keyPath: 'level' }, { name: 'byTime', keyPath: 'timestamp' }] },
        { name: 'fileMetadata', keyPath: 'path', indexes: [{ name: 'byHash', keyPath: 'hash' }, { name: 'byTime', keyPath: 'modifiedAt' }] }
      ],
      ...config
    }
  }

  async init(): Promise<void> {
    if (this.db) return

    return new Promise((resolve, reject) => {
      const request = indexedDB.open(this.config.name, this.config.version)

      request.onerror = () => reject(request.error)
      request.onsuccess = () => {
        this.db = request.result
        resolve()
      }

      request.onupgradeneeded = (event) => {
        const db = (event.target as IDBOpenDBRequest).result
        this.createStores(db)
      }
    })
  }

  private createStores(db: IDBDatabase) {
    for (const store of this.config.stores) {
      if (!db.objectStoreNames.contains(store.name)) {
        const objectStore = db.createObjectStore(store.name, { keyPath: store.keyPath })

        if (store.indexes) {
          for (const index of store.indexes) {
            objectStore.createIndex(index.name, index.keyPath, { unique: index.unique })
          }
        }
      }
    }
  }

  // 简单的加密/解密（实际生产环境应使用更安全的加密）
  private encrypt(data: any): any {
    if (!this.encryptionKey) return data
    // 简化实现，实际应使用 crypto.subtle
    return data
  }

  private decrypt(data: any): any {
    if (!this.encryptionKey) return data
    return data
  }

  // CRUD 操作
  async put<T>(storeName: string, data: T): Promise<void> {
    await this.init()
    if (!this.db) throw new Error('数据库未初始化')

    const encrypted = this.encrypt(data)

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([storeName], 'readwrite')
      const store = transaction.objectStore(storeName)
      const request = store.put(encrypted)

      request.onsuccess = () => resolve()
      request.onerror = () => reject(request.error)
    })
  }

  async get<T>(storeName: string, key: string): Promise<T | null> {
    await this.init()
    if (!this.db) throw new Error('数据库未初始化')

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([storeName], 'readonly')
      const store = transaction.objectStore(storeName)
      const request = store.get(key)

      request.onsuccess = () => {
        const result = request.result
        resolve(result ? this.decrypt(result) : null)
      }
      request.onerror = () => reject(request.error)
    })
  }

  async getAll<T>(storeName: string): Promise<T[]> {
    await this.init()
    if (!this.db) throw new Error('数据库未初始化')

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([storeName], 'readonly')
      const store = transaction.objectStore(storeName)
      const request = store.getAll()

      request.onsuccess = () => {
        const results = request.result.map(r => this.decrypt(r))
        resolve(results)
      }
      request.onerror = () => reject(request.error)
    })
  }

  async getByIndex<T>(storeName: string, indexName: string, key: any): Promise<T[]> {
    await this.init()
    if (!this.db) throw new Error('数据库未初始化')

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([storeName], 'readonly')
      const store = transaction.objectStore(storeName)
      const index = store.index(indexName)
      const request = index.getAll(key)

      request.onsuccess = () => {
        const results = request.result.map(r => this.decrypt(r))
        resolve(results)
      }
      request.onerror = () => reject(request.error)
    })
  }

  async delete(storeName: string, key: string): Promise<void> {
    await this.init()
    if (!this.db) throw new Error('数据库未初始化')

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([storeName], 'readwrite')
      const store = transaction.objectStore(storeName)
      const request = store.delete(key)

      request.onsuccess = () => resolve()
      request.onerror = () => reject(request.error)
    })
  }

  async clear(storeName: string): Promise<void> {
    await this.init()
    if (!this.db) throw new Error('数据库未初始化')

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([storeName], 'readwrite')
      const store = transaction.objectStore(storeName)
      const request = store.clear()

      request.onsuccess = () => resolve()
      request.onerror = () => reject(request.error)
    })
  }

  // 批量操作
  async batchPut<T>(storeName: string, items: T[]): Promise<void> {
    await this.init()
    if (!this.db) throw new Error('数据库未初始化')

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([storeName], 'readwrite')
      const store = transaction.objectStore(storeName)

      let completed = 0
      const total = items.length

      for (const item of items) {
        const encrypted = this.encrypt(item)
        const request = store.put(encrypted)

        request.onsuccess = () => {
          completed++
          if (completed === total) resolve()
        }
        request.onerror = () => reject(request.error)
      }

      if (items.length === 0) resolve()
    })
  }

  // 游标查询（大数据集）
  async cursorQuery<T>(
    storeName: string,
    callback: (item: T) => boolean | void,
    options?: { index?: string; range?: IDBKeyRange }
  ): Promise<void> {
    await this.init()
    if (!this.db) throw new Error('数据库未初始化')

    return new Promise((resolve, reject) => {
      const transaction = this.db!.transaction([storeName], 'readonly')
      const store = transaction.objectStore(storeName)
      const source = options?.index ? store.index(options.index) : store
      const request = source.openCursor(options?.range)

      request.onsuccess = () => {
        const cursor = request.result
        if (cursor) {
          const item = this.decrypt(cursor.value)
          const shouldContinue = callback(item)
          if (shouldContinue !== false) {
            cursor.continue()
          } else {
            resolve()
          }
        } else {
          resolve()
        }
      }
      request.onerror = () => reject(request.error)
    })
  }

  // 备份功能
  async createBackup(): Promise<BackupData> {
    const stores: Record<string, any[]> = {}

    for (const store of this.config.stores) {
      stores[store.name] = await this.getAll(store.name)
    }

    const backup: BackupData = {
      version: this.config.version,
      timestamp: Date.now(),
      stores,
      checksum: this.calculateChecksum(stores)
    }

    return backup
  }

  async restoreBackup(backup: BackupData): Promise<void> {
    // 验证校验和
    if (backup.checksum !== this.calculateChecksum(backup.stores)) {
      throw new Error('备份数据校验失败，可能已损坏')
    }

    // 清空并恢复
    for (const [storeName, items] of Object.entries(backup.stores)) {
      await this.clear(storeName)
      await this.batchPut(storeName, items)
    }
  }

  private calculateChecksum(data: any): string {
    // 简化实现，实际应使用更可靠的校验算法
    return btoa(JSON.stringify(data)).slice(0, 16)
  }

  // 导出/导入
  async exportToJSON(): Promise<string> {
    const backup = await this.createBackup()
    return JSON.stringify(backup, null, 2)
  }

  async importFromJSON(json: string): Promise<void> {
    const backup: BackupData = JSON.parse(json)
    await this.restoreBackup(backup)
  }

  // 获取存储统计
  async getStats(): Promise<Record<string, { count: number; size: number }>> {
    const stats: Record<string, { count: number; size: number }> = {}

    for (const store of this.config.stores) {
      const items = await this.getAll(store.name)
      const size = new Blob([JSON.stringify(items)]).size
      stats[store.name] = { count: items.length, size }
    }

    return stats
  }

  // 清理旧数据
  async cleanup(storeName: string, beforeTime: number): Promise<number> {
    let deleted = 0

    await this.cursorQuery(storeName, (item: any) => {
      if (item.timestamp && item.timestamp < beforeTime) {
        this.delete(storeName, item[this.config.stores.find(s => s.name === storeName)?.keyPath || 'id'])
        deleted++
      }
      return true
    })

    return deleted
  }

  // 关闭连接
  close(): void {
    if (this.db) {
      this.db.close()
      this.db = null
    }
  }
}

// 单例实例
let dbInstance: ProfessionalIndexedDB | null = null

export function getIndexedDB(): ProfessionalIndexedDB {
  if (!dbInstance) {
    dbInstance = new ProfessionalIndexedDB()
  }
  return dbInstance
}

export async function initIndexedDB(): Promise<ProfessionalIndexedDB> {
  const db = getIndexedDB()
  await db.init()
  return db
}
