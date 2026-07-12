<script setup lang="ts">
/**
 * 高级筛选组件
 * @component AdvancedFilter
 * @description 提供多条件组合筛选功能
 */

import { ref, computed } from 'vue'
import { Search, Filter, Refresh } from '@element-plus/icons-vue'
import type { FileInfo } from '../types'

interface FilterCriteria {
  fileName: string
  extensions: string[]
  minSize: number | null
  maxSize: number | null
  startDate: string
  endDate: string
  pathPattern: string
}

interface Props {
  files: FileInfo[]
}

const props = defineProps<Props>()

interface Emits {
  (e: 'filter', files: FileInfo[]): void
  (e: 'reset'): void
}

const emit = defineEmits<Emits>()

const showFilter = ref(false)

const criteria = ref<FilterCriteria>({
  fileName: '',
  extensions: [],
  minSize: null,
  maxSize: null,
  startDate: '',
  endDate: '',
  pathPattern: ''
})

const sizeOptions = [
  { label: '0 - 1 KB', min: 0, max: 1024 },
  { label: '1 KB - 1 MB', min: 1024, max: 1024 * 1024 },
  { label: '1 MB - 100 MB', min: 1024 * 1024, max: 100 * 1024 * 1024 },
  { label: '100 MB - 1 GB', min: 100 * 1024 * 1024, max: 1024 * 1024 * 1024 },
  { label: '> 1 GB', min: 1024 * 1024 * 1024, max: Infinity }
]

const filteredCount = computed(() => applyFilter().length)

function applyFilter(): FileInfo[] {
  return props.files.filter(file => {
    // 文件名筛选
    if (criteria.value.fileName && !file.filename.toLowerCase().includes(criteria.value.fileName.toLowerCase())) {
      return false
    }

    // 扩展名筛选
    if (criteria.value.extensions.length > 0) {
      const ext = file.file_extension.toLowerCase()
      if (!criteria.value.extensions.some(e => ext === e.toLowerCase())) {
        return false
      }
    }

    // 大小筛选
    if (criteria.value.minSize !== null && file.size < criteria.value.minSize) {
      return false
    }
    if (criteria.value.maxSize !== null && file.size > criteria.value.maxSize) {
      return false
    }

    // 日期筛选
    if (criteria.value.startDate) {
      const fileDate = new Date(file.modified_at)
      const startDate = new Date(criteria.value.startDate)
      if (fileDate < startDate) return false
    }
    if (criteria.value.endDate) {
      const fileDate = new Date(file.modified_at)
      const endDate = new Date(criteria.value.endDate)
      if (fileDate > endDate) return false
    }

    // 路径筛选
    if (criteria.value.pathPattern && !file.path.toLowerCase().includes(criteria.value.pathPattern.toLowerCase())) {
      return false
    }

    return true
  })
}

function handleFilter() {
  const result = applyFilter()
  emit('filter', result)
  showFilter.value = false
}

function handleReset() {
  criteria.value = {
    fileName: '',
    extensions: [],
    minSize: null,
    maxSize: null,
    startDate: '',
    endDate: '',
    pathPattern: ''
  }
  emit('reset')
}

function selectSizeRange(min: number, max: number) {
  criteria.value.minSize = min
  criteria.value.maxSize = max === Infinity ? null : max
}
</script>

<template>
  <div class="advanced-filter">
    <el-button @click="showFilter = true">
      <el-icon><Filter /></el-icon>
      高级筛选
    </el-button>

    <el-drawer
      v-model="showFilter"
      title="高级筛选"
      direction="rtl"
      size="400px"
    >
      <div class="filter-form">
        <el-form label-position="top">
          <el-form-item label="文件名">
            <el-input
              v-model="criteria.fileName"
              placeholder="支持模糊搜索"
              clearable
            />
          </el-form-item>

          <el-form-item label="文件类型">
            <el-select
              v-model="criteria.extensions"
              multiple
              placeholder="选择文件类型"
              clearable
            >
              <el-option label="图片" value="jpg,jpeg,png,gif,bmp,webp" />
              <el-option label="文档" value="pdf,doc,docx,xls,xlsx,ppt,pptx,txt" />
              <el-option label="视频" value="mp4,avi,mov,wmv,flv,mkv" />
              <el-option label="音频" value="mp3,wav,flac,ape,aac" />
              <el-option label="压缩包" value="zip,rar,7z,tar,gz" />
            </el-select>
          </el-form-item>

          <el-form-item label="文件大小">
            <div class="size-range-buttons">
              <el-button
                v-for="option in sizeOptions"
                :key="option.label"
                size="small"
                @click="selectSizeRange(option.min, option.max)"
                :type="criteria.minSize === option.min ? 'primary' : 'default'"
              >
                {{ option.label }}
              </el-button>
            </div>
            <div class="size-inputs">
              <el-input-number
                v-model="criteria.minSize"
                placeholder="最小"
                :min="0"
                :step="1024"
                style="width: 45%"
              />
              <span class="size-separator">-</span>
              <el-input-number
                v-model="criteria.maxSize"
                placeholder="最大"
                :min="0"
                :step="1024"
                style="width: 45%"
              />
            </div>
          </el-form-item>

          <el-form-item label="修改日期">
            <el-date-picker
              v-model="criteria.startDate"
              type="date"
              placeholder="开始日期"
              style="width: 48%"
            />
            <span class="date-separator">至</span>
            <el-date-picker
              v-model="criteria.endDate"
              type="date"
              placeholder="结束日期"
              style="width: 48%"
            />
          </el-form-item>

          <el-form-item label="路径包含">
            <el-input
              v-model="criteria.pathPattern"
              placeholder="路径关键词"
              clearable
            />
          </el-form-item>
        </el-form>

        <div class="filter-result">
          <el-alert
            :title="`筛选结果: ${filteredCount} 个文件`"
            type="info"
            :closable="false"
          />
        </div>
      </div>

      <template #footer>
        <div class="filter-actions">
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
          <el-button type="primary" @click="handleFilter">
            <el-icon><Search /></el-icon>
            应用筛选
          </el-button>
        </div>
      </template>
    </el-drawer>
  </div>
</template>

<style scoped>
.advanced-filter {
  display: inline-block;
}

.filter-form {
  padding: 20px;
}

.size-range-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
}

.size-inputs {
  display: flex;
  align-items: center;
  gap: 10px;
}

.size-separator,
.date-separator {
  color: #909399;
}

.filter-result {
  margin-top: 20px;
}

.filter-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}
</style>
