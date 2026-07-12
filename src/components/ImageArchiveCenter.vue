<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  Search, Folder, Box, FolderOpened, CopyDocument, Picture, Cpu, Files,
  Delete, Link, Connection, Grid
} from '@element-plus/icons-vue';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import { t, getLanguage } from '../utils/i18n';

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

interface SimilarImageGroup {
  files: FileInfo[];
  wasted_space: number;
  hash_values: string[];
}

const props = defineProps<{
  db_path: string;
}>()

// 强制组件刷新 key，配合手写语言包达成无刷新秒切
const forceUpdateKey = ref(0);
const onLanguageChange = () => {
  forceUpdateKey.value++;
};

// 页面 Tab 切换
const activeTab = ref('archive');

// ================== 图片打包逻辑 ==================
const minImageCount = ref(10);
const selectedExtensions = ref(['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp']);
const availableExtensions = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'svg', 'ico', 'tiff'];

const report = ref<ImageArchiveReport | null>(null);
const isAnalyzing = ref(false);

const compressionLevel = ref(6);
const archiveFormat = ref('zip');
const deleteAfterArchive = ref(false); 
const selectedDirectories = ref<string[]>([]);

async function analyzeDirectories() {
  if (!props.db_path) {
    ElMessage.warning(t('zh-CN' === getLanguage() ? '数据库未初始化' : 'Database not initialized'));
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
    ElMessage.success(t('zh-CN' === getLanguage() 
      ? `找到 ${result.total_directories} 个图片密集目录，共 ${result.total_images} 张图片` 
      : `Found ${result.total_directories} directories, ${result.total_images} images total`));
  } catch (error) {
    console.error('分析失败:', error);
    ElMessage.error(t('zh-CN' === getLanguage() ? '分析图片目录失败' : 'Failed to analyze image directories'));
  } finally {
    isAnalyzing.value = false;
  }
}

async function createArchive(directoryPath: string) {
  try {
    const dirName = getDirectoryName(directoryPath);
    const defaultFileName = `${dirName}_${new Date().toISOString().slice(0, 10)}.zip`;
    
    const outputPath = await save({
      filters: [
        { name: 'ZIP 压缩文件', extensions: ['zip'] }
      ],
      defaultPath: defaultFileName
    });

    if (!outputPath) return;

    ElMessage.info(t('zh-CN' === getLanguage() ? '正在创建压缩文件，请稍候...' : 'Creating archive, please wait...'));

    const result: ArchiveResult = await invoke('create_archive', {
      db_path: props.db_path,
      source_directory: directoryPath,
      output_path: outputPath,
      archive_format: archiveFormat.value,
      compression_level: compressionLevel.value
    });

    ElMessage.success(t('zh-CN' === getLanguage() 
      ? `压缩完成！压缩率: ${result.compression_ratio.toFixed(1)}%` 
      : `Archive complete! Ratio: ${result.compression_ratio.toFixed(1)}%`));

    if (deleteAfterArchive.value) {
      try {
        await ElMessageBox.confirm(
          t('zh-CN' === getLanguage() 
            ? `压缩完成！是否删除原图片文件？\n\n压缩文件: ${result.archive_path}\n此操作不可恢复！` 
            : `Archive done! Delete source images?\n\nPath: ${result.archive_path}\nThis action is permanent!`),
          t('zh-CN' === getLanguage() ? '确认删除原文件' : 'Confirm deletion'),
          {
            confirmButtonText: t('common.confirm'),
            cancelButtonText: t('common.cancel'),
            type: 'warning'
          }
        );

        await invoke('delete_files_after_archive', {
          db_path: props.db_path,
          directory_path: directoryPath,
          image_extensions: selectedExtensions.value
        });

        ElMessage.success(t('common.success'));
        await analyzeDirectories();
      } catch (deleteError) {
        if (deleteError !== 'cancel') {
          console.error('删除失败:', deleteError);
        }
      }
    }

    await ElMessageBox.confirm(
      t('zh-CN' === getLanguage() 
        ? `压缩文件已创建: ${result.archive_path}\n文件数: ${result.file_count}\n大小: ${formatSize(result.total_size)}` 
        : `Archive created: ${result.archive_path}\nFiles: ${result.file_count}\nSize: ${formatSize(result.total_size)}`),
      t('zh-CN' === getLanguage() ? '压缩完成' : 'Archive Completed'),
      {
        confirmButtonText: t('zh-CN' === getLanguage() ? '打开所在目录' : 'Show in Folder'),
        cancelButtonText: t('common.cancel'),
        type: 'success'
      }
    ).then(() => {
      invoke('show_in_folder', { path: outputPath });
    }).catch(() => {});

  } catch (error) {
    console.error('打包失败:', error);
    ElMessage.error(t('common.error') + ': ' + error);
  }
}

