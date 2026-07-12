<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { 
  Delete, Folder, Refresh, 
  Document, Search, 
  InfoFilled, Brush, Download, List
} from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';

interface EmptyFile {
  path: string;
  filename: string;
  created_at: string;
  modified_at: string;
}

interface EmptyFileScanResult {
  empty_files: EmptyFile[];
  total_count: number;
  scan_time_ms: number;
}

interface DuplicateFolder {
  hash: string;
  folders: string[];
  file_count: number;
  total_size: number;
}

interface RecycleBinItem {
  id: string;
  original_path: string;
  deleted_at: string;
  size: number;
  file_name: string;
}

const props = defineProps<{
  db_path: string;
}>()

// 当前激活的工具
const activeTool = ref('empty-files');

// 0字节文件
const emptyFiles = ref<EmptyFile[]>([]);
const isScanningEmpty = ref(false);
const selectedEmptyFiles = ref<Set<string>>(new Set());

// 重复文件夹
const duplicateFolders = ref<DuplicateFolder[]>([]);
const isScanningFolders = ref(false);

// 回收站
const recycleBinItems = ref<RecycleBinItem[]>([]);
const isLoadingRecycleBin = ref(false);

// 审计日志
const auditLogs = ref<any[]>([]);
const isLoadingAuditLogs = ref(false);
const auditLogStats = ref<any>(null);

// 扫描0字节文件
async function scanEmptyFiles() {
  if (!props.db_path) {
    ElMessage.warning('请先进行文件扫描');
    return;
  }

  isScanningEmpty.value = true;
  try {
    const result = await invoke('scan_empty_files', { db_path: props.db_path }) as EmptyFileScanResult;
    emptyFiles.value = result.empty_files;
    ElMessage.success(`发现 ${result.total_count} 个0字节文件`);
  } catch (error) {
    ElMessage.error(`扫描失败: ${error}`);
  } finally {
    isScanningEmpty.value = false;
  }
}

// 删除选中的0字节文件
async function deleteSelectedEmptyFiles() {
  if (selectedEmptyFiles.value.size === 0) {
    ElMessage.warning('请先选择要删除的文件');
    return;
  }

  try {
    await ElMessageBox.confirm(
      `确定要删除选中的 ${selectedEmptyFiles.value.size} 个0字节文件吗？`,
      '确认删除',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'warning'
      }
    );

    const filesToDelete = Array.from(selectedEmptyFiles.value);
    await invoke('delete_empty_files', {
      db_path: props.db_path,
      paths: filesToDelete
    });
    ElMessage.success('删除成功');
    selectedEmptyFiles.value.clear();
    await scanEmptyFiles(); // 刷新列表
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`删除失败: ${error}`);
    }
  }
}

// 查找重复文件夹
async function findDuplicateFolders() {
  if (!props.db_path) {
    ElMessage.warning('请先进行文件扫描');
    return;
  }

  isScanningFolders.value = true;
  try {
    const result = await invoke('find_duplicate_folders', { db_path: props.db_path }) as DuplicateFolder[];
    duplicateFolders.value = result;
    ElMessage.success(`发现 ${result.length} 组重复文件夹`);
  } catch (error) {
    ElMessage.error(`扫描失败: ${error}`);
  } finally {
    isScanningFolders.value = false;
  }
}

// 获取回收站的物理路径（同数据库在同一个目录下）
function getRecycleBinPath(dbPath: string): string {
  const lastSlash = Math.max(dbPath.lastIndexOf('/'), dbPath.lastIndexOf('\\'));
  if (lastSlash === -1) return 'recycle_bin';
  return dbPath.substring(0, lastSlash) + '/recycle_bin';
}

// 加载回收站
async function loadRecycleBin() {
  if (!props.db_path) {
    ElMessage.warning('请先进行文件扫描');
    return;
  }
  isLoadingRecycleBin.value = true;
  try {
    const result = await invoke('list_recycle_bin', { db_path: props.db_path }) as any;
    recycleBinItems.value = result.items || [];
  } catch (error) {
    ElMessage.error(`加载失败: ${error}`);
  } finally {
    isLoadingRecycleBin.value = false;
  }
}

