<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  Folder, FolderOpened, Document, Search, Refresh,
  Delete, CopyDocument, View, Grid, List
} from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';

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

interface DirectoryNode {
  name: string;
  path: string;
  is_directory: boolean;
  size: number;
  file_count: number;
  children: DirectoryNode[];
}

const props = defineProps<{
  db_path: string;
}>()

// 显示模式: 'tree' = 目录树, 'list' = 文件列表
const displayMode = ref<'tree' | 'list'>('tree');

// 目录树数据
const treeData = ref<DirectoryNode[]>([]);
const treeLoading = ref(false);
const expandedKeys = ref<string[]>([]);

// 当前选中的目录
const selectedPath = ref('');
const selectedDirName = ref('');

// 文件列表（全部文件）
const allFiles = ref<FileInfo[]>([]);
const fileList = ref<FileInfo[]>([]);
const fileLoading = ref(false);
const fileTotal = ref(0);
const currentPage = ref(1);
const pageSize = ref(50);

// 搜索
const searchQuery = ref('');
const isSearching = ref(false);

// 文件详情对话框
const showFileDetail = ref(false);
const selectedFile = ref<FileInfo | null>(null);

// 加载目录树
async function loadDirectoryTree() {
  if (!props.db_path) {
    ElMessage.warning('数据库未初始化');
    return;
  }

  treeLoading.value = true;
  try {
    const result: DirectoryNode[] = await invoke('get_directory_tree', {
      db_path: props.db_path,
      root_path: null
    });
    treeData.value = result;
  } catch (error) {
    console.error('加载目录树失败:', error);
    ElMessage.error('加载目录树失败');
  } finally {
    treeLoading.value = false;
  }
}

// 加载所有文件
async function loadAllFiles() {
  if (!props.db_path) {
    ElMessage.warning('数据库未初始化');
    return;
  }

  fileLoading.value = true;
  try {
    // 从数据库获取所有文件
    const files: FileInfo[] = await invoke('get_all_files', {
      db_path: props.db_path
    });
    allFiles.value = files;
    fileTotal.value = files.length;
    updateFileList();
  } catch (error) {
    console.error('加载文件列表失败:', error);
    ElMessage.error('加载文件列表失败');
  } finally {
    fileLoading.value = false;
  }
}

// 更新分页后的文件列表
function updateFileList() {
  const start = (currentPage.value - 1) * pageSize.value;
  const end = start + pageSize.value;
  fileList.value = allFiles.value.slice(start, end);
}

// 处理节点点击
async function handleNodeClick(data: DirectoryNode) {
  if (!data.is_directory) return;

  selectedPath.value = data.path;
  selectedDirName.value = data.name;
}

// 搜索文件
async function searchFiles() {
  if (!searchQuery.value.trim()) {
    fileTotal.value = allFiles.value.length;
    updateFileList();
    return;
  }

  isSearching.value = true;
  try {
    const query = searchQuery.value.toLowerCase();
    const filtered = allFiles.value.filter(file =>
      file.filename.toLowerCase().includes(query) ||
      file.path.toLowerCase().includes(query)
    );
    fileTotal.value = filtered.length;
    const start = (currentPage.value - 1) * pageSize.value;
    const end = start + pageSize.value;
    fileList.value = filtered.slice(start, end);
  } finally {
    isSearching.value = false;
  }
}

// 处理分页变化
function handlePageChange(page: number) {
  currentPage.value = page;
  if (searchQuery.value) {
    searchFiles();
  } else {
    updateFileList();
  }
}

// 切换显示模式
function switchMode(mode: 'tree' | 'list') {
  displayMode.value = mode;
  if (mode === 'tree') {
    loadDirectoryTree();
  } else {
    loadAllFiles();
  }
}

// 查看文件详情
function viewFileDetail(file: FileInfo) {
  selectedFile.value = file;
  showFileDetail.value = true;
}

// 复制文件路径
async function copyFilePath(path: string) {
  try {
    await navigator.clipboard.writeText(path);
    ElMessage.success('路径已复制到剪贴板');
  } catch (error) {
    console.error('复制路径失败:', error);
    ElMessage.error('复制路径失败');
  }
}

// 打开文件所在目录
async function openFileLocation(path: string) {
  try {
    await invoke('show_in_folder', { path });
  } catch (error) {
    console.error('打开文件位置失败:', error);
    ElMessage.error('无法打开文件位置');
  }
}