async function batchArchive() {
  if (selectedDirectories.value.length === 0) return;

  try {
    await ElMessageBox.confirm(
      t('zh-CN' === getLanguage() ? `确定要压缩选中的 ${selectedDirectories.value.length} 个目录吗？` : `Archive ${selectedDirectories.value.length} directories?`),
      t('zh-CN' === getLanguage() ? '确认批量压缩' : 'Batch Archive'),
      {
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
        type: 'warning'
      }
    );

    const timestamp = new Date().toISOString().slice(0, 10);
    const baseOutputPath = await save({
      filters: [
        { name: 'ZIP 压缩文件', extensions: ['zip'] }
      ],
      defaultPath: `批量图片压缩_${timestamp}.zip`
    });

    if (!baseOutputPath) return;

    let successCount = 0;
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
        console.error(error);
      }
    }
    ElMessage.success(t('zh-CN' === getLanguage() ? `批量压缩完成！成功: ${successCount} 个` : `Batch complete! Success: ${successCount}`));
    selectedDirectories.value = [];
  } catch (e) {
    //
  }
}

// ================== 相似图片查重逻辑 ==================
const simConfig = ref({
  algorithm: 'dhash',
  threshold: 10
});
const similarGroups = ref<SimilarImageGroup[]>([]);
const isScanningSimilar = ref(false);

const activeCompareGroup = ref<SimilarImageGroup | null>(null);
const compareDialogVisible = ref(false);

async function scanSimilarImages() {
  if (!props.db_path) {
    ElMessage.warning(t('zh-CN' === getLanguage() ? '数据库未初始化' : 'Database not initialized'));
    return;
  }

  isScanningSimilar.value = true;
  try {
    const result = await invoke('find_similar_images', {
      db_path: props.db_path,
      config: {
        algorithm: simConfig.value.algorithm,
        threshold: simConfig.value.threshold
      },
      allowed_roots: null
    }) as SimilarImageGroup[];

    similarGroups.value = result;
    ElMessage.success(t('zh-CN' === getLanguage() 
      ? `扫描完成！共检索出 ${result.length} 组相似图片` 
      : `Scan complete! Found ${result.length} groups of similar images`));
  } catch (error) {
    console.error('检索相似图片失败:', error);
    ElMessage.error(t('zh-CN' === getLanguage() ? '相似图片扫描失败' : 'Failed to scan similar images') + ': ' + error);
  } finally {
    isScanningSimilar.value = false;
  }
}

function openCompare(group: SimilarImageGroup) {
  activeCompareGroup.value = group;
  compareDialogVisible.value = true;
}

// 物理删除单个相似图
async function deleteSimilarFile(path: string) {
  try {
    await ElMessageBox.confirm(
      t('zh-CN' === getLanguage() ? `确定要物理删除此图片吗？\n"${path}"` : `Permanently delete this image?\n"${path}"`),
      t('zh-CN' === getLanguage() ? '物理删除' : 'Delete File'),
      {
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
        type: 'warning'
      }
    );

    await invoke('delete_file', {
      path,
      db_path: props.db_path,
      allowed_roots: null
    });
    ElMessage.success(t('common.success'));
    
    // 从当前对比视图和列表中移除该文件
    if (activeCompareGroup.value) {
      activeCompareGroup.value.files = activeCompareGroup.value.files.filter(f => f.path !== path);
      if (activeCompareGroup.value.files.length <= 1) {
        compareDialogVisible.value = false;
        similarGroups.value = similarGroups.value.filter(g => g !== activeCompareGroup.value);
      }
    }
  } catch (e) {
    //
  }
}