// 恢复文件
async function restoreFile(id: string) {
  if (!props.db_path) {
    ElMessage.warning('请先进行文件扫描');
    return;
  }
  try {
    await invoke('restore_from_recycle_bin', { 
      id, 
      db_path: props.db_path,
      recycle_bin_path: getRecycleBinPath(props.db_path)
    });
    ElMessage.success('文件已恢复');
    await loadRecycleBin();
  } catch (error) {
    ElMessage.error(`恢复失败: ${error}`);
  }
}

// 清空回收站
async function emptyRecycleBin() {
  if (!props.db_path) {
    ElMessage.warning('请先进行文件扫描');
    return;
  }
  if (recycleBinItems.value.length === 0) {
    ElMessage.warning('回收站为空');
    return;
  }

  try {
    await ElMessageBox.confirm(
      '确定要清空回收站吗？此操作不可恢复！',
      '清空回收站',
      {
        confirmButtonText: '清空',
        cancelButtonText: '取消',
        type: 'error'
      }
    );

    await invoke('empty_recycle_bin', {
      db_path: props.db_path,
      recycle_bin_path: getRecycleBinPath(props.db_path)
    });
    ElMessage.success('回收站已清空');
    await loadRecycleBin();
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`清空失败: ${error}`);
    }
  }
}

// 导出操作日志
async function exportOperationLogs() {
  if (!props.db_path) {
    ElMessage.warning('请先进行文件扫描');
    return;
  }

  try {
    const filePath = await save({
      filters: [
        { name: 'CSV文件', extensions: ['csv'] },
        { name: 'Markdown文件', extensions: ['md'] }
      ],
      defaultPath: `操作日志_${new Date().toISOString().slice(0, 10)}.csv`
    });

    if (!filePath) {
      return;
    }

    const format = filePath.endsWith('.md') ? 'md' : 'csv';

    await invoke('export_operation_logs', {
      db_path: props.db_path,
      output_path: filePath,
      format: format
    });

    ElMessage.success(`操作日志导出成功: ${filePath}`);
  } catch (error) {
    console.error('导出操作日志失败:', error);
    ElMessage.error(`导出失败: ${error}`);
  }
}

// 查询审计日志
async function queryAuditLogs() {
  if (!props.db_path) {
    ElMessage.warning('请先进行文件扫描');
    return;
  }

  isLoadingAuditLogs.value = true;
  try {
    const result = await invoke('query_audit_logs', {
      params: {
        start_date: null,
        end_date: null,
        event_types: [],
        severity_levels: [],
        page: 1,
        page_size: 100
      }
    }) as any;

    auditLogs.value = result.logs || [];
    auditLogStats.value = result.summary;
    ElMessage.success(`加载了 ${auditLogs.value.length} 条审计日志`);
  } catch (error) {
    console.error('查询审计日志失败:', error);
    ElMessage.error(`查询失败: ${error}`);
  } finally {
    isLoadingAuditLogs.value = false;
  }
}

// 格式化大小
function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// 格式化日期
function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

onMounted(() => {
  if (props.db_path) {
    scanEmptyFiles();
  }
});
</script>

