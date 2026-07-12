<script setup lang="ts">
/**
 * 文件预览组件
 * @component FilePreview
 * @description 支持图片、视频、文本文件的预览
 */

import { ref, computed, onMounted } from 'vue'
import { Close, ZoomIn, ZoomOut, Download } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { formatSize } from '../utils/formatters'

interface Props {
  filePath: string
  fileName: string
  fileSize: number
  visible: boolean
}

const props = defineProps<Props>()

interface Emits {
  (e: 'update:visible', value: boolean): void
  (e: 'close'): void
  (e: 'download', path: string): void
}

const emit = defineEmits<Emits>()

// 预览类型
const previewType = ref<'image' | 'video' | 'text' | 'unknown'>('unknown')

// 图片缩放
const imageScale = ref(1)

// 文本内容
const textContent = ref('')

// 是否加载中
const isLoading = ref(false)

// 文件扩展名
const fileExtension = computed(() => {
  const lastDot = props.fileName.lastIndexOf('.')
  return lastDot === -1 ? '' : props.fileName.substring(lastDot + 1).toLowerCase()
})

// 判断预览类型
function detectPreviewType() {
  const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'svg']
  const videoExts = ['mp4', 'avi', 'mov', 'wmv', 'flv', 'mkv']
  const textExts = ['txt', 'md', 'json', 'xml', 'html', 'css', 'js', 'ts', 'vue', 'rs', 'toml']

  if (imageExts.includes(fileExtension.value)) {
    previewType.value = 'image'
  } else if (videoExts.includes(fileExtension.value)) {
    previewType.value = 'video'
  } else if (textExts.includes(fileExtension.value)) {
    previewType.value = 'text'
    loadTextContent()
  } else {
    previewType.value = 'unknown'
  }
}

// 加载文本内容
async function loadTextContent() {
  if (props.fileSize > 1024 * 1024) {
    textContent.value = '文件过大，不支持预览'
    return
  }

  isLoading.value = true
  try {
    const content = await invoke('read_file_content', { path: props.filePath }) as string
    // 限制显示前 5000 字符
    textContent.value = content.length > 5000 
      ? content.substring(0, 5000) + '\n\n... (内容已截断)' 
      : content
  } catch (error) {
    textContent.value = `加载失败: ${error}`
  } finally {
    isLoading.value = false
  }
}

// 缩放图片
function zoomIn() {
  imageScale.value = Math.min(imageScale.value + 0.25, 3)
}

function zoomOut() {
  imageScale.value = Math.max(imageScale.value - 0.25, 0.5)
}

function resetZoom() {
  imageScale.value = 1
}

// 下载文件
function downloadFile() {
  emit('download', props.filePath)
}

// 关闭预览
function closePreview() {
  emit('close')
  resetZoom()
}

onMounted(() => {
  if (props.visible) {
    detectPreviewType()
  }
})
</script>

<template>
  <el-dialog
    :model-value="visible"
    @update:model-value="(val: boolean) => emit('update:visible', val)"
    :title="fileName"
    width="80%"
    top="5vh"
    :close-on-click-modal="false"
    @close="closePreview"
    class="file-preview-dialog"
  >
    <template #header>
      <div class="preview-header">
        <div class="file-info">
          <span class="file-name">{{ fileName }}</span>
          <span class="file-size">{{ formatSize(fileSize) }}</span>
        </div>
        <div class="preview-actions">
          <template v-if="previewType === 'image'">
            <el-button text circle @click="zoomOut" title="缩小">
              <el-icon><ZoomOut /></el-icon>
            </el-button>
            <span class="zoom-level">{{ Math.round(imageScale * 100) }}%</span>
            <el-button text circle @click="zoomIn" title="放大">
              <el-icon><ZoomIn /></el-icon>
            </el-button>
          </template>
          <el-button text circle @click="downloadFile" title="下载">
            <el-icon><Download /></el-icon>
          </el-button>
          <el-button text circle @click="closePreview" title="关闭">
            <el-icon><Close /></el-icon>
          </el-button>
        </div>
      </div>
    </template>

    <div class="preview-content" v-loading="isLoading">
      <!-- 图片预览 -->
      <div v-if="previewType === 'image'" class="image-preview">
        <img
          :src="'file://' + filePath"
          :style="{ transform: `scale(${imageScale})` }"
          @click="resetZoom"
        />
      </div>

      <!-- 视频预览 -->
      <div v-else-if="previewType === 'video'" class="video-preview">
        <video controls :src="'file://' + filePath" style="max-width: 100%; max-height: 70vh;">
          您的浏览器不支持视频播放
        </video>
      </div>

      <!-- 文本预览 -->
      <div v-else-if="previewType === 'text'" class="text-preview">
        <pre>{{ textContent }}</pre>
      </div>

      <!-- 不支持预览 -->
      <div v-else class="unknown-preview">
        <el-empty description="该文件类型暂不支持预览">
          <el-button type="primary" @click="downloadFile">下载文件</el-button>
        </el-empty>
      </div>
    </div>
  </el-dialog>
</template>

<style scoped>
.preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.file-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.file-name {
  font-weight: 600;
  font-size: 16px;
}

.file-size {
  color: #909399;
  font-size: 14px;
}

.preview-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.zoom-level {
  font-size: 14px;
  color: #606266;
  min-width: 50px;
  text-align: center;
}

.preview-content {
  min-height: 400px;
  max-height: 70vh;
  overflow: auto;
  display: flex;
  align-items: center;
  justify-content: center;
}

.image-preview {
  overflow: auto;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.image-preview img {
  max-width: 100%;
  max-height: 70vh;
  transition: transform 0.2s ease;
  cursor: zoom-out;
}

.video-preview {
  width: 100%;
  display: flex;
  justify-content: center;
}

.text-preview {
  width: 100%;
  height: 100%;
  overflow: auto;
}

.text-preview pre {
  margin: 0;
  padding: 16px;
  background: #f5f7fa;
  border-radius: 4px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-wrap: break-word;
  max-height: 70vh;
  overflow: auto;
}

.unknown-preview {
  padding: 40px;
}
</style>