// 删除文件
async function deleteFile(file: FileInfo) {
  try {
    await ElMessageBox.confirm(
      `确定要删除文件 "${file.filename}" 吗？`,
      '确认删除',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'warning'
      }
    );

    await invoke('delete_file', {
      path: file.path,
      db_path: props.db_path,
      allowed_roots: null
    });

    ElMessage.success('文件已删除');
    // 从列表中移除
    allFiles.value = allFiles.value.filter(f => f.path !== file.path);
    if (searchQuery.value) {
      searchFiles();
    } else {
      fileTotal.value = allFiles.value.length;
      updateFileList();
    }
  } catch (error: any) {
    if (error !== 'cancel') {
      console.error('删除文件失败:', error);
      ElMessage.error('删除文件失败');
    }
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

// 格式化日期
function formatDate(dateStr: string): string {
  if (!dateStr) return '未知';
  try {
    const date = new Date(dateStr);
    return date.toLocaleString('zh-CN');
  } catch {
    return dateStr;
  }
}

// 获取文件图标
function getFileIcon(extension: string): any {
  const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'svg'];
  const videoExts = ['mp4', 'avi', 'mov', 'mkv', 'flv'];
  const audioExts = ['mp3', 'wav', 'flac', 'aac', 'ogg'];
  const docExts = ['doc', 'docx', 'pdf', 'txt', 'md', 'xls', 'xlsx', 'ppt', 'pptx'];
  const archiveExts = ['zip', 'rar', '7z', 'tar', 'gz'];

  const ext = extension.toLowerCase();
  if (imageExts.includes(ext)) return 'Picture';
  if (videoExts.includes(ext)) return 'VideoCamera';
  if (audioExts.includes(ext)) return 'Headset';
  if (docExts.includes(ext)) return 'Document';
  if (archiveExts.includes(ext)) return 'Box';
  return 'Document';
}

onMounted(() => {
  loadDirectoryTree();
  loadAllFiles();
});

watch(() => props.db_path, () => {
  if (props.db_path) {
    loadDirectoryTree();
    loadAllFiles();
  }
});

// 选定目录下的文件数据
const folderPage = ref(1);
const folderPageSize = ref(20);

// 计算当前选定目录下（包含子目录）的所有文件
const currentFolderFiles = computed(() => {
  if (!selectedPath.value) {
    return allFiles.value; // 如果没选中目录，默认显示全部
  }
  const prefix = selectedPath.value.endsWith('\\') ? selectedPath.value : selectedPath.value + '\\';
  return allFiles.value.filter(file => file.path === selectedPath.value || file.path.startsWith(prefix));
});

// 分页截取后的当前目录下文件
const paginatedFolderFiles = computed(() => {
  const start = (folderPage.value - 1) * folderPageSize.value;
  const end = start + folderPageSize.value;
  return currentFolderFiles.value.slice(start, end);
});

// 监听选定目录变化，自动将分页重置为 1
watch(selectedPath, () => {
  folderPage.value = 1;
});
</script>