// 硬链接转换去重
async function hardlinkSimilarFiles(keep: FileInfo, replace: FileInfo) {
  try {
    await ElMessageBox.confirm(
      t('zh-CN' === getLanguage() 
        ? `确定将 "${replace.filename}" 物理替换为指向 "${keep.filename}" 的物理硬链接吗？这能释放空间且对系统完全透明。` 
        : `Replace "${replace.filename}" with a hardlink to "${keep.filename}"?`),
      t('zh-CN' === getLanguage() ? '硬链接去重' : 'Hardlink Convert'),
      {
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
        type: 'warning'
      }
    );

    const count = await invoke('replace_files_with_hardlinks', {
      keep_path: keep.path,
      replace_paths: [replace.path],
      db_path: props.db_path,
      allowed_roots: null
    }) as number;

    if (count > 0) {
      ElMessage.success(t('zh-CN' === getLanguage() ? '硬链接无损去重成功' : 'Hardlink conversion succeeded'));
      compareDialogVisible.value = false;
      await scanSimilarImages();
    }
  } catch (error) {
    console.error(error);
    ElMessage.error(t('zh-CN' === getLanguage() ? '硬链接转换失败 (可能跨分区)' : 'Hardlink conversion failed (cannot cross volumes)') + ': ' + error);
  }
}

// 辅助方法
async function copyPath(path: string) {
  try {
    await navigator.clipboard.writeText(path);
    ElMessage.success(t('common.copySuccess'));
  } catch (e) {
    //
  }
}

