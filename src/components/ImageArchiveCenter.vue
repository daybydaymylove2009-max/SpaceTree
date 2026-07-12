<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  Search, Folder, Box, FolderOpened, CopyDocument, Picture, Cpu, Files
} from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';

interface FileInfo {
  id: number;
  path: string;
  filename: string;
  size: number;
  hash: string;
  hash_algorithm: string;
  created_at: string;
  modified_at: string;
  file_extension: string;
}

interface ImageDirectoryInfo {
  directory_path: string;
  image_count: number;
  total_size: number;
  image_files: FileInfo[];
}

interface ImageArchiveReport {
  directories: ImageDirectoryInfo[];
  total_directories: number;
  total_images: number;
  total_size: number;
}

interface ArchiveResult {
  archive_path: string;
  file_count: number;
  total_size: number;
  compression_ratio: number;
}

const props = defineProps<{
  db_path: string;
}>()

// 分析参数
const minImageCount = ref(10);
const selectedExtensions = ref(['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp']);
const availableExtensions = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'svg', 'ico', 'tiff'];

// 分析结果
const report = ref<ImageArchiveReport | null>(null);
const isAnalyzing = ref(false);

// 压缩参数
const compressionLevel = ref(6);
const archiveFormat = ref('zip');
const deleteAfterArchive = ref(false); // 打包后删除原文件

// 选中的目录
const selectedDirectories = ref<string[]>([]);

// 分析图片密集目录
async function analyzeDirectories() {
  if (!props.db_path) {
    ElMessage.warning('数据库未初始化');
    return;
  }

  isAnalyzing.value = true;
  try {
    const result: ImageArchiveReport = await invoke('analyze_image_directories', {
      db_path: props.db_path,
      min_image_count: minImageCount.value,
      image_extensions: selectedExtensions.value
    });

    report.value = result;
    ElMessage.success(`找到 ${result.total_directories} 个图片密集目录，共 ${result.total_images} 张图片`);
  } catch (error) {
    console.error('分析失败:', error);
    ElMessage.error('分析图片目录失败');
  } finally {
    isAnalyzing.value = false;
  }
}

// 创建压缩文件
async function createArchive(directoryPath: string) {
  try {
    // 获取目录名称作为默认文件名
    const dirName = getDirectoryName(directoryPath);
    const defaultFileName = `${dirName}_${new Date().toISOString().slice(0, 10)}.zip`;
    
    // 选择保存位置
    const outputPath = await save({
      filters: [
        { name: 'ZIP 压缩文件', extensions: ['zip'] }
      ],
      defaultPath: defaultFileName
    });

    if (!outputPath) {
      return;
    }

    ElMessage.info('正在创建压缩文件，请稍候...');

    const result: ArchiveResult = await invoke('create_archive', {
      db_path: props.db_path,
      source_directory: directoryPath,
      output_path: outputPath,
      archive_format: archiveFormat.value,
      compression_level: compressionLevel.value
    });

    ElMessage.success(`压缩完成！压缩率: ${result.compression_ratio.toFixed(1)}%`);

    // 询问是否删除原文件
    if (deleteAfterArchive.value) {
      try {
        await ElMessageBox.confirm(
          `压缩完成！是否删除原图片文件？\n\n` +
          `压缩文件: ${result.archive_path}\n` +
          `原文件数: ${result.file_count}\n` +
          `此操作不可恢复！`,
          '确认删除原文件',
          {
            confirmButtonText: '删除原文件',
            cancelButtonText: '保留原文件',
            type: 'warning'
          }
        );

        // 删除原文件
        await invoke('delete_files_after_archive', {
          db_path: props.db_path,
          directory_path: directoryPath,
          image_extensions: selectedExtensions.value
        });

        ElMessage.success('原文件已删除');
        // 刷新分析结果
        await analyzeDirectories();
      } catch (deleteError: any) {
        if (deleteError !== 'cancel') {
          console.error('删除原文件失败:', deleteError);
          ElMessage.error('删除原文件失败');
        }
      }
    }

    // 显示结果
    await ElMessageBox.confirm(
      `压缩文件已创建: ${result.archive_path}\n\n` +
      `文件数: ${result.file_count}\n` +
      `原始大小: ${formatSize(result.total_size)}\n` +
      `压缩率: ${result.compression_ratio.toFixed(1)}%`,
      '压缩完成',
      {
        confirmButtonText: '打开所在目录',
        cancelButtonText: '关闭',
        type: 'success'
      }
    ).then(() => {
      invoke('show_in_folder', { path: outputPath });
    }).catch(() => {});

  } catch (error) {
    console.error('创建压缩文件失败:', error);
    ElMessage.error('创建压缩文件失败: ' + error);
  }
}