<template>
  <div class="directory-tree-container">
    <!-- 引入流光背景背景斑块 -->
    <div class="glow-spot glow-blue"></div>
    <div class="glow-spot glow-purple"></div>

    <!-- 页面头部 -->
    <div class="page-header">
      <div class="header-title">
        <h2>目录浏览</h2>
        <p class="header-subtitle">提供物理目录结构树与全盘平铺文件列表的交互式深度探查</p>
      </div>
      <div class="header-actions">
        <!-- 显示模式切换 -->
        <el-button :text="displayMode !== 'tree'" :type="displayMode === 'tree' ? 'primary' : 'default'" @click="switchMode('tree')" size="default">
          <el-icon><Grid /></el-icon>
          结构树视图
        </el-button>
        <el-button :text="displayMode !== 'list'" :type="displayMode === 'list' ? 'primary' : 'default'" @click="switchMode('list')" size="default">
          <el-icon><List /></el-icon>
          平铺列表视图
        </el-button>
        <el-button text @click="displayMode === 'tree' ? loadDirectoryTree() : loadAllFiles()" :loading="treeLoading || fileLoading">
          <el-icon><Refresh /></el-icon>
          刷新数据
        </el-button>
      </div>
    </div>

    <el-card class="main-card glass-card" shadow="never">

      <!-- 目录树与文件双栏分栏视图 -->
      <div v-if="displayMode === 'tree'" class="tree-layout-wrapper">
        <!-- 左侧：目录树 -->
        <div class="tree-sidebar-card">
          <el-tree
            :data="treeData"
            :props="{ label: 'name', children: 'children' }"
            :expand-on-click-node="false"
            :default-expanded-keys="expandedKeys"
            highlight-current
            @node-click="handleNodeClick"
            v-loading="treeLoading"
            empty-text="暂无文件索引"
            class="directory-tree"
          >
            <template #default="{ node, data }">
              <span class="tree-node" :class="{ 'is-directory': data.is_directory }">
                <el-icon v-if="data.is_directory" class="folder-icon">
                  <Folder v-if="!node.expanded" />
                  <FolderOpened v-else />
                </el-icon>
                <span class="node-label">{{ data.name }}</span>
                <span v-if="data.file_count > 0" class="file-count-tag">{{ data.file_count }}</span>
              </span>
            </template>
          </el-tree>
        </div>

        <!-- 右侧：选定目录下的文件 -->
        <div class="tree-files-panel">
          <div class="panel-header-desc">
            <el-icon :size="16" class="panel-folder-icon"><FolderOpened /></el-icon>
            <span class="path-title">当前浏览位置：<span class="path-highlight">{{ selectedPath || '所有已索引的物理根目录' }}</span></span>
            <span class="stats-badge" v-if="currentFolderFiles.length > 0">共包含 {{ currentFolderFiles.length }} 个文件</span>
          </div>

          <div class="files-table-wrapper">
            <el-table 
              :data="paginatedFolderFiles" 
              v-loading="fileLoading"
              style="width: 100%; height: 100%;"
              class="glass-table"
              stripe
            >
              <el-table-column prop="filename" label="文件名" min-width="180" show-overflow-tooltip>
                <template #default="{ row }">
                  <span class="file-name-cell">
                    <el-icon class="file-icon-doc"><Document /></el-icon>
                    <span>{{ row.filename }}</span>
                  </span>
                </template>
              </el-table-column>
              <el-table-column prop="size" label="大小" width="100">
                <template #default="{ row }">
                  <span class="size-text">{{ formatSize(row.size) }}</span>
                </template>
              </el-table-column>
              <el-table-column prop="modified_at" label="最后修改日期" width="160">
                <template #default="{ row }">
                  <span class="date-text">{{ formatDate(row.modified_at) }}</span>
                </template>
              </el-table-column>
              <el-table-column label="操作" width="160" fixed="right" align="right">
                <template #default="{ row }">
                  <div class="actions-group">
                    <el-button-group class="glass-btn-group">
                      <el-button size="small" @click="viewFileDetail(row)" title="详情">
                        <el-icon><View /></el-icon>
                      </el-button>
                      <el-button size="small" @click="openFileLocation(row.path)" title="定位所在文件夹">
                        <el-icon><FolderOpened /></el-icon>
                      </el-button>
                      <el-button size="small" type="danger" class="row-delete-btn" @click="deleteFile(row)" title="删除此文件">
                        <el-icon><Delete /></el-icon>
                      </el-button>
                    </el-button-group>
                  </div>
                </template>
              </el-table-column>
            </el-table>
          </div>

          <!-- 分页器 -->
          <div class="panel-pagination">
            <el-pagination
              v-model:current-page="folderPage"
              v-model:page-size="folderPageSize"
              :page-sizes="[10, 20, 50, 100]"
              :total="currentFolderFiles.length"
              layout="total, sizes, prev, pager, next"
              small
              class="glass-pagination"
              @size-change="folderPage = 1"
            />
          </div>
        </div>
      </div>

      <!-- 文件列表视图 -->
      <div v-else class="view-content">
        <!-- 搜索栏 -->
        <div class="search-bar">
          <el-input
            v-model="searchQuery"
            placeholder="在已索引的文件中搜索文件名或路径..."
            clearable
            class="glass-search-input"
            @keyup.enter="searchFiles"
          >
            <template #append>
              <el-button @click="searchFiles" :loading="isSearching" class="search-btn">
                <el-icon><Search /></el-icon>
              </el-button>
            </template>
          </el-input>
          <span class="file-stats">
            共匹配到 <strong class="highlight-count">{{ fileTotal }}</strong> 个文件
          </span>
        </div>

        <!-- 文件列表 -->
        <el-table
          :data="fileList"
          v-loading="fileLoading"
          style="width: 100%"
          height="calc(100vh - 300px)"
          class="glass-table"
          stripe
        >
          <el-table-column label="文件名" min-width="200" show-overflow-tooltip>
            <template #default="{ row }">
              <div class="file-name-cell">
                <div class="file-type-icon-wrapper">
                  <el-icon class="file-icon">
                    <component :is="getFileIcon(row.file_extension)" />
                  </el-icon>
                </div>
                <span class="file-name">{{ row.filename }}</span>
              </div>
            </template>
          </el-table-column>

          <el-table-column label="类型" width="90">
            <template #default="{ row }">
              <span class="ext-badge">{{ row.file_extension ? row.file_extension.toUpperCase() : '未知' }}</span>
            </template>
          </el-table-column>

          <el-table-column label="大小" width="100">
            <template #default="{ row }">
              <span class="size-text">{{ formatSize(row.size) }}</span>
            </template>
          </el-table-column>

          <el-table-column label="修改时间" width="150">
            <template #default="{ row }">
              <span class="date-text">{{ formatDate(row.modified_at) }}</span>
            </template>
          </el-table-column>

          <el-table-column label="哈希校验值" min-width="200" show-overflow-tooltip>
            <template #default="{ row }">
              <span class="hash-value">{{ row.hash || '-' }}</span>
            </template>
          </el-table-column>

          <el-table-column label="操作" width="180" fixed="right" align="right">
            <template #default="{ row }">
              <div class="actions-group">
                <el-button-group class="glass-btn-group">
                  <el-button size="small" @click="viewFileDetail(row)" title="查看详情">
                    <el-icon><View /></el-icon>
                  </el-button>
                  <el-button size="small" @click="copyFilePath(row.path)" title="复制物理路径">
                    <el-icon><CopyDocument /></el-icon>
                  </el-button>
                  <el-button size="small" @click="openFileLocation(row.path)" title="打开所在文件夹">
                    <el-icon><FolderOpened /></el-icon>
                  </el-button>
                  <el-button size="small" type="danger" class="row-delete-btn" @click="deleteFile(row)" title="删除">
                    <el-icon><Delete /></el-icon>
                  </el-button>
                </el-button-group>
              </div>
            </template>
          </el-table-column>
        </el-table>

        <!-- 分页 -->
        <div class="pagination-container">
          <el-pagination
            v-model:current-page="currentPage"
            v-model:page-size="pageSize"
            :page-sizes="[20, 50, 100, 200]"
            :total="fileTotal"
            layout="total, sizes, prev, pager, next"
            class="glass-pagination"
            @size-change="handlePageChange(1)"
            @current-change="handlePageChange"
          />
        </div>
      </div>
    </el-card>

    <!-- 文件详情对话框 -->
    <el-dialog
      v-model="showFileDetail"
      title="文件元数据信息"
      width="600px"
      class="glass-dialog"
    >
      <el-descriptions v-if="selectedFile" :column="1" border class="glass-descriptions">
        <el-descriptions-item label="文件名">{{ selectedFile.filename }}</el-descriptions-item>
        <el-descriptions-item label="物理完整路径"><span style="word-break: break-all; user-select: all;">{{ selectedFile.path }}</span></el-descriptions-item>
        <el-descriptions-item label="文件大小">{{ formatSize(selectedFile.size) }} ({{ selectedFile.size }} 字节)</el-descriptions-item>
        <el-descriptions-item label="扩展名">{{ selectedFile.file_extension ? selectedFile.file_extension.toUpperCase() : '无扩展名' }}</el-descriptions-item>
        <el-descriptions-item label="哈希校验特征值"><span style="word-break: break-all; user-select: all; font-family: monospace;">{{ selectedFile.hash || '未计算' }}</span></el-descriptions-item>
        <el-descriptions-item label="哈希算法">{{ selectedFile.hash_algorithm || '无' }}</el-descriptions-item>
        <el-descriptions-item label="数据库索引时间">{{ formatDate(selectedFile.created_at) }}</el-descriptions-item>
        <el-descriptions-item label="系统修改时间">{{ formatDate(selectedFile.modified_at) }}</el-descriptions-item>
      </el-descriptions>

      <template #footer>
        <el-button @click="showFileDetail = false" class="dialog-cancel-btn">关闭</el-button>
        <el-button type="primary" @click="selectedFile && openFileLocation(selectedFile.path)" class="dialog-ok-btn">
          定位物理位置
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.directory-tree-container {
  padding: 24px;
  height: 100%;
  position: relative;
}