<template>
  <div class="tools-center">
    <!-- 引入流光背景背景斑块 -->
    <div class="glow-spot glow-blue"></div>
    <div class="glow-spot glow-purple"></div>

    <!-- 页面头部 -->
    <div class="page-header">
      <div class="header-title">
        <h2>工具箱</h2>
        <p class="header-subtitle">内置空文件剪枝、重复文件夹合并、物理级安全回收站及合规级审计日志等实用工具</p>
      </div>
    </div>

    <!-- 工具选择标签 -->
    <div class="tool-tabs">
      <div 
        class="tool-tab" 
        :class="{ 'is-active': activeTool === 'empty-files' }"
        @click="activeTool = 'empty-files'"
      >
        <el-icon :size="20"><Document /></el-icon>
        <span>0 字节空文件</span>
        <el-tag v-if="emptyFiles.length > 0" type="danger" size="small" effect="dark" class="tab-badge">{{ emptyFiles.length }}</el-tag>
      </div>
      <div 
        class="tool-tab" 
        :class="{ 'is-active': activeTool === 'duplicate-folders' }"
        @click="activeTool = 'duplicate-folders'"
      >
        <el-icon :size="20"><Folder /></el-icon>
        <span>重复文件夹</span>
        <el-tag v-if="duplicateFolders.length > 0" type="warning" size="small" effect="dark" class="tab-badge">{{ duplicateFolders.length }}</el-tag>
      </div>
      <div 
        class="tool-tab" 
        :class="{ 'is-active': activeTool === 'recycle-bin' }"
        @click="activeTool = 'recycle-bin'; loadRecycleBin()"
      >
        <el-icon :size="20"><Delete /></el-icon>
        <span>回收站安全管理</span>
        <el-tag v-if="recycleBinItems.length > 0" type="info" size="small" effect="dark" class="tab-badge">{{ recycleBinItems.length }}</el-tag>
      </div>
      <div 
        class="tool-tab" 
        :class="{ 'is-active': activeTool === 'logs' }"
        @click="activeTool = 'logs'"
      >
        <el-icon :size="20"><List /></el-icon>
        <span>合规审计日志</span>
      </div>
    </div>

    <!-- 0字节文件工具 -->
    <div v-show="activeTool === 'empty-files'" class="tool-content">
      <el-card shadow="never" class="glass-card">
        <template #header>
          <div class="tool-header">
            <div class="header-left">
              <el-icon :size="20" color="#F56C6C"><Document /></el-icon>
              <span>0 字节文件检测与清理</span>
              <el-tooltip content="0字节文件通常是没有写入任何有效内容的废弃空文件，可安全清理释放节点占用">
                <el-icon :size="16" color="#909399" style="cursor: help;"><InfoFilled /></el-icon>
              </el-tooltip>
            </div>
            <div class="header-right">
              <el-button type="primary" @click="scanEmptyFiles" :loading="isScanningEmpty" class="glass-refresh-btn">
                <el-icon><Refresh /></el-icon>
                重新检索
              </el-button>
              <el-button 
                type="danger" 
                :disabled="selectedEmptyFiles.size === 0"
                @click="deleteSelectedEmptyFiles"
                class="glass-danger-btn"
              >
                <el-icon><Delete /></el-icon>
                彻底清空已选 ({{ selectedEmptyFiles.size }})
              </el-button>
            </div>
          </div>
        </template>

        <div v-if="emptyFiles.length === 0 && !isScanningEmpty" class="empty-state">
          <el-icon :size="54" color="rgba(64,158,255,0.25)"><Document /></el-icon>
          <p class="empty-desc">未在您的物理磁盘索引库中检索到 0 字节废弃文件</p>
          <el-button type="primary" plain size="small" @click="scanEmptyFiles" class="empty-action-btn">
            立即开始深度检索
          </el-button>
        </div>

        <div v-else-if="isScanningEmpty" class="loading-state">
          <el-skeleton :rows="6" animated />
        </div>

        <el-table 
          v-else 
          :data="emptyFiles" 
          class="glass-table"
          stripe 
          @selection-change="(rows: EmptyFile[]) => selectedEmptyFiles = new Set(rows.map(r => r.path))"
        >
          <el-table-column type="selection" width="55" />
          <el-table-column label="空文件名" prop="filename" min-width="180" show-overflow-tooltip>
            <template #default="{ row }">
              <span class="file-name-cell">
                <el-icon class="table-file-icon"><Document /></el-icon>
                <span>{{ row.filename }}</span>
              </span>
            </template>
          </el-table-column>
          <el-table-column label="完整物理路径" prop="path" min-width="320" show-overflow-tooltip>
            <template #default="{ row }">
              <span class="path-text">{{ row.path }}</span>
            </template>
          </el-table-column>
          <el-table-column label="文件建立日期" width="180">
            <template #default="{ row }">
              <span class="date-text">{{ formatDate(row.created_at) }}</span>
            </template>
          </el-table-column>
        </el-table>
      </el-card>
    </div>

    <!-- 重复文件夹工具 -->
    <div v-show="activeTool === 'duplicate-folders'" class="tool-content">
      <el-card shadow="never" class="glass-card">
        <template #header>
          <div class="tool-header">
            <div class="header-left">
              <el-icon :size="20" color="#E6A23C"><Folder /></el-icon>
              <span>完全相同文件夹检测</span>
              <el-tooltip content="检索并合并目录下所含子文件及哈希完全一致的镜像文件夹">
                <el-icon :size="16" color="#909399" style="cursor: help;"><InfoFilled /></el-icon>
              </el-tooltip>
            </div>
            <div class="header-right">
              <el-button type="primary" @click="findDuplicateFolders" :loading="isScanningFolders" class="glass-refresh-btn">
                <el-icon><Search /></el-icon>
                开启深度扫描
              </el-button>
            </div>
          </div>
        </template>

        <div v-if="duplicateFolders.length === 0 && !isScanningFolders" class="empty-state">
          <el-icon :size="54" color="rgba(230,162,60,0.25)"><Folder /></el-icon>
          <p class="empty-desc">点击下方按钮，智能检索文件库中具有完全相同树结构的目录</p>
          <el-button type="primary" plain size="small" @click="findDuplicateFolders" class="empty-action-btn">
            开始扫描重复文件夹
          </el-button>
        </div>

        <div v-else-if="isScanningFolders" class="loading-state">
          <el-skeleton :rows="6" animated />
        </div>

        <div v-else class="folder-groups">
          <el-collapse class="glass-collapse">
            <el-collapse-item 
              v-for="(group, index) in duplicateFolders" 
              :key="group.hash"
              :name="group.hash"
            >
              <template #title>
                <div class="collapse-header-title">
                  <el-icon class="folder-group-icon"><Folder /></el-icon>
                  <span class="group-title-text">重复文件夹镜像组 {{ index + 1 }}</span>
                  <el-tag size="small" type="danger" effect="plain" class="collapse-tag">{{ group.folders.length }} 处相同</el-tag>
                </div>
              </template>
              
              <div class="folder-group-info">
                <el-descriptions :column="3" border class="glass-descriptions">
                  <el-descriptions-item label="单组文件数">{{ group.file_count }}</el-descriptions-item>
                  <el-descriptions-item label="单组总大小">{{ formatSize(group.total_size) }}</el-descriptions-item>
                  <el-descriptions-item label="可优化空间"><strong style="color: #f56c6c;">{{ formatSize(group.total_size * (group.folders.length - 1)) }}</strong></el-descriptions-item>
                </el-descriptions>
              </div>
              
              <el-table :data="group.folders.map(f => ({ path: f }))" class="glass-table inner-table" stripe>
                <el-table-column label="文件夹物理位置" prop="path" min-width="400" show-overflow-tooltip />
                <el-table-column label="安全操作" width="120" align="right">
                  <template #default>
                    <el-button type="danger" link size="small" class="row-action-delete">
                      <el-icon><Delete /></el-icon>
                      合并删除
                    </el-button>
                  </template>
                </el-table-column>
              </el-table>
            </el-collapse-item>
          </el-collapse>
        </div>
      </el-card>
    </div>

    <!-- 回收站工具 -->
    <div v-show="activeTool === 'recycle-bin'" class="tool-content">
      <el-card shadow="never" class="glass-card">
        <template #header>
          <div class="tool-header">
            <div class="header-left">
              <el-icon :size="20" color="#909399"><Delete /></el-icon>
              <span>系统回收站安全管理</span>
            </div>
            <div class="header-right">
              <el-button type="danger" plain @click="emptyRecycleBin" :disabled="recycleBinItems.length === 0" class="glass-danger-btn">
                <el-icon><Brush /></el-icon>
                清空回收站
              </el-button>
            </div>
          </div>
        </template>

        <div v-if="recycleBinItems.length === 0 && !isLoadingRecycleBin" class="empty-state">
          <el-icon :size="54" color="rgba(144,147,153,0.25)"><Delete /></el-icon>
          <p class="empty-desc">回收站已完全清空，盘符空间充足</p>
        </div>

        <div v-else-if="isLoadingRecycleBin" class="loading-state">
          <el-skeleton :rows="6" animated />
        </div>

        <el-table v-else :data="recycleBinItems" class="glass-table" stripe>
          <el-table-column label="物理原始删除路径" prop="original_path" min-width="320" show-overflow-tooltip>
            <template #default="{ row }">
              <span class="path-text">{{ row.original_path }}</span>
            </template>
          </el-table-column>
          <el-table-column label="删除日期" width="180">
            <template #default="{ row }">
              <span class="date-text">{{ formatDate(row.deleted_at) }}</span>
            </template>
          </el-table-column>
          <el-table-column label="文件占用体积" width="130">
            <template #default="{ row }">
              <span class="size-text">{{ formatSize(row.size) }}</span>
            </template>
          </el-table-column>
          <el-table-column label="恢复操作" width="120" align="right">
            <template #default="{ row }">
              <el-button type="primary" link size="small" @click="restoreFile(row.id)" class="row-restore-btn">
                <el-icon><Refresh /></el-icon>
                原位恢复
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-card>
    </div>

    <!-- 日志管理工具 -->
    <div v-show="activeTool === 'logs'" class="tool-content">
      <el-card shadow="never" class="glass-card">
        <template #header>
          <div class="tool-header">
            <div class="header-left">
              <el-icon :size="20" color="#409EFF"><List /></el-icon>
              <span>合规审计日志分析</span>
            </div>
            <div class="header-right">
              <el-button type="primary" @click="exportOperationLogs" class="glass-export-btn">
                <el-icon><Download /></el-icon>
                导出操作日志
              </el-button>
              <el-button type="success" @click="queryAuditLogs" :loading="isLoadingAuditLogs" class="glass-submit-btn">
                <el-icon><Search /></el-icon>
                读取审计流水
              </el-button>
            </div>
          </div>
        </template>

        <!-- 高科技独立渐变色审计日志统计面板 -->
        <div class="stats-grid" v-if="auditLogStats">
          <div class="stat-card stat-blue">
            <div class="stat-icon-wrapper">
              <el-icon :size="22"><List /></el-icon>
            </div>
            <div class="stat-details">
              <span class="stat-lbl">审计事件总数</span>
              <span class="stat-val">{{ auditLogStats.total_events || 0 }} <small>条</small></span>
            </div>
            <div class="card-light"></div>
          </div>

          <div class="stat-card stat-red">
            <div class="stat-icon-wrapper">
              <el-icon :size="22"><Delete /></el-icon>
            </div>
            <div class="stat-details">
              <span class="stat-lbl">高危阻断事件</span>
              <span class="stat-val" style="color: #f56c6c;">{{ auditLogStats.critical_events || 0 }} <small>条</small></span>
            </div>
            <div class="card-light"></div>
          </div>

          <div class="stat-card stat-orange">
            <div class="stat-icon-wrapper">
              <el-icon :size="22"><Brush /></el-icon>
            </div>
            <div class="stat-details">
              <span class="stat-lbl">待处理风险流</span>
              <span class="stat-val" style="color: #e6a23c;">{{ auditLogStats.high_events || 0 }} <small>条</small></span>
            </div>
            <div class="card-light"></div>
          </div>

          <div class="stat-card stat-green">
            <div class="stat-icon-wrapper">
              <el-icon :size="22"><Brush /></el-icon>
            </div>
            <div class="stat-details">
              <span class="stat-lbl">系统合规指数</span>
              <span class="stat-val" style="color: #67c23a;">{{ auditLogStats.compliance_score || 100 }}%</span>
            </div>
            <div class="card-light"></div>
          </div>
        </div>

        <!-- 审计日志列表 -->
        <div v-if="auditLogs.length > 0" class="audit-log-results">
          <div class="results-header">
            <h4>操作审计流水历史 (已载入 {{ auditLogs.length }} 项)</h4>
          </div>
          <el-table :data="auditLogs" class="glass-table" stripe max-height="350">
            <el-table-column label="发生时间" prop="timestamp" width="180">
              <template #default="{ row }">
                <span class="date-text">{{ row.timestamp }}</span>
              </template>
            </el-table-column>
            <el-table-column label="事件类型" prop="event_type" width="120">
              <template #default="{ row }">
                <el-tag size="small" type="info">{{ row.event_type }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="严重级别" width="100">
              <template #default="{ row }">
                <el-tag :type="row.severity === 'critical' ? 'danger' : row.severity === 'high' ? 'warning' : 'success'" size="small" effect="plain">
                  {{ row.severity.toUpperCase() }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="受审计物理资源" prop="resource" width="180" show-overflow-tooltip />
            <el-table-column label="触发动作" prop="action" width="120" />
            <el-table-column label="事件详细描述" prop="details" show-overflow-tooltip />
            <el-table-column label="行为结果" prop="result" width="100">
              <template #default="{ row }">
                <span :style="{ color: row.result === 'success' ? '#67c23a' : '#f56c6c', fontWeight: 'bold' }">
                  {{ row.result.toUpperCase() }}
                </span>
              </template>
            </el-table-column>
          </el-table>
        </div>

        <div v-else-if="!isLoadingAuditLogs" class="empty-state">
          <el-icon :size="54" color="rgba(64,158,255,0.25)"><List /></el-icon>
          <p class="empty-desc">尚未检索到任何磁盘合规操作审计日志</p>
          <el-button type="primary" plain size="small" @click="queryAuditLogs" class="empty-action-btn">
            拉取最新审计流水
          </el-button>
        </div>

        <div v-else class="loading-state">
          <el-skeleton :rows="6" animated />
        </div>
      </el-card>
    </div>
  </div>
</template>

<style scoped>
.tools-center {
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
  opacity: 0.45;
}

.glow-blue {
  background: rgba(64, 158, 255, 0.12);
  top: 8%;
  right: 8%;
}

.glow-purple {
  background: rgba(155, 89, 182, 0.08);
  bottom: 8%;
  left: 8%;
}

.page-header {
  margin-bottom: 24px;
  position: relative;
  z-index: 2;
}

.page-header h2 {
  margin: 0 0 4px 0;
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}

.header-subtitle {
  margin: 0;
  color: #909399;
  font-size: 14px;
}

/* 工具标签胶囊化 */
.tool-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 24px;
  position: relative;
  z-index: 2;
}

.tool-tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 18px;
  background: rgba(255, 255, 255, 0.5) !important;
  backdrop-filter: blur(4px) !important;
  -webkit-backdrop-filter: blur(4px) !important;
  border: 1px solid rgba(255, 255, 255, 0.45) !important;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 600;
  color: #606266;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  user-select: none;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.01);
  position: relative;
}