// 批量压缩选中的目录
async function batchArchive() {
  if (selectedDirectories.value.length === 0) {
    ElMessage.warning('请先选择要压缩的目录');
    return;
  }

  try {
    await ElMessageBox.confirm(
      `确定要压缩选中的 ${selectedDirectories.value.length} 个目录吗？`,
      '确认批量压缩',
      {
        confirmButtonText: '开始压缩',
        cancelButtonText: '取消',
        type: 'warning'
      }
    );

    // 选择保存目录
    const timestamp = new Date().toISOString().slice(0, 10);
    const baseOutputPath = await save({
      filters: [
        { name: 'ZIP 压缩文件', extensions: ['zip'] }
      ],
      defaultPath: `批量图片压缩_${timestamp}.zip`
    });

    if (!baseOutputPath) {
      return;
    }

    let successCount = 0;
    let failCount = 0;

    for (const dirPath of selectedDirectories.value) {
      try {
        const dirName = dirPath.split(/[\\/]/).pop() || 'archive';
        const outputPath = baseOutputPath.replace('.zip', `_${dirName}.zip`);

        await invoke('create_archive', {
          db_path: props.db_path,
          source_directory: dirPath,
          output_path: outputPath,
          archive_format: archiveFormat.value,
          compression_level: compressionLevel.value
        });

        successCount++;
      } catch (error) {
        console.error(`压缩目录失败 ${dirPath}:`, error);
        failCount++;
      }
    }

    ElMessage.success(`批量压缩完成！成功: ${successCount}, 失败: ${failCount}`);
    selectedDirectories.value = [];

  } catch (error: any) {
    if (error !== 'cancel') {
      console.error('批量压缩失败:', error);
      ElMessage.error('批量压缩失败');
    }
  }
}

// 复制路径
async function copyPath(path: string) {
  try {
    await navigator.clipboard.writeText(path);
    ElMessage.success('路径已复制到剪贴板');
  } catch (error) {
    console.error('复制路径失败:', error);
    ElMessage.error('复制路径失败');
  }
}

// 打开目录
async function openDirectory(path: string) {
  try {
    await invoke('show_in_folder', { path });
  } catch (error) {
    console.error('打开目录失败:', error);
    ElMessage.error('无法打开目录');
  }
}

// 格式化文件大小
function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// 获取目录名称
function getDirectoryName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

// 获取父目录路径
function getParentDirectory(path: string): string {
  const parts = path.split(/[\\/]/);
  parts.pop();
  return parts.join('/') || path;
}

onMounted(() => {
  if (props.db_path) {
    analyzeDirectories();
  }
});
</script>