/* 蓝紫背景斑块 */
.glow-spot {
  position: absolute;
  width: 300px;
  height: 300px;
  border-radius: 50%;
  pointer-events: none;
  filter: blur(100px);
  z-index: 1;
  opacity: 0.45;
}

.glow-blue {
  background: rgba(64, 158, 255, 0.12);
  top: 5%;
  right: 10%;
}

.glow-purple {
  background: rgba(155, 89, 182, 0.08);
  bottom: 5%;
  left: 10%;
}

.main-card {
  height: calc(100vh - 200px);
  position: relative;
  z-index: 2;
}

.glass-card {
  background: rgba(255, 255, 255, 0.65) !important;
  backdrop-filter: blur(20px) saturate(180%) !important;
  -webkit-backdrop-filter: blur(20px) saturate(180%) !important;
  border: 1px solid rgba(255, 255, 255, 0.45) !important;
  border-radius: 14px !important;
  box-shadow: 0 10px 30px rgba(31, 38, 135, 0.04) !important;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 24px;
}

.title {
  font-size: 16px;
  font-weight: bold;
  color: #303133;
  display: flex;
  align-items: center;
  gap: 8px;
}

.title-icon {
  color: #409EFF;
}

.glass-btn-group :deep(.el-button) {
  background: rgba(255, 255, 255, 0.5) !important;
  border-color: rgba(220, 223, 230, 0.4) !important;
  font-weight: 500;
  transition: all 0.2s;
}