async function openDirectory(path: string) {
  try {
    await invoke('show_in_folder', { path });
  } catch (e) {
    //
  }
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function getDirectoryName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

function getParentDirectory(path: string): string {
  const parts = path.split(/[\\/]/);
  parts.pop();
  return parts.join('/') || path;
}

onMounted(() => {
  window.addEventListener('app-lang-change', onLanguageChange);
  if (props.db_path) {
    analyzeDirectories();
    scanSimilarImages();
  }
});

onUnmounted(() => {
  window.removeEventListener('app-lang-change', onLanguageChange);
});
</script>

<template>
  <div class="image-archive-center" :key="forceUpdateKey">
    <div class="glow-spot glow-blue"></div>
    <div class="glow-spot glow-purple"></div>

    <div class="page-header">
      <div class="header-title">
        <h2>{{ t('zh-CN' === getLanguage() ? '图像管理与相似去重' : 'Image Manager & Similarity') }}</h2>
        <p class="header-subtitle">{{ t('zh-CN' === getLanguage() ? '密集图片打包归档与 BK-Tree 度量空间相似图高速查重工作台' : 'Image dense packager & BK-Tree measure space similarity de-duplication') }}</p>
      </div>
    </div>

    <!-- 顶部选项卡 -->
    <div class="tabs-header-bar">
      <div 
        class="tab-item" 
        :class="{ active: activeTab === 'archive' }" 
        @click="activeTab = 'archive'"
      >
        <el-icon><Box /></el-icon>
        <span>{{ t('zh-CN' === getLanguage() ? '密集图片打包' : 'Image Packager') }}</span>
      </div>
      <div 
        class="tab-item" 
        :class="{ active: activeTab === 'similarity' }" 
        @click="activeTab = 'similarity'"
      >
        <el-icon><Picture /></el-icon>
        <span>{{ t('zh-CN' === getLanguage() ? '相似图片查重' : 'Similar Images') }}</span>
      </div>
    </div>

    <!-- TAB A: 密集图片打包 -->
    <div v-show="activeTab === 'archive'">
      <el-card class="config-card glass-card" shadow="never">
        <template #header>
          <div class="card-header">
            <span class="card-title">
              <el-icon class="title-icon"><Cpu /></el-icon>
              {{ t('zh-CN' === getLanguage() ? '分析参数配置' : 'Parameters') }}
            </span>
            <el-button type="primary" @click="analyzeDirectories" :loading="isAnalyzing" class="glass-submit-btn">
              <el-icon><Search /></el-icon>
              {{ t('zh-CN' === getLanguage() ? '开始扫描分析' : 'Scan') }}
            </el-button>
          </div>
        </template>

        <el-row :gutter="24">
          <el-col :span="8" :xs="24">
            <el-form-item :label="t('zh-CN' === getLanguage() ? '最小图片数量阈值' : 'Min image count')">
              <el-input-number v-model="minImageCount" :min="2" :max="1000" style="width: 100%" class="glass-number-input" />
            </el-form-item>
          </el-col>
          <el-col :span="8" :xs="24">
            <el-form-item :label="t('zh-CN' === getLanguage() ? 'ZIP 压缩等级' : 'ZIP Compression level')">
              <el-slider v-model="compressionLevel" :min="1" :max="9" show-stops class="glass-slider" />
            </el-form-item>
          </el-col>
          <el-col :span="8" :xs="24">
            <el-form-item :label="t('zh-CN' === getLanguage() ? '目标图片后缀过滤' : 'Image extensions')">
              <el-select v-model="selectedExtensions" multiple collapse-tags style="width: 100%" class="glass-select">
                <el-option v-for="ext in availableExtensions" :key="ext" :label="ext.toUpperCase()" :value="ext" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>

        <div class="danger-warning-block">
          <el-alert type="warning" :closable="false" show-icon :title="t('zh-CN' === getLanguage() ? '防误删安全警告' : 'Safety Warning')">
            <div class="warning-checkbox-wrapper">
              <el-checkbox v-model="deleteAfterArchive" class="danger-checkbox">
                <span class="warning-highlight-text">{{ t('zh-CN' === getLanguage() ? '打包完成后自动彻底删除原始图片（高风险）' : 'Auto-delete original images after archive (high risk)') }}</span>
              </el-checkbox>
            </div>
          </el-alert>
        </div>
      </el-card>

      <!-- 结果列表 -->
      <el-card v-if="report && report.directories.length > 0" class="result-card glass-card" shadow="never">
        <template #header>
          <div class="card-header">
            <span class="card-title">
              <el-icon class="title-icon"><Files /></el-icon>
              {{ t('zh-CN' === getLanguage() ? '密集图像目录清单' : 'Dense directories list') }}
            </span>
            <el-button type="success" @click="batchArchive" :disabled="selectedDirectories.length === 0" class="glass-batch-btn">
              <el-icon><Box /></el-icon>
              {{ t('zh-CN' === getLanguage() ? `批量执行压缩 (已选 ${selectedDirectories.length} 处)` : `Batch Archive (${selectedDirectories.length} selected)`) }}
            </el-button>
          </div>
        </template>

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
                <div class="folder-icon-wrapper"><el-icon class="folder-icon"><Folder /></el-icon></div>
                <div class="directory-details">
                  <div class="directory-name" :title="row.directory_path">{{ getDirectoryName(row.directory_path) }}</div>
                  <div class="directory-path" :title="row.directory_path">{{ getParentDirectory(row.directory_path) }}</div>
                </div>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="图片数量" width="130">
            <template #default="{ row }">
              <span class="image-count-badge"><el-icon><Picture /></el-icon> {{ row.image_count }} 张</span>
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
                  <el-button size="small" @click="openDirectory(row.directory_path)"><el-icon><FolderOpened /></el-icon></el-button>
                  <el-button size="small" @click="copyPath(row.directory_path)"><el-icon><CopyDocument /></el-icon></el-button>
                </el-button-group>
                <el-button size="small" type="primary" class="row-zip-btn" @click="createArchive(row.directory_path)">
                  <el-icon><Box /></el-icon>
                  <span>{{ t('zh-CN' === getLanguage() ? '打包' : 'Archive') }}</span>
                </el-button>
              </div>
            </template>
          </el-table-column>
        </el-table>
      </el-card>

      <el-card v-else class="empty-result-card glass-card" shadow="never">
        <el-empty :description="t('zh-CN' === getLanguage() ? '未检索到密集的图片目录' : 'No dense image directory found')" />
      </el-card>
    </div>

    <!-- TAB B: 相似图片查重 -->
    <div v-show="activeTab === 'similarity'">
      <el-card class="config-card glass-card" shadow="never">
        <template #header>
          <div class="card-header">
            <span class="card-title">
              <el-icon class="title-icon"><Connection /></el-icon>
              {{ t('zh-CN' === getLanguage() ? '图像算法特征参数' : 'BK-Tree Space parameters') }}
            </span>
            <el-button type="primary" @click="scanSimilarImages" :loading="isScanningSimilar" class="glass-submit-btn">
              <el-icon><Search /></el-icon>
              {{ t('zh-CN' === getLanguage() ? '开始相似图片扫描' : 'Scan Similar') }}
            </el-button>
          </div>
        </template>

        <el-row :gutter="24">
          <el-col :span="12" :xs="24">
            <el-form-item :label="t('zh-CN' === getLanguage() ? '相似检索算法' : 'Fingerprint Algorithm')">
              <el-select v-model="simConfig.algorithm" style="width: 100%" class="glass-select">
                <el-option label="dHash (基于横纵灰度差分 - 高度推荐)" value="dhash" />
                <el-option label="pHash (基于离散余弦变换 DCT - 精确度极高)" value="phash" />
                <el-option label="aHash (基于平均灰度比对 - 速度极快)" value="ahash" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12" :xs="24">
            <el-form-item :label="t('zh-CN' === getLanguage() ? '相似度阈值 (汉明距离，值越小相似度越高)' : 'Similarity distance (Hamming)')">
              <el-slider v-model="simConfig.threshold" :min="1" :max="25" show-stops class="glass-slider" />
              <div class="form-hint">{{ t('zh-CN' === getLanguage() ? '建议范围: 5 ~ 12，距离过大可能引入无关图片' : 'Suggested distance: 5 to 12') }}</div>
            </el-form-item>
          </el-col>
        </el-row>
      </el-card>

      <!-- 相似组网格展示 -->
      <div v-if="similarGroups.length > 0" class="similar-groups-container">
        <el-row :gutter="16">
          <el-col :span="12" :xs="24" v-for="(group, idx) in similarGroups" :key="idx" style="margin-bottom: 16px;">
            <div class="glass-card similar-group-card">
              <div class="group-card-header">
                <span class="group-tag">🎯 {{ t('zh-CN' === getLanguage() ? `相似组 #${idx+1}` : `Group #${idx+1}`) }}</span>
                <span class="wasted-badge">{{ formatSize(group.wasted_space) }}</span>
              </div>
              <div class="group-images-row">
                <div 
                  v-for="img in group.files.slice(0, 3)" 
                  :key="img.path"
                  class="group-image-cell"
                >
                  <img :src="convertFileSrc(img.path)" class="group-cell-img" />
                  <div class="cell-info-box">
                    <span class="cell-filename" :title="img.filename">{{ img.filename }}</span>
                    <span class="cell-size">{{ formatSize(img.size) }}</span>
                  </div>
                </div>
              </div>
              <div class="group-actions-footer">
                <el-button type="primary" size="small" plain :icon="Grid" @click="openCompare(group)">
                  {{ t('zh-CN' === getLanguage() ? '对比预览与去重' : 'Compare & Clean') }}
                </el-button>
              </div>
            </div>
          </el-col>
        </el-row>
      </div>

      <el-card v-else class="empty-result-card glass-card" shadow="never">
        <el-empty :description="t('zh-CN' === getLanguage() ? '没有查找到任何相似图片' : 'No similar images found')" />
      </el-card>
    </div>

    <!-- 左右图片对比大弹窗 -->
    <el-dialog 
      v-model="compareDialogVisible" 
      width="80%" 
      align-center 
      class="glass-dialog"
      :title="t('zh-CN' === getLanguage() ? '🔍 相似图片高清大图比对去重工作台' : 'Image Compare Workbench')"
    >
      <div class="compare-workbench-body" v-if="activeCompareGroup && activeCompareGroup.files.length >= 2">
        <div class="compare-side-card">
          <div class="side-card-header">
            <span class="badge primary">IMAGE A</span>
            <el-button type="danger" size="small" :icon="Delete" @click="deleteSimilarFile(activeCompareGroup.files[0].path)">
              {{ t('zh-CN' === getLanguage() ? '物理删除 A' : 'Delete A') }}
            </el-button>
          </div>
          <div class="compare-img-box">
            <img :src="convertFileSrc(activeCompareGroup.files[0].path)" class="compare-img" />
          </div>
          <div class="compare-info-list">
            <p><strong>{{ t('zh-CN' === getLanguage() ? '路径' : 'Path') }}:</strong> {{ activeCompareGroup.files[0].path }}</p>
            <p><strong>{{ t('zh-CN' === getLanguage() ? '大小' : 'Size') }}:</strong> {{ formatSize(activeCompareGroup.files[0].size) }}</p>
          </div>
        </div>

        <div class="compare-center-divider">
          <div class="vs-text">VS</div>
          <el-button 
            type="warning" 
            size="default" 
            :icon="Link" 
            @click="hardlinkSimilarFiles(activeCompareGroup.files[0], activeCompareGroup.files[1])"
            class="vs-action-btn"
          >
            {{ t('zh-CN' === getLanguage() ? '将 B 替换为 A 的硬链接' : 'Replace B with Hardlink of A') }}
          </el-button>
        </div>

        <div class="compare-side-card">
          <div class="side-card-header">
            <span class="badge success">IMAGE B</span>
            <el-button type="danger" size="small" :icon="Delete" @click="deleteSimilarFile(activeCompareGroup.files[1].path)">
              {{ t('zh-CN' === getLanguage() ? '物理删除 B' : 'Delete B') }}
            </el-button>
          </div>
          <div class="compare-img-box">
            <img :src="convertFileSrc(activeCompareGroup.files[1].path)" class="compare-img" />
          </div>
          <div class="compare-info-list">
            <p><strong>{{ t('zh-CN' === getLanguage() ? '路径' : 'Path') }}:</strong> {{ activeCompareGroup.files[1].path }}</p>
            <p><strong>{{ t('zh-CN' === getLanguage() ? '大小' : 'Size') }}:</strong> {{ formatSize(activeCompareGroup.files[1].size) }}</p>
          </div>
        </div>
      </div>
    </el-dialog>
  </div>
</template>

<style scoped>
.image-archive-center {
  padding: 24px;
  position: relative;
  min-height: 100%;
}

.glow-spot {
  position: absolute;
  width: 300px;
  height: 300px;
  border-radius: 50%;
  pointer-events: none;
  filter: blur(100px);
  z-index: 1;
  opacity: 0.3;
}

.glow-blue {
  background: rgba(64, 158, 255, 0.12);
  top: 10%;
  right: 5%;
}

.glow-purple {
  background: rgba(155, 89, 182, 0.08);
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
  color: var(--el-text-color-primary);
}

.header-subtitle {
  margin: 0;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

/* 现代玻璃选项卡 */
.tabs-header-bar {
  display: flex;
  gap: 16px;
  margin-bottom: 24px;
  border-bottom: 1px solid var(--el-border-color-light);
  padding-bottom: 8px;
  z-index: 2;
  position: relative;
}

.tab-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  cursor: pointer;
  border-radius: 8px;
  color: var(--el-text-color-regular);
  transition: all 0.2s ease;
}

.tab-item:hover {
  background: var(--el-fill-color-light);
  color: var(--el-color-primary);
}

.tab-item.active {
  background: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
  font-weight: 600;
}

.glass-card {
  background: var(--el-bg-color-overlay) !important;
  backdrop-filter: blur(20px) saturate(180%) !important;
  -webkit-backdrop-filter: blur(20px) saturate(180%) !important;
  border: 1px solid var(--el-border-color-light) !important;
  border-radius: 14px !important;
  box-shadow: 0 10px 30px rgba(31, 38, 135, 0.02) !important;
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
  color: var(--el-text-color-primary);
}

.title-icon {
  color: var(--el-color-primary);
}

.glass-submit-btn {
  background: linear-gradient(135deg, var(--el-color-primary) 0%, var(--el-color-primary-light-3) 100%) !important;
  border: none !important;
  color: white !important;
  font-weight: 500;
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.15);
  transition: all 0.3s ease;
}