.tool-tab:hover {
  background: rgba(64, 158, 255, 0.12) !important;
  border-color: rgba(64, 158, 255, 0.4) !important;
  color: #409EFF !important;
  transform: translateY(-1.5px);
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.08);
}

.tool-tab.is-active {
  background: linear-gradient(135deg, #409EFF 0%, #66b1ff 100%) !important;
  border-color: #409EFF !important;
  color: #ffffff !important;
  box-shadow: 0 4px 16px rgba(64, 158, 255, 0.25) !important;
}

.tab-badge {
  margin-left: 4px;
  border: none;
}

.tool-content {
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
  overflow: hidden;
}

.tool-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 14px;
  color: #303133;
}

.header-right {
  display: flex;
  gap: 12px;
}

/* 按钮磨砂美化 */
.glass-refresh-btn {
  background: rgba(64, 158, 255, 0.1) !important;
  border: 1px solid rgba(64, 158, 255, 0.2) !important;
  color: #409EFF !important;
  font-weight: 500;
}

.glass-refresh-btn:hover {
  background: #409EFF !important;
  border-color: #409EFF !important;
  color: white !important;
}

.glass-danger-btn {
  background: rgba(245, 108, 108, 0.1) !important;
  border: 1px solid rgba(245, 108, 108, 0.2) !important;
  color: #f56c6c !important;
  font-weight: 500;
}