.glass-btn-group :deep(.el-button.el-button--primary) {
  background: #409EFF !important;
  border-color: #409EFF !important;
  color: white !important;
}

.glass-btn-group :deep(.el-button:hover:not(.el-button--primary)) {
  color: #409EFF !important;
  background: #ffffff !important;
}

.glass-refresh-btn {
  background: rgba(64, 158, 255, 0.1) !important;
  border: 1px solid rgba(64, 158, 255, 0.2) !important;
  color: #409EFF !important;
}

.glass-refresh-btn:hover {
  background: #409EFF !important;
  border-color: #409EFF !important;
  color: white !important;
}

/* 双栏目录浏览器布局 */
.tree-layout-wrapper {
  display: flex;
  gap: 20px;
  height: calc(100% - 20px);
  overflow: hidden;
  padding: 4px 0;
}

.tree-sidebar-card {
  width: 300px;
  flex-shrink: 0;
  border-right: 1px solid rgba(220, 223, 230, 0.3);
  padding-right: 16px;
  overflow-y: auto;
}

.tree-files-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.panel-header-desc {
  display: flex;
  align-items: center;
  margin-bottom: 16px;
  padding: 10px 16px;
  background: linear-gradient(135deg, rgba(64, 158, 255, 0.08) 0%, rgba(155, 89, 182, 0.03) 100%);
  border-radius: 8px;
  border: 1px solid rgba(64, 158, 255, 0.18);
  box-shadow: 0 4px 15px rgba(64, 158, 255, 0.01);
}

.panel-folder-icon {
  color: #409EFF;
  margin-right: 8px;
}

.path-title {
  font-size: 13px;
  font-weight: 500;
  color: #606266;
}

.path-highlight {
  font-weight: 600;
  color: #409EFF;
}

.stats-badge {
  margin-left: auto;
  font-size: 11px;
  font-weight: 600;
  color: #9b59b6;
  background: rgba(155, 89, 182, 0.08);
  border: 1px solid rgba(155, 89, 182, 0.15);
  padding: 2px 8px;
  border-radius: 10px;
}

.files-table-wrapper {
  flex: 1;
  overflow: hidden;
  margin-bottom: 12px;
  background: rgba(255, 255, 255, 0.3);
  border-radius: 10px;
  border: 1px solid rgba(220, 223, 230, 0.2);
}

.panel-pagination {
  display: flex;
  justify-content: flex-end;
}

.view-content {
  height: calc(100% - 20px);
  display: flex;
  flex-direction: column;
}

/* Tree 美化 */
.directory-tree {
  background: transparent;
}

.directory-tree :deep(.el-tree-node__content) {
  height: 38px;
  border-radius: 6px;
  margin-bottom: 2px;
  transition: all 0.2s ease;
}

.directory-tree :deep(.el-tree-node__content:hover) {
  background: rgba(64, 158, 255, 0.08) !important;
  color: #409EFF;
}