<template>
  <div class="image-archive-center">
    <!-- 引入流光背景背景斑块 -->
    <div class="glow-spot glow-blue"></div>
    <div class="glow-spot glow-purple"></div>

    <div class="page-header">
      <div class="header-title">
        <h2>图片打包中心</h2>
        <p class="header-subtitle">多维度智能分析并批量打包盘符中高度密集的图片文件，释放海量冗余空间</p>
      </div>
    </div>

    <!-- 分析参数 -->
    <el-card class="config-card glass-card" shadow="never">
      <template #header>
        <div class="card-header">
          <span class="card-title">
            <el-icon class="title-icon"><Cpu /></el-icon>分析参数配置
          </span>
          <el-button type="primary" @click="analyzeDirectories" :loading="isAnalyzing" class="glass-submit-btn">
            <el-icon><Search /></el-icon>
            开始扫描分析
          </el-button>
        </div>
      </template>

      <el-row :gutter="24">
        <el-col :span="8" :xs="24">
          <el-form-item label="最小图片数量阈值">
            <el-input-number v-model="minImageCount" :min="2" :max="1000" style="width: 100%" class="glass-number-input" />
            <div class="form-hint">仅当目录内包含的图片总数不少于该值时才会被捕获</div>
          </el-form-item>
        </el-col>

        <el-col :span="8" :xs="24">
          <el-form-item label="ZIP 压缩等级">
            <el-slider v-model="compressionLevel" :min="1" :max="9" show-stops class="glass-slider" />
            <div class="form-hint">1 = 极速打包，9 = 极致压缩比（需消耗更多 CPU）</div>
          </el-form-item>
        </el-col>

        <el-col :span="8" :xs="24">
          <el-form-item label="目标图片后缀过滤">
            <el-select v-model="selectedExtensions" multiple collapse-tags style="width: 100%" class="glass-select">
              <el-option
                v-for="ext in availableExtensions"
                :key="ext"
                :label="ext.toUpperCase()"
                :value="ext"
              />
            </el-select>
            <div class="form-hint">选择要纳入打包范围的图像格式</div>
          </el-form-item>
        </el-col>
      </el-row>

      <div class="danger-warning-block">
        <el-alert
          type="warning"
          :closable="false"
          show-icon
          title="防误删安全警告"
          description="启用“打包后删除原文件”可能会导致数据永久擦除，请确保生成的压缩包已妥善同步或备份！"
        >
          <div class="warning-checkbox-wrapper">
            <el-checkbox v-model="deleteAfterArchive" class="danger-checkbox">
              <span class="warning-highlight-text">打包完成后自动彻底删除原始图片（高风险）</span>
            </el-checkbox>
          </div>
        </el-alert>
      </div>
    </el-card>

    <!-- 分析结果 -->
    <el-card v-if="report && report.directories.length > 0" class="result-card glass-card" shadow="never">
      <template #header>
        <div class="card-header">
          <span class="card-title">
            <el-icon class="title-icon"><Files /></el-icon>密集图像目录清单
          </span>
          <div>
            <el-button
              type="success"
              @click="batchArchive"
              :disabled="selectedDirectories.length === 0"
              class="glass-batch-btn"
            >
              <el-icon><Box /></el-icon>
              批量执行压缩 (已选 {{ selectedDirectories.length }} 处)
            </el-button>
          </div>
        </div>
      </template>

      <!-- 高科技感统计面板网格 -->
      <div class="stats-grid">
        <div class="stat-card stat-blue">
          <div class="stat-icon-wrapper">
            <el-icon :size="24"><Folder /></el-icon>
          </div>
          <div class="stat-details">
            <span class="stat-lbl">密集目录总数</span>
            <span class="stat-val">{{ report.total_directories }} <small>处</small></span>
          </div>
          <div class="card-light"></div>
        </div>

        <div class="stat-card stat-purple">
          <div class="stat-icon-wrapper">
            <el-icon :size="24"><Picture /></el-icon>
          </div>
          <div class="stat-details">
            <span class="stat-lbl">图像文件总数</span>
            <span class="stat-val">{{ report.total_images }} <small>张</small></span>
          </div>
          <div class="card-light"></div>
        </div>

        <div class="stat-card stat-green">
          <div class="stat-icon-wrapper">
            <el-icon :size="24"><Cpu /></el-icon>
          </div>
          <div class="stat-details">
            <span class="stat-lbl">预估总存储体积</span>
            <span class="stat-val">{{ formatSize(report.total_size) }}</span>
          </div>
          <div class="card-light"></div>
        </div>
      </div>

      <!-- 目录列表表格 -->
      <el-table
        :data="report.directories"
        style="width: 100%"
        class="glass-table"
        @selection-change="(selection: ImageDirectoryInfo[]) => selectedDirectories = selection.map(d => d.directory_path)"
      >
        <el-table-column type="selection" width="55" />

        <el-table-column label="密集目录物理路径" min-width="320">
          <template #default="{ row }">
            <div class="directory-info">
              <div class="folder-icon-wrapper">
                <el-icon class="folder-icon"><Folder /></el-icon>
              </div>
              <div class="directory-details">
                <div class="directory-name" :title="row.directory_path">
                  {{ getDirectoryName(row.directory_path) }}
                </div>
                <div class="directory-path" :title="row.directory_path">
                  {{ getParentDirectory(row.directory_path) }}
                </div>
              </div>
            </div>
          </template>
        </el-table-column>

        <el-table-column label="图片数量" width="130">
          <template #default="{ row }">
            <span class="image-count-badge">
              <el-icon><Picture /></el-icon>
              {{ row.image_count }} 张
            </span>
          </template>
        </el-table-column>

        <el-table-column label="目录体积" width="130">
          <template #default="{ row }">
            <span class="size-text">{{ formatSize(row.total_size) }}</span>
          </template>
        </el-table-column>

        <el-table-column label="操作" width="220" fixed="right" align="right">
          <template #default="{ row }">
            <div class="actions-group">
              <el-button-group class="glass-btn-group">
                <el-button size="small" @click="openDirectory(row.directory_path)" title="打开目录">
                  <el-icon><FolderOpened /></el-icon>
                </el-button>
                <el-button size="small" @click="copyPath(row.directory_path)" title="复制路径">
                  <el-icon><CopyDocument /></el-icon>
                </el-button>
              </el-button-group>
              <el-button size="small" type="primary" class="row-zip-btn" @click="createArchive(row.directory_path)" title="一键打包">
                <el-icon><Box /></el-icon>
                <span>打包</span>
              </el-button>
            </div>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 无数据提示 -->
    <el-card v-else-if="report && report.directories.length === 0" class="empty-result-card glass-card" shadow="never">
      <el-empty description="未检索到任何密集的图片目录">
        <template #image>
          <el-icon :size="64" color="rgba(64,158,255,0.25)"><Picture /></el-icon>
        </template>
        <p style="color: #909399; font-size: 13px; max-width: 320px; margin: 0 auto 16px;">
          当前设定的“最小图片数量”可能过高，或者您的磁盘数据库中未建立相应后缀名的图片索引。
        </p>
        <el-button type="primary" plain size="small" @click="minImageCount = 5">
          降低过滤阈值 (设为 5 张)
        </el-button>
      </el-empty>
    </el-card>
  </div>