.glass-danger-btn:hover:not(:disabled) {
  background: #f56c6c !important;
  border-color: #f56c6c !important;
  color: white !important;
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

.glass-export-btn {
  background: rgba(255, 255, 255, 0.6) !important;
  border: 1px solid rgba(220, 223, 230, 0.5) !important;
  color: #606266 !important;
}

.glass-export-btn:hover {
  border-color: #409EFF !important;
  color: #409EFF !important;
}

/* 空状态美化 */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 60px 20px;
  text-align: center;
}

.empty-desc {
  margin: 16px 0;
  color: #909399;
  font-size: 13px;
  max-width: 320px;
  line-height: 1.5;
}

.empty-action-btn {
  margin-top: 4px;
}

.loading-state {
  padding: 24px;
}

/* Collapse 面板美化 */
.glass-collapse {
  background: transparent !important;
  border: none !important;
}

.glass-collapse :deep(.el-collapse-item) {
  border-bottom: 1px solid rgba(220, 223, 230, 0.3) !important;
  background: transparent !important;
}

.glass-collapse :deep(.el-collapse-item__header) {
  background: rgba(245, 247, 250, 0.3) !important;
  border-bottom: 1px solid rgba(220, 223, 230, 0.2) !important;
  padding: 0 16px;
  height: 48px;
  color: #303133;
  font-weight: 600;
  font-size: 13px;
}