.directory-tree :deep(.el-tree-node.is-current > .el-tree-node__content) {
  background: linear-gradient(90deg, rgba(64, 158, 255, 0.14) 0%, rgba(102, 177, 255, 0.02) 100%) !important;
  color: #409EFF;
  font-weight: bold;
  border-left: 3px solid #409EFF;
}

.tree-node {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  width: 100%;
}

.tree-node.is-directory {
  font-weight: 500;
}

.folder-icon {
  color: #e6a23c;
  font-size: 16px;
}

.file-icon {
  color: #909399;
}

.node-label {
  flex: 1;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-count-tag {
  color: #909399;
  font-size: 11px;
  background: rgba(220, 223, 230, 0.4);
  padding: 1px 6px;
  border-radius: 8px;
}

/* 列表模式下的搜索栏 */
.search-bar {
  margin-bottom: 18px;
  display: flex;
  gap: 15px;
  align-items: center;
  padding: 14px;
  background: rgba(255, 255, 255, 0.45);
  border-radius: 10px;
  border: 1px solid rgba(220, 223, 230, 0.3);
}

.glass-search-input {
  flex: 1;
  max-width: 460px;
}

.glass-search-input :deep(.el-input__wrapper) {
  background: rgba(255, 255, 255, 0.6);
  border: 1px solid rgba(220, 223, 230, 0.4);
  border-radius: 8px 0 0 8px;
  box-shadow: none !important;
}

.glass-search-input :deep(.el-input-group__append) {
  background: #409EFF;
  border: 1px solid #409EFF;
  border-radius: 0 8px 8px 0;
  color: white;
}

.file-stats {
  color: #606266;
  font-size: 13px;
}

.highlight-count {
  color: #409EFF;
  font-size: 15px;
}

.file-name-cell {
  display: flex;
  align-items: center;
  gap: 10px;
}

.file-type-icon-wrapper {
  width: 28px;
  height: 28px;
  background: rgba(144, 147, 153, 0.12);
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.file-icon-doc {
  color: #909399;
}

.file-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  color: #303133;
}

.ext-badge {
  font-size: 10px;
  font-weight: bold;
  background: rgba(144, 147, 153, 0.12);
  color: #606266;
  padding: 2px 6px;
  border-radius: 4px;
}

.size-text {
  font-size: 12px;
  color: #606266;
  font-weight: 500;
}

.date-text {
  font-size: 12px;
  color: #909399;
}

.pagination-container {
  margin-top: 15px;
  display: flex;
  justify-content: center;
}

.hash-value {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 11px;
  color: #909399;
}

.actions-group {
  display: flex;
  align-items: center;
  justify-content: flex-end;
}

.row-delete-btn:hover {
  background: #f56c6c !important;
  color: white !important;
  border-color: #f56c6c !important;
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

.glass-pagination :deep(.el-pager li) {
  background: rgba(255, 255, 255, 0.5) !important;
  border: 1px solid rgba(220, 223, 230, 0.4) !important;
}

.glass-pagination :deep(.el-pager li.is-active) {
  background: #409EFF !important;
  color: white !important;
}

/* 弹窗设计 */
.glass-dialog :deep(.el-dialog) {
  background: rgba(255, 255, 255, 0.8) !important;
  backdrop-filter: blur(25px) saturate(180%) !important;
  -webkit-backdrop-filter: blur(25px) saturate(180%) !important;
  border: 1px solid rgba(255, 255, 255, 0.5) !important;
  border-radius: 16px !important;
  box-shadow: 0 20px 50px rgba(0,0,0,0.08) !important;
}

.glass-descriptions :deep(.el-descriptions__body) {
  background: transparent !important;
}

.glass-descriptions :deep(.el-descriptions-item__cell) {
  background: rgba(255, 255, 255, 0.3) !important;
}

.dialog-cancel-btn {
  background: rgba(255, 255, 255, 0.6) !important;
  border-color: rgba(220, 223, 230, 0.5) !important;
}

.dialog-ok-btn {
  background: linear-gradient(135deg, #409EFF 0%, #66b1ff 100%) !important;
  border: none !important;
  color: white !important;
}

/* 页面头部参照扫描中心进行设计与优化 */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.header-title h2 {
  margin: 0 0 4px 0;
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}

.header-subtitle {
  color: #909399;
  font-size: 14px;
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}
</style>