</template>

<style scoped>
.image-archive-center {
  padding: 24px;
  position: relative;
  min-height: 100%;
}

/* 蓝紫侧边栏光斑 */
.glow-spot {
  position: absolute;
  width: 300px;
  height: 300px;
  border-radius: 50%;
  pointer-events: none;
  filter: blur(100px);
  z-index: 1;
  opacity: 0.5;
}

.glow-blue {
  background: rgba(64, 158, 255, 0.15);
  top: 10%;
  right: 5%;
}

.glow-purple {
  background: rgba(155, 89, 182, 0.1);
  bottom: 10%;
  left: 5%;
}

.page-header {
  margin-bottom: 24px;
  position: relative;
  z-index: 2;
}

.page-header h2 {
  margin: 0 0 6px 0;
  font-size: 22px;
  font-weight: 650;
  background: linear-gradient(120deg, #303133, #606266);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.header-subtitle {
  margin: 0;
  color: #8492a6;
  font-size: 13px;
}

.glass-card {
  background: rgba(255, 255, 255, 0.65) !important;
  backdrop-filter: blur(20px) saturate(180%) !important;
  -webkit-backdrop-filter: blur(20px) saturate(180%) !important;
  border: 1px solid rgba(255, 255, 255, 0.45) !important;
  border-radius: 14px !important;
  box-shadow: 0 10px 30px rgba(31, 38, 135, 0.04) !important;
  margin-bottom: 24px;
  position: relative;
  z-index: 2;
  overflow: hidden;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: #303133;
}

.title-icon {
  color: #409EFF;
}

.glass-submit-btn {
  background: linear-gradient(135deg, #409EFF 0%, #66b1ff 100%) !important;
  border: none !important;
  color: white !important;
  font-weight: 500;
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.2);
  transition: all 0.3s ease;
}

.glass-submit-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(64, 158, 255, 0.3);
}

.form-hint {
  font-size: 11px;
  color: #909399;
  margin-top: 4px;
  line-height: 1.4;
}

/* 安全警告区域 */
.danger-warning-block {
  margin-top: 20px;
}

.danger-warning-block :deep(.el-alert) {
  background: rgba(253, 246, 236, 0.6) !important;
  backdrop-filter: blur(8px);
  border: 1px solid rgba(230, 162, 60, 0.3);
  border-radius: 8px;
  padding: 12px 16px;
}

.warning-checkbox-wrapper {
  margin-top: 10px;
}

.danger-checkbox {
  height: auto;
}

.warning-highlight-text {
  color: #f56c6c;
  font-weight: 600;
  font-size: 12px;
}

/* 统计卡片网格 */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  margin-bottom: 24px;
}

.stat-card {
  position: relative;
  border-radius: 12px;
  padding: 16px 20px;
  display: flex;
  align-items: center;
  gap: 16px;
  border: 1px solid rgba(255, 255, 255, 0.5);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.01);
  overflow: hidden;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(31, 38, 135, 0.06);
}