.glass-collapse :deep(.el-collapse-item__wrap) {
  background: rgba(255, 255, 255, 0.2) !important;
  border-bottom: none !important;
}

.glass-collapse :deep(.el-collapse-item__content) {
  padding: 16px;
}

.collapse-header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.folder-group-icon {
  color: #e6a23c;
  font-size: 16px;
}

.group-title-text {
  font-size: 13px;
  font-weight: 600;
}

.collapse-tag {
  margin-left: auto;
  margin-right: 12px;
  border-radius: 4px;
}

.folder-group-info {
  margin-bottom: 16px;
}

.glass-descriptions :deep(.el-descriptions__body) {
  background: transparent !important;
}

.glass-descriptions :deep(.el-descriptions-item__cell) {
  background: rgba(255, 255, 255, 0.3) !important;
}

/* 统计卡片网格 */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
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

.stat-red {
  background: linear-gradient(135deg, rgba(245, 108, 108, 0.08) 0%, rgba(245, 108, 108, 0.02) 100%);
}

.stat-red:hover {
  border-color: rgba(245, 108, 108, 0.3);
}

.stat-red .stat-icon-wrapper {
  background: rgba(245, 108, 108, 0.15);
  color: #f56c6c;
}

.stat-orange {
  background: linear-gradient(135deg, rgba(230, 162, 60, 0.08) 0%, rgba(230, 162, 60, 0.02) 100%);
}

