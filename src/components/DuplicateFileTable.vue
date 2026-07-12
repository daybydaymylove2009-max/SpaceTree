<script setup lang="ts">
/**
 * 重复文件表格组件
 * @component DuplicateFileTable
 * @description 显示重复文件列表的表格组件，支持日期显示和操作按钮
 */

import { Document, FolderOpened, Delete, View } from '@element-plus/icons-vue'
import { ref } from 'vue'
import type { FileInfo } from '../types'
import { formatSize, formatDate } from '../utils/formatters'
import { useFileOperations } from '../composables/useFileOperations'
import FilePreview from './FilePreview.vue'

/**
 * 组件属性定义
 */
interface Props {
  /** 文件列表数据 */
  files: FileInfo[]
  /** 数据库路径 */
  dbPath: string
  /** 是否显示操作列 */
  showActions?: boolean
  /** 表格高度 */
  height?: string | number
  /** 是否可排序 */
  sortable?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  showActions: true,
  height: undefined,
  sortable: true
})

/**
 * 组件事件定义
 */
interface Emits {
  /** 文件删除后触发 */
  (e: 'fileDeleted', file: FileInfo): void
  /** 文件操作后触发刷新 */
  (e: 'refresh'): void
}

const emit = defineEmits<Emits>()

// 使用文件操作组合式函数
const { openFile, openFolder, deleteFile } = useFileOperations({
  dbPath: props.dbPath,
  onDeleteSuccess: () => emit('refresh')
})

// 预览相关状态
const previewVisible = ref(false)
const previewFile = ref<FileInfo | null>(null)

/**
 * 打开文件预览
 * @param file - 文件信息
 */
function openPreview(file: FileInfo) {
  previewFile.value = file
  previewVisible.value = true
}

/**
 * 关闭预览
 */
function closePreview() {
  previewVisible.value = false
  previewFile.value = null
}

/**
 * 包装删除函数，添加事件发射
 * @param fileInfo - 文件信息
 */
async function handleDelete(fileInfo: FileInfo): Promise<void> {
  const success = await deleteFile(fileInfo)
  if (success) {
    emit('fileDeleted', fileInfo)
  }
}
</script>

<template>
  <el-table
    :data="files"
    stripe
    :height="height"
    v-bind="$attrs"
  >
    <!-- 文件名列 -->
    <el-table-column
      label="文件名"
      prop="filename"
      min-width="150"
      show-overflow-tooltip
      :sortable="sortable"
    />

    <!-- 文件大小列 -->
    <el-table-column
      label="大小"
      width="100"
      :sortable="sortable"
      sort-by="size"
    >
      <template #default="{ row }">
        <span class="file-size">{{ formatSize(row.size) }}</span>
      </template>
    </el-table-column>

    <!-- 修改日期列 -->
    <el-table-column
      label="修改日期"
      width="160"
      :sortable="sortable"
      sort-by="modified_at"
    >
      <template #default="{ row }">
        <span class="file-date">{{ formatDate(row.modified_at) }}</span>
      </template>
    </el-table-column>

    <!-- 哈希值列（可选显示） -->
    <el-table-column
      label="哈希值"
      prop="hash"
      min-width="200"
      show-overflow-tooltip
      v-if="$slots.hash"
    >
      <template #default="{ row }">
        <slot name="hash" :row="row" />
      </template>
    </el-table-column>

    <!-- 路径列 -->
    <el-table-column
      label="路径"
      prop="path"
      min-width="250"
      show-overflow-tooltip
    />

    <!-- 操作列 -->
    <el-table-column
      v-if="showActions"
      label="操作"
      width="280"
      fixed="right"
    >
      <template #default="{ row }">
        <el-button-group class="action-buttons">
          <el-button
            size="small"
            type="success"
            @click="openPreview(row)"
            title="预览文件"
          >
            <el-icon><View /></el-icon>
          </el-button>
          <el-button
            size="small"
            type="primary"
            @click="openFile(row.path)"
            title="打开文件"
          >
            <el-icon><Document /></el-icon>
          </el-button>
          <el-button
            size="small"
            type="info"
            @click="openFolder(row.path)"
            title="打开所在文件夹"
          >
            <el-icon><FolderOpened /></el-icon>
          </el-button>
          <el-button
            size="small"
            type="danger"
            @click="handleDelete(row)"
            title="删除文件"
          >
            <el-icon><Delete /></el-icon>
          </el-button>
        </el-button-group>
      </template>
    </el-table-column>

    <!-- 自定义列插槽 -->
    <template v-for="(_, name) in $slots" :key="name" #[name]="slotData">
      <slot :name="name" v-bind="slotData" />
    </template>
  </el-table>

  <!-- 文件预览对话框 -->
  <FilePreview
    v-if="previewFile"
    v-model:visible="previewVisible"
    :file-path="previewFile.path"
    :file-name="previewFile.filename"
    :file-size="previewFile.size"
    @close="closePreview"
  />
</template>

<style scoped>
.file-size {
  font-weight: 500;
  color: #409EFF;
}

.file-date {
  color: #606266;
  font-size: 13px;
}

.action-buttons {
  display: flex;
  gap: 4px;
}

:deep(.el-button--small) {
  padding: 6px 10px;
}

:deep(.el-icon) {
  font-size: 14px;
}
</style>