.stat-blue {
  background: linear-gradient(135deg, rgba(64, 158, 255, 0.08) 0%, rgba(64, 158, 255, 0.02) 100%);
}

.stat-blue:hover {
  border-color: rgba(64, 158, 255, 0.3);
}

.stat-blue .stat-icon-wrapper {
  background: rgba(64, 158, 255, 0.15);
  color: #409EFF;
}

.stat-purple {
  background: linear-gradient(135deg, rgba(155, 89, 182, 0.08) 0%, rgba(155, 89, 182, 0.02) 100%);
}

.stat-purple:hover {
  border-color: rgba(155, 89, 182, 0.3);
}

.stat-purple .stat-icon-wrapper {
  background: rgba(155, 89, 182, 0.15);
  color: #9b59b6;
}

.stat-green {
  background: linear-gradient(135deg, rgba(103, 194, 58, 0.08) 0%, rgba(103, 194, 58, 0.02) 100%);
}

.stat-green:hover {
  border-color: rgba(103, 194, 58, 0.3);
}

.stat-green .stat-icon-wrapper {
  background: rgba(103, 194, 58, 0.15);
  color: #67C23A;
}

.stat-icon-wrapper {
  width: 46px;
  height: 46px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.03);
}

.stat-details {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.stat-lbl {
  font-size: 11px;
  color: #909399;
  font-weight: 500;
}

.stat-val {
  font-size: 18px;
  font-weight: 700;
  color: #303133;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.stat-val small {
  font-size: 12px;
  font-weight: 500;
  color: #606266;
}

/* 流光微光效果 */
.card-light {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: linear-gradient(135deg, transparent 40%, rgba(255,255,255,0.4) 50%, transparent 60%);
  transform: translateX(-100%);
  transition: transform 0.5s ease;
  pointer-events: none;
}

.stat-card:hover .card-light {
  transform: translateX(100%);
}

/* 目录展示元素 */
.directory-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.folder-icon-wrapper {
  width: 32px;
  height: 32px;
  background: rgba(230, 162, 60, 0.12);
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.folder-icon {
  font-size: 18px;
  color: #e6a23c;
}

.directory-details {
  min-width: 0;
  flex: 1;
}

.directory-name {
  font-weight: 600;
  font-size: 13px;
  color: #303133;
  text-overflow: ellipsis;
  overflow: hidden;
  white-space: nowrap;
}

.directory-path {
  font-size: 11px;
  color: #909399;
  text-overflow: ellipsis;
  overflow: hidden;
  white-space: nowrap;
  margin-top: 2px;
}

.image-count-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  font-weight: 600;
  color: #409EFF;
  background: rgba(64, 158, 255, 0.1);
  padding: 3px 8px;
  border-radius: 6px;
}

.size-text {
  font-size: 12px;
  color: #606266;
  font-weight: 500;
}

/* 按钮美化 */
.glass-batch-btn {
  background: linear-gradient(135deg, #67C23A 0%, #85ce61 100%) !important;
  border: none !important;
  color: white !important;
  box-shadow: 0 4px 12px rgba(103, 194, 58, 0.2);
}

.glass-batch-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(103, 194, 58, 0.3);
}

.glass-btn-group :deep(.el-button) {
  background: rgba(255, 255, 255, 0.5) !important;
  border-color: rgba(220, 223, 230, 0.4) !important;
  padding: 6px 10px;
}

.glass-btn-group :deep(.el-button:hover) {
  color: #409EFF !important;
  background: #ffffff !important;
}

.actions-group {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

.row-zip-btn {
  background: rgba(64, 158, 255, 0.1) !important;
  border-color: rgba(64, 158, 255, 0.2) !important;
  color: #409EFF !important;
}

.row-zip-btn:hover {
  background: #409EFF !important;
  border-color: #409EFF !important;
  color: white !important;
}

/* 表格覆盖 */
.glass-table {
  background: transparent !important;
}

.glass-table :deep(tr) {
  background: transparent !important;
}

.glass-table :deep(th.el-table__cell) {
  background: rgba(245, 247, 250, 0.3) !important;
  color: #909399;
  font-weight: 600;
  border-bottom: 1px solid rgba(220, 223, 230, 0.3);
}

.glass-table :deep(td.el-table__cell) {
  border-bottom: 1px solid rgba(220, 223, 230, 0.2);
}

@media (max-width: 768px) {
  .stats-grid {
    grid-template-columns: 1fr;
  }
}
</style>