.stat-orange:hover {
  border-color: rgba(230, 162, 60, 0.3);
}

.stat-orange .stat-icon-wrapper {
  background: rgba(230, 162, 60, 0.15);
  color: #e6a23c;
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
  width: 44px;
  height: 44px;
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
}

.stat-val small {
  font-size: 12px;
  font-weight: 500;
  color: #606266;
}

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

/* 审计列表 */
.audit-log-results {
  margin-top: 20px;
}

.audit-log-results .results-header {
  margin-bottom: 12px;
}

.audit-log-results .results-header h4 {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
}

/* 文件单元格 */
.file-name-cell {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 500;
}

.table-file-icon {
  color: #909399;
}

.path-text {
  font-size: 11.5px;
  color: #606266;
}

.date-text {
  font-size: 12px;
  color: #909399;
}

.size-text {
  font-size: 12px;
  color: #606266;
  font-weight: 500;
}

.row-action-delete:hover {
  color: #f56c6c !important;
}

.row-restore-btn {
  font-weight: 500;
}

/* 表格全局覆盖 */
.glass-table {
  background: transparent !important;
  width: 100%;
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

.inner-table {
  background: rgba(255, 255, 255, 0.2) !important;
  border-radius: 8px;
  border: 1px solid rgba(220, 223, 230, 0.2);
}

@media (max-width: 992px) {
  .stats-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 768px) {
  .stats-grid {
    grid-template-columns: 1fr;
  }
  
  .tool-header {
    flex-direction: column;
    gap: 16px;
    align-items: flex-start;
  }
}
</style>




