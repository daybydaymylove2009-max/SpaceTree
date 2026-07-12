<script setup lang="ts">
/**
 * 重复文件列表组件
 * @component DuplicateFileList
 * @description 配置驱动的重复文件分组列表，支持三种重复类型
 */

import type { DuplicateGroup, DuplicateType } from '../types'
import DuplicateFileTable from './DuplicateFileTable.vue'

/**
 * 组件属性定义
 */
interface Props {
  /** 重复文件组列表 */
  groups: DuplicateGroup[]
  /** 数据库路径 */
  dbPath: string
  /** 重复类型 */
  type: DuplicateType
  /** 空状态描述 */
  emptyDescription?: string
}

const props = withDefaults(defineProps<Props>(), {
  emptyDescription: '未发现重复文件'
})

/**
 * 组件事件定义
 */
interface Emits {
  /** 刷新列表 */
  (e: 'refresh'): void
}

const emit = defineEmits<Emits>()

/**
 * 生成折叠面板标题
 * @param group - 重复文件组
 * @param index - 索引
 * @returns 标题字符串
 */
function getGroupTitle(group: DuplicateGroup, index: number): string {
  const fileName = group.files[0]?.filename || '未知'
  return `${index + 1} - ${group.files.length} 个文件 (${fileName})`
}
</script>

<template>
  <el-empty v-if="groups.length === 0" :description="emptyDescription" />
  <el-collapse v-else>
    <el-collapse-item
      v-for="(group, index) in groups"
      :key="group.hash"
      :title="getGroupTitle(group, index)"
    >
      <DuplicateFileTable
        :files="group.files"
        :db-path="dbPath"
        @refresh="emit('refresh')"
      />
    </el-collapse-item>
  </el-collapse>
</template>