.glass-submit-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(64, 158, 255, 0.25);
}

.form-hint {
  font-size: 11px;
  color: var(--el-text-color-placeholder);
  margin-top: 4px;
  line-height: 1.4;
  text-align: left;
}

.danger-warning-block {
  margin-top: 20px;
}

.warning-checkbox-wrapper {
  margin-top: 10px;
  text-align: left;
}

.danger-checkbox {
  height: auto;
}

.warning-highlight-text {
  color: var(--el-color-danger);
  font-weight: 600;
  font-size: 12px;
}

/* 相似图片卡片 */
.similar-group-card {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.group-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.group-tag {
  font-weight: 600;
  font-size: 13px;
  color: var(--el-text-color-primary);
}

.wasted-badge {
  font-size: 11px;
  background: var(--el-color-danger-light-9);
  color: var(--el-color-danger);
  padding: 2px 8px;
  border-radius: 10px;
  font-weight: 600;
}

.group-images-row {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}

.group-image-cell {
  background: var(--el-fill-color-blank);
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.group-cell-img {
  width: 100%;
  height: 90px;
  object-fit: cover;
  background: #f5f7fa;
}

.cell-info-box {
  padding: 6px;
  width: 100%;
  display: flex;
  flex-direction: column;
  text-align: left;
  box-sizing: border-box;
}

.cell-filename {
  font-size: 11px;
  font-weight: 500;
  color: var(--el-text-color-regular);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cell-size {
  font-size: 10px;
  color: var(--el-text-color-placeholder);
  margin-top: 2px;
}

.group-actions-footer {
  display: flex;
  justify-content: flex-end;
  margin-top: 6px;
}

/* 对比工作台样式 */
.compare-workbench-body {
  display: flex;
  gap: 20px;
  align-items: stretch;
  justify-content: space-between;
}

.compare-side-card {
  flex: 1;
  border: 1px solid var(--el-border-color-light);
  border-radius: 12px;
  background: var(--el-bg-color-overlay);
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.side-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.badge {
  font-size: 11px;
  font-weight: 700;
  padding: 3px 8px;
  border-radius: 4px;
}

.badge.primary {
  background: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
}

.badge.success {
  background: var(--el-color-success-light-9);
  color: var(--el-color-success);
}

.compare-img-box {
  width: 100%;
  height: 250px;
  background: #f5f7fa;
  border-radius: 8px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--el-border-color-lighter);
}

.compare-img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.compare-info-list {
  text-align: left;
  font-size: 12px;
  color: var(--el-text-color-regular);
}

.compare-info-list p {
  margin: 4px 0;
  word-break: break-all;
}

.compare-center-divider {
  width: 140px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 20px;
  flex-shrink: 0;
}

.vs-text {
  font-size: 24px;
  font-weight: 800;
  color: var(--el-text-color-placeholder);
  letter-spacing: 2px;
}

.vs-action-btn {
  width: 100%;
  white-space: normal;
  height: auto;
  padding: 10px 8px;
  font-weight: 600;
}

/* 目录列表 */
.directory-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.folder-icon-wrapper {
  width: 32px;
  height: 32px;
  background: var(--el-color-warning-light-9);
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.folder-icon {
  font-size: 18px;
  color: var(--el-color-warning);
}

.directory-details {
  min-width: 0;
  flex: 1;
  text-align: left;
}

.directory-name {
  font-weight: 600;
  font-size: 13px;
  color: var(--el-text-color-primary);
  text-overflow: ellipsis;
  overflow: hidden;
  white-space: nowrap;
}

.directory-path {
  font-size: 11px;
  color: var(--el-text-color-placeholder);
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
  color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
  padding: 3px 8px;
  border-radius: 6px;
}

.size-text {
  font-size: 12px;
  color: var(--el-text-color-regular);
  font-weight: 500;
}

.glass-batch-btn {
  background: linear-gradient(135deg, var(--el-color-success) 0%, var(--el-color-success-light-3) 100%) !important;
  border: none !important;
  color: white !important;
  box-shadow: 0 4px 12px rgba(103, 194, 58, 0.15);
}

.glass-btn-group :deep(.el-button) {
  background: var(--el-bg-color-overlay) !important;
  border-color: var(--el-border-color-light) !important;
  padding: 6px 10px;
}

.glass-btn-group :deep(.el-button:hover) {
  color: var(--el-color-primary) !important;
  background: var(--el-fill-color-light) !important;
}

.actions-group {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

.row-zip-btn {
  background: var(--el-color-primary-light-9) !important;
  border-color: var(--el-color-primary-light-7) !important;
  color: var(--el-color-primary) !important;
}

.row-zip-btn:hover {
  background: var(--el-color-primary) !important;
  border-color: var(--el-color-primary) !important;
  color: white !important;
}

.glass-table {
  background: transparent !important;
}

.glass-table :deep(tr) {
  background: transparent !important;
}

.glass-table :deep(th.el-table__cell) {
  background: var(--el-fill-color-light) !important;
  color: var(--el-text-color-regular);
  font-weight: 600;
}

@media (max-width: 900px) {
  .compare-workbench-body {
    flex-direction: column;
    gap: 16px;
  }
  .compare-center-divider {
    width: 100%;
    flex-direction: row;
    height: auto;
  }
}
</style>
