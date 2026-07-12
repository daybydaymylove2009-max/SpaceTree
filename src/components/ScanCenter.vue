<script setup lang="ts">
import { ref, onUnmounted, h } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { 
  Folder, Plus, Delete, Refresh, Loading, VideoPause, VideoPlay, 
  Setting, Clock, Check, Warning, Document, FolderOpened, Close,
  FullScreen, Search, ArrowDown, InfoFilled
} from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

// 进度更新定时器
let progressInterval: number | null = null;

interface ScanConfig {
  max_depth: number;
  include_extensions: string[];
  exclude_extensions: string[];
  min_size: number;
  max_size: number;
  exclude_hidden: boolean;
  exclude_system: boolean;
  exclude_patterns: string[];
  hash_algorithm: 'xxhash3' | 'xxhash64' | 'blake3' | 'sha256' | 'sha512' | 'md5';
}

// 哈希算法选项
const hashAlgorithmOptions = [
  {
    value: 'xxhash3',
    label: 'XXH3 (推荐)',
    description: '极速非加密哈希，适合重复文件检测',
    speed: '⭐⭐⭐⭐⭐',
    security: '低',
    useCase: '大文件快速扫描'
  },
  {
    value: 'xxhash64',
    label: 'XXH64',
    description: '64位高速哈希，兼容性更好',
    speed: '⭐⭐⭐⭐⭐',
    security: '低',
    useCase: '通用快速扫描'
  },
  {
    value: 'blake3',
    label: 'BLAKE3',
    description: '现代安全哈希，速度与安全的最佳平衡',
    speed: '⭐⭐⭐⭐',
    security: '高',
    useCase: '需要安全校验的场景'
  },
  {
    value: 'sha256',
    label: 'SHA-256',
    description: '标准安全哈希算法，广泛兼容',
    speed: '⭐⭐',
    security: '高',
    useCase: '安全完整性校验'
  },
  {
    value: 'sha512',
    label: 'SHA-512',
    description: '高安全级别哈希，抗碰撞性更好',
    speed: '⭐⭐',
    security: '极高',
    useCase: '超高安全需求'
  },
  {
    value: 'md5',
    label: 'MD5 (不推荐)',
    description: '已弃用的哈希算法，仅用于兼容旧系统',
    speed: '⭐⭐⭐',
    security: '低',
    useCase: '遗留系统兼容'
  }
];

interface IncrementalScanConfig {
  directories: string[];
  last_scan_time?: string;
  scan_config?: ScanConfig;
  use_file_watcher: boolean;
  detect_moves: boolean;
  detect_renames: boolean;
}

const props = defineProps<{
  db_path: string;
}>()

const emit = defineEmits<{
  (e: 'scanComplete', result: any): void;
  (e: 'duplicatesFound', classification: any): void;
  (e: 'goToDuplicates'): void;
}>()

// 扫描状态
const directories = ref<string[]>([]);
const isScanning = ref(false);
const isPaused = ref(false);
const scanProgress = ref(0);
const scanStatus = ref('');
const scanCurrentFile = ref('');
const scannedFiles = ref(0);
const totalFiles = ref(0);
const scannedDirectories = ref(0);
const currentDirectory = ref('');
const recentFiles = ref<string[]>([]);
const scanMode = ref<'full' | 'incremental'>('incremental');
const scanStartTime = ref<Date | null>(null);
const estimatedTimeRemaining = ref('');

// 配置面板
const showConfigPanel = ref(false);
const scanConfig = ref<ScanConfig>({
  max_depth: 10,
  include_extensions: [],
  exclude_extensions: ['tmp', 'log', 'cache'],
  min_size: 0,
  max_size: 0,
  exclude_hidden: true,
  exclude_system: true,
  exclude_patterns: ['node_modules', '.git', 'target', 'dist'],
  hash_algorithm: 'xxhash3'
});

const commonExtensions = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'txt', 'md', 'json', 'xml', 'csv', 'doc', 'docx', 'pdf', 'xls', 'xlsx', 'ppt', 'pptx', 'mp3', 'mp4', 'avi', 'mov', 'zip', 'rar', '7z', 'tar', 'gz', 'exe', 'dll', 'so', 'dylib'];

// 扫描历史
const scanHistory = ref<any[]>([]);
const showHistoryPanel = ref(false);
const historyLoading = ref(false);
const historyTotal = ref(0);
const historyPage = ref(1);
const historyPageSize = ref(10);

// 历史记录搜索和筛选
const historySearchKeyword = ref('');
const historyDateRange = ref<[Date, Date] | null>(null);
const historyStatus = ref<string>('');
const historySortBy = ref<string>('scan_time');
const historySortOrder = ref<string>('DESC');

// 历史记录统计
const historyStatistics = ref<any>(null);
const showHistoryStats = ref(false);

// 选择目录
async function selectDirectory() {
  try {
    const selected = await open({
      directory: true,
      multiple: false
    });
    if (selected && selected !== '') {
      if (!directories.value.includes(selected as string)) {
        directories.value.push(selected as string);
      }
    }
  } catch (error) {
    console.error('选择目录失败:', error);
    ElMessage.error(`选择目录失败: ${error}`);
  }
}

// 移除目录
function removeDirectory(index: number) {
  directories.value.splice(index, 1);
}

// 清空目录
function clearAllDirectories() {
  ElMessageBox.confirm(
    '确定要清空所有已选目录吗？',
    '清空目录',
    {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    }
  ).then(() => {
    directories.value = [];
    ElMessage.success('已清空所有目录');
  }).catch(() => {});
}

// 获取目录标签
function getDirLabel(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

// 获取显示路径
function getDisplayPath(path: string): string {
  if (path.length <= 40) return path;
  return path.substring(0, 15) + '...' + path.substring(path.length - 20);
}

// 开始扫描
async function startScan() {
  if (directories.value.length === 0) {
    ElMessage.warning('请至少选择一个扫描目录');
    return;
  }

  if (!props.db_path) {
    ElMessage.warning('数据库路径未设置，请等待应用初始化完成');
    return;
  }

  if (scanMode.value === 'full') {
    try {
      await ElMessageBox.confirm(
        '全量扫描将清空现有数据库，是否继续？',
        '全量扫描确认',
        {
          confirmButtonText: '继续扫描',
          cancelButtonText: '取消',
          type: 'warning'
        }
      );
    } catch {
      return;
    }
  }

  isScanning.value = true;
  isPaused.value = false;
  scanProgress.value = 0;
  scanStatus.value = '正在准备扫描...';
  scanStartTime.value = new Date();
  scannedFiles.value = 0;
  totalFiles.value = 0;
  scannedDirectories.value = 0;
  currentDirectory.value = '';
  recentFiles.value = [];
  scanCurrentFile.value = '';
  estimatedTimeRemaining.value = '';
  
  // 启动进度更新
  startProgressUpdate();
  
  // 给进度更新一点时间启动
  await new Promise(resolve => setTimeout(resolve, 100));

  try {
    if (scanMode.value === 'incremental') {
      // 增量扫描
      const config: IncrementalScanConfig = {
        directories: directories.value,
        scan_config: scanConfig.value,
        use_file_watcher: true,
        detect_moves: true,
        detect_renames: true
      };

      const result = await invoke('incremental_scan', {
        db_path: props.db_path,
        config: config
      });

      ElMessage.success(`增量扫描完成！新文件: ${(result as any).new_files.length}, 修改: ${(result as any).modified_files.length}`);
      
      // 自动查找重复文件
      await findDuplicates();
    } else {
      // 全量扫描
      console.log('开始全量扫描，先清空数据库...');
      try {
        const deleted = await invoke('clear_database', { db_path: props.db_path });
        console.log('数据库已清空，删除记录数:', deleted);
      } catch (clearError) {
        console.error('清空数据库失败:', clearError);
        ElMessage.error(`清空数据库失败: ${clearError}`);
        isScanning.value = false;
        return;
      }
      
      for (const dir of directories.value) {
        if (!isScanning.value) break;
        
        scanStatus.value = `正在扫描: ${getDirLabel(dir)}`;
        
        await invoke('scan_directories', {
          db_path: props.db_path,
          directories: [dir],
          config: scanConfig.value
        });
      }

      if (isScanning.value) {
        ElMessage.success('全量扫描完成');
        await findDuplicates();
      }
    }

    const wasScanning = isScanning.value;
    emit('scanComplete', { mode: scanMode.value, directories: directories.value });
    if (wasScanning) {
      showDuplicatesRedirectDialog();
    }
  } catch (error) {
    if (isScanning.value) {
      ElMessage.error(`扫描失败: ${error}`);
    }
  } finally {
    isScanning.value = false;
    isPaused.value = false;
    scanProgress.value = 0;
    scanStatus.value = '';
    // 停止进度更新
    stopProgressUpdate();
  }
}

// 提示是否跳转到极速查重工作台（倒计时 30 秒）
function showDuplicatesRedirectDialog() {
  const timeLeft = ref(30);
  let hasClosed = false;

  const timer = setInterval(() => {
    timeLeft.value--;
    if (timeLeft.value <= 0) {
      clearInterval(timer);
      if (!hasClosed) {
        hasClosed = true;
        ElMessageBox.close();
        emit('goToDuplicates');
      }
    }
  }, 1000);

  ElMessageBox.confirm(
    () => h('div', { style: 'text-align: left; padding: 4px 0;' }, [
      h('p', { style: 'margin-bottom: 8px; font-size: 15px; font-weight: 500;' }, '文件扫描已圆满成功！'),
      h('p', { style: 'color: #606266; font-size: 14px; margin-bottom: 12px;' }, '是否立即前往 重复文件-极速查重工作台 进行一键清理与重定位？'),
      h('p', { style: 'color: #909399; font-size: 13px;' }, `（若无操作，将在 ${timeLeft.value} 秒后自动跳转）`)
    ]),
    '扫描完成',
    {
      confirmButtonText: '立即前往',
      cancelButtonText: '留在当前页',
      type: 'success',
      closeOnClickModal: false,
      closeOnPressEscape: false,
      showClose: false
    }
  ).then(() => {
    clearInterval(timer);
    hasClosed = true;
    emit('goToDuplicates');
  }).catch(() => {
    clearInterval(timer);
    hasClosed = true;
    console.log('用户选择留在当前扫描中心页面');
  });
}

// 暂停扫描
async function pauseScan() {
  try {
    await invoke('pause_scan');
    isPaused.value = true;
    ElMessage.info('扫描已暂停');
  } catch (error) {
    ElMessage.error(`暂停失败: ${error}`);
  }
}

// 恢复扫描
async function resumeScan() {
  try {
    await invoke('resume_scan');
    isPaused.value = false;
    ElMessage.success('扫描已恢复');
  } catch (error) {
    ElMessage.error(`恢复失败: ${error}`);
  }
}

// 停止扫描
async function stopScan() {
  try {
    await invoke('stop_scan');
    isScanning.value = false;
    isPaused.value = false;
    ElMessage.info('扫描已停止');
  } catch (error) {
    ElMessage.error(`停止失败: ${error}`);
  }
}

// 开始进度更新
function startProgressUpdate() {
  // 清除之前的定时器
  if (progressInterval) {
    clearInterval(progressInterval);
    progressInterval = null;
  }
  
  // 每100ms更新一次进度
  progressInterval = window.setInterval(async () => {
    if (!isScanning.value) {
      stopProgressUpdate();
      return;
    }
    
    try {
      const progress = await invoke('get_scan_progress') as {
        total_files: number;
        processed_files: number;
        current_file: string;
        percentage: number;
        is_paused: boolean;
        is_stopped: boolean;
        scanned_directories: number;
        current_directory: string;
        recent_files: string[];
      };
      
      scanProgress.value = Math.round(progress.percentage);
      scannedFiles.value = progress.processed_files;
      totalFiles.value = progress.total_files;
      scanCurrentFile.value = progress.current_file;
      scannedDirectories.value = progress.scanned_directories;
      currentDirectory.value = progress.current_directory;
      recentFiles.value = progress.recent_files;
      
      // 更新预计剩余时间
      if (scanStartTime.value && progress.percentage > 0) {
        const elapsed = Date.now() - scanStartTime.value.getTime();
        const totalEstimated = elapsed / (progress.percentage / 100);
        const remaining = totalEstimated - elapsed;
        estimatedTimeRemaining.value = formatTimeRemaining(remaining);
      }
    } catch (error) {
      console.error('获取进度失败:', error);
    }
  }, 100);
}

// 停止进度更新
function stopProgressUpdate() {
  if (progressInterval) {
    clearInterval(progressInterval);
    progressInterval = null;
  }
}

// 格式化剩余时间
function formatTimeRemaining(ms: number): string {
  if (ms < 0) return '计算中...';
  const seconds = Math.ceil(ms / 1000);
  if (seconds < 60) return `${seconds}秒`;
  const minutes = Math.ceil(seconds / 60);
  if (minutes < 60) return `${minutes}分钟`;
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return `${hours}小时${remainingMinutes}分钟`;
}

// 查找重复文件
async function findDuplicates() {
  try {
    console.log('开始查找重复文件, db_path:', props.db_path);
    
    if (!props.db_path) {
      console.error('数据库路径为空');
      ElMessage.error('数据库路径为空，请等待应用初始化完成');
      return;
    }
    
    console.log('调用 find_duplicates 命令...');
    const duplicates = await invoke('find_duplicates', { db_path: props.db_path });
    console.log('查找重复文件结果:', duplicates);
    console.log('结果类型:', typeof duplicates);
    console.log('结果JSON:', JSON.stringify(duplicates, null, 2));
    
    if (!duplicates) {
      console.error('查找重复文件返回空结果');
      ElMessage.error('查找重复文件返回空结果');
      return;
    }
    
    // 检查返回的数据结构
    const keys = Object.keys(duplicates);
    console.log('返回数据的键:', keys);
    
    // 尝试访问字段
    console.log('complete_duplicates:', (duplicates as any).complete_duplicates);
    console.log('name_duplicates:', (duplicates as any).name_duplicates);
    console.log('content_duplicates:', (duplicates as any).content_duplicates);
    
    emit('duplicatesFound', duplicates);
  } catch (error) {
    console.error('查找重复文件失败:', error);
    ElMessage.error(`查找重复文件失败: ${error}`);
  }
}

// 打开配置面板
function openConfigPanel() {
  console.log('打开扫描配置面板');
  showConfigPanel.value = true;
}

// 保存配置
function saveConfig() {
  ElMessage.success('扫描配置已保存');
  showConfigPanel.value = false;
}

// 恢复默认配置
function resetConfig() {
  ElMessageBox.confirm(
    '确定要恢复默认配置吗？',
    '恢复默认',
    {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    }
  ).then(() => {
    scanConfig.value = {
      max_depth: 10,
      include_extensions: [],
      exclude_extensions: ['tmp', 'log', 'cache'],
      min_size: 0,
      max_size: 0,
      exclude_hidden: true,
      exclude_system: true,
      exclude_patterns: ['node_modules', '.git', 'target', 'dist'],
      hash_algorithm: 'xxhash3'
    };
    ElMessage.success('已恢复默认配置');
  }).catch(() => {});
}

// 打开历史记录面板
async function openHistoryPanel() {
  console.log('打开历史记录面板');
  showHistoryPanel.value = true;
  await loadScanHistory();
  await loadHistoryStatistics();
}

// 加载扫描历史
async function loadScanHistory() {
  try {
    historyLoading.value = true;
    console.log('开始加载扫描历史，dbPath:', props.db_path);

    // 构建查询参数
    const params: any = {
      page: historyPage.value,
      page_size: historyPageSize.value,
      sort_by: historySortBy.value,
      sort_order: historySortOrder.value,
    };

    // 添加搜索关键词
    if (historySearchKeyword.value) {
      params.search_keyword = historySearchKeyword.value;
    }

    // 添加日期范围
    if (historyDateRange.value && historyDateRange.value.length === 2) {
      params.start_date = historyDateRange.value[0].toISOString().split('T')[0];
      params.end_date = historyDateRange.value[1].toISOString().split('T')[0];
    }

    // 添加状态筛选
    if (historyStatus.value) {
      params.status = historyStatus.value;
    }

    const result = await invoke('get_scan_history_list', {
      db_path: props.db_path,
      params: params
    });

    console.log('获取到历史记录:', result);
    const historyResult = result as { history: any[], total_count: number, page: number, page_size: number };
    scanHistory.value = historyResult.history || [];
    historyTotal.value = historyResult.total_count || 0;
    console.log('设置历史记录完成，数量:', scanHistory.value.length, '总计:', historyTotal.value);
  } catch (error) {
    console.error('加载历史失败:', error);
    ElMessage.error(`加载历史失败: ${error}`);
  } finally {
    historyLoading.value = false;
  }
}

// 加载历史统计
async function loadHistoryStatistics() {
  try {
    const result = await invoke('get_scan_history_statistics', { db_path: props.db_path });
    historyStatistics.value = result;
    console.log('历史统计:', result);
  } catch (error) {
    console.error('加载历史统计失败:', error);
  }
}

// 搜索历史记录
function searchHistory() {
  historyPage.value = 1;
  loadScanHistory();
}

// 重置搜索条件
function resetHistorySearch() {
  historySearchKeyword.value = '';
  historyDateRange.value = null;
  historyStatus.value = '';
  historySortBy.value = 'scan_time';
  historySortOrder.value = 'DESC';
  historyPage.value = 1;
  loadScanHistory();
}

// 切换排序
function toggleSort(column: string) {
  if (historySortBy.value === column) {
    historySortOrder.value = historySortOrder.value === 'ASC' ? 'DESC' : 'ASC';
  } else {
    historySortBy.value = column;
    historySortOrder.value = 'DESC';
  }
  loadScanHistory();
}

// 处理分页变化
function handlePageChange(page: number) {
  historyPage.value = page;
  loadScanHistory();
}

// 处理每页条数变化
function handleSizeChange(size: number) {
  historyPageSize.value = size;
  historyPage.value = 1;
  loadScanHistory();
}

// 清除历史记录
async function clearHistory() {
  try {
    await ElMessageBox.confirm(
      '确定要清空所有扫描历史吗？此操作不可恢复',
      '清空历史',
      {
        confirmButtonText: '确定清空',
        cancelButtonText: '取消',
        type: 'warning'
      }
    );

    const deleted = await invoke('clear_scan_history', { db_path: props.db_path });
    ElMessage.success(`已清空 ${deleted} 条历史记录`);
    await loadScanHistory();
    await loadHistoryStatistics();
  } catch (error) {
    if (error !== 'cancel') {
      console.error('清空历史失败:', error);
      ElMessage.error(`清空历史失败: ${error}`);
    }
  }
}

// 清除指定日期之前的历史记录
async function clearHistoryBeforeDate() {
  try {
    const { value: date } = await ElMessageBox.prompt(
      '请输入日期（格式：YYYY-MM-DD），将删除该日期之前的所有历史记录',
      '按日期清除',
      {
        confirmButtonText: '确定删除',
        cancelButtonText: '取消',
        inputPattern: /^\d{4}-\d{2}-\d{2}$/,
        inputErrorMessage: '日期格式不正确，请使用 YYYY-MM-DD 格式'
      }
    );

    if (date) {
      const deleted = await invoke('clear_scan_history', {
        db_path: props.db_path,
        before_date: date
      });
      ElMessage.success(`已删除 ${date} 之前的 ${deleted} 条历史记录`);
      await loadScanHistory();
      await loadHistoryStatistics();
    }
  } catch (error) {
    if (error !== 'cancel') {
      console.error('清除历史失败:', error);
      ElMessage.error(`清除历史失败: ${error}`);
    }
  }
}

// 保留最近N条历史记录
async function keepRecentHistory() {
  try {
    const { value: count } = await ElMessageBox.prompt(
      '请输入要保留的最近记录数',
      '保留最近记录',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        inputPattern: /^[1-9]\d*$/,
        inputErrorMessage: '请输入有效的数字'
      }
    );

    if (count) {
      const keepCount = parseInt(count);
      const deleted = await invoke('clear_scan_history', {
        db_path: props.db_path,
        keep_recent: keepCount
      });
      ElMessage.success(`已保留最近 ${keepCount} 条记录，删除 ${deleted} 条`);
      await loadScanHistory();
      await loadHistoryStatistics();
    }
  } catch (error) {
    if (error !== 'cancel') {
      console.error('清除历史失败:', error);
      ElMessage.error(`清除历史失败: ${error}`);
    }
  }
}

// 处理清除命令
function handleClearCommand(command: string) {
  switch (command) {
    case 'all':
      clearHistory();
      break;
    case 'beforeDate':
      clearHistoryBeforeDate();
      break;
    case 'keepRecent':
      keepRecentHistory();
      break;
  }
}

// 格式化文件大小
function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// 格式化秒数为时长
function formatDurationSeconds(seconds: number): string {
  if (seconds < 60) {
    return seconds + '秒';
  } else if (seconds < 3600) {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return secs > 0 ? `${mins}分${secs}秒` : `${mins}分`;
  } else {
    const hours = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    return mins > 0 ? `${hours}小时${mins}分` : `${hours}小时`;
  }
}

// 格式化时长
function formatDuration(startTime: Date): string {
  const now = new Date();
  const diff = Math.floor((now.getTime() - startTime.getTime()) / 1000);
  const minutes = Math.floor(diff / 60);
  const seconds = diff % 60;
  return `${minutes}分${seconds}秒`;
}

async function addAllLocalDrives() {
  const possibleDrives = ['C:\\', 'D:\\', 'E:\\', 'F:\\', 'G:\\', 'H:\\'];
  const activeDrives: string[] = [];
  const { exists } = await import('@tauri-apps/plugin-fs');
  
  for (const drive of possibleDrives) {
    try {
      const hasDrive = await exists(drive);
      if (hasDrive) {
        activeDrives.push(drive);
      }
    } catch {
      // 忽略
    }
  }
  
  if (activeDrives.length === 0) {
    activeDrives.push('C:\\');
  }
  
  directories.value = activeDrives;
  ElMessage.success(`已探测并载入本地磁盘: ${activeDrives.join(', ')}。即刻为您开启 MFT/USN 驱动级极速扫描...`);
  
  // 自动切换为增量扫描并执行
  scanMode.value = 'incremental';
  setTimeout(() => {
    startScan();
  }, 500);
}

// 组件卸载时清理
onUnmounted(() => {
  stopProgressUpdate();
});
</script>

<template>
  <div class="scan-center">
    <!-- 页面标题 -->
    <div class="page-header">
      <div class="header-title">
        <h2>扫描中心</h2>
        <p class="header-subtitle">智能扫描和管理您的文件</p>
      </div>
      <div class="header-actions">
        <el-button text @click="openConfigPanel">
          <el-icon><Setting /></el-icon>
          扫描配置
        </el-button>
        <el-button text @click="openHistoryPanel">
          <el-icon><Clock /></el-icon>
          历史记录
        </el-button>
      </div>
    </div>

    <!-- 扫描模式选择 -->
    <el-card class="mode-card" shadow="never">
      <div class="mode-selector">
        <div 
          class="mode-option" 
          :class="{ 'is-active': scanMode === 'incremental' }"
          @click="scanMode = 'incremental'"
        >
          <div class="mode-icon">
            <el-icon :size="24"><Refresh /></el-icon>
          </div>
          <div class="mode-info">
            <h4>增量扫描</h4>
            <p>仅扫描新增和修改的文件，速度更快</p>
          </div>
          <el-tag v-if="scanMode === 'incremental'" type="success" effect="dark">推荐</el-tag>
        </div>
        <div 
          class="mode-option" 
          :class="{ 'is-active': scanMode === 'full' }"
          @click="scanMode = 'full'"
        >
          <div class="mode-icon">
            <el-icon :size="24"><FullScreen /></el-icon>
          </div>
          <div class="mode-info">
            <h4>全量扫描</h4>
            <p>重新扫描所有文件，数据更准确</p>
          </div>
        </div>
      </div>
    </el-card>

    <!-- 目录选择 -->
    <el-card class="directory-card" shadow="never">
      <template #header>
        <div class="card-header">
          <span>选择扫描目录</span>
          <el-badge v-if="directories.length > 0" :value="directories.length" type="primary" />
        </div>
      </template>

      <div class="directory-content">
        <!-- 空状态 -->
        <div v-if="directories.length === 0" class="directory-empty">
          <!-- 一键 Windows NTFS USN 极速全盘索引卡片 (MFT/USN 驱动级极速体验) -->
          <div class="usn-fast-entry" @click="addAllLocalDrives">
            <el-icon class="usn-icon"><Cpu /></el-icon>
            <div class="usn-info">
              <h3>一键 MFT/USN 闪电全盘索引</h3>
              <p>特权级驱动扫描通道，秒级完成全盘百万级文件建库</p>
            </div>
          </div>

          <div class="divider-line">
            <span>或者</span>
          </div>

          <el-icon :size="48" color="#dcdfe6"><FolderOpened /></el-icon>
          <p>添加要扫描的目录</p>
          <el-button type="primary" @click="selectDirectory">
            <el-icon><Plus /></el-icon>
            添加目录
          </el-button>
        </div>

        <!-- 目录列表 -->
        <div v-else class="directory-list">
          <TransitionGroup name="directory">
            <div 
              v-for="(dir, index) in directories" 
              :key="dir"
              class="directory-item"
            >
              <div class="item-icon">
                <el-icon :size="20"><Folder /></el-icon>
              </div>
              <div class="item-info">
                <span class="item-name">{{ getDirLabel(dir) }}</span>
                <span class="item-path">{{ getDisplayPath(dir) }}</span>
              </div>
              <el-button 
                circle
                text
                size="small"
                @click="removeDirectory(index)"
                class="item-remove"
              >
                <el-icon><Close /></el-icon>
              </el-button>
            </div>
          </TransitionGroup>

          <div class="directory-actions">
            <el-button type="primary" plain @click="selectDirectory">
              <el-icon><Plus /></el-icon>
              继续添加
            </el-button>
            <el-button text type="danger" @click="clearAllDirectories">
              <el-icon><Delete /></el-icon>
              清空
            </el-button>
          </div>
        </div>
      </div>
    </el-card>

    <!-- 扫描配置抽屉 -->
    <el-drawer
      v-model="showConfigPanel"
      title="扫描配置"
      size="550px"
      :append-to-body="true"
      :modal="true"
      :close-on-click-modal="false"
      destroy-on-close
    >
      <div class="config-drawer-content">
        <!-- 配置说明 -->
        <el-alert
          title="配置说明"
          description="以下配置将影响扫描行为。合理的配置可以提高扫描效率并减少不必要的文件处理"
          type="info"
          :closable="false"
          style="margin-bottom: 20px;"
        />

        <el-form :model="scanConfig" label-position="top" class="config-form">
          <!-- 扫描深度 -->
          <el-card shadow="never" style="margin-bottom: 16px;">
            <template #header>
              <span>扫描深度</span>
            </template>
            <el-form-item label="最大扫描深度">
              <el-slider v-model="scanConfig.max_depth" :min="1" :max="20" show-stops show-input />
              <div class="form-item-hint">限制扫描的目录层级深度，较大的值会扫描更深层的子目录</div>
            </el-form-item>
          </el-card>

          <!-- 文件大小范围 -->
          <el-card shadow="never" style="margin-bottom: 16px;">
            <template #header>
              <span>文件大小范围</span>
            </template>
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="最小文件大小(字节)">
                  <el-input-number v-model="scanConfig.min_size" :min="0" :step="1024" style="width: 100%" />
                  <div class="form-item-hint">{{ formatFileSize(scanConfig.min_size) }}</div>
                </el-form-item>
              </el-col>
              <el-col :span="12">
                <el-form-item label="最大文件大小(字节)">
                  <el-input-number v-model="scanConfig.max_size" :min="0" :step="1024" style="width: 100%" />
                  <div class="form-item-hint">{{ scanConfig.max_size === 0 ? '无限' : formatFileSize(scanConfig.max_size) }}</div>
                </el-form-item>
              </el-col>
            </el-row>
          </el-card>

          <!-- 文件类型过滤 -->
          <el-card shadow="never" style="margin-bottom: 16px;">
            <template #header>
              <span>文件类型过滤</span>
            </template>
            <el-form-item label="包含的文件类型">
              <el-select
                v-model="scanConfig.include_extensions"
                multiple
                filterable
                allow-create
                placeholder="选择或输入文件扩展名（为空则包含所有类型）"
                style="width: 100%"
              >
                <el-option
                  v-for="ext in commonExtensions"
                  :key="ext"
                  :label="`.${ext}`"
                  :value="ext"
                />
              </el-select>
              <div class="form-item-hint">只扫描指定的文件类型，为空则扫描所有类型</div>
            </el-form-item>

            <el-form-item label="排除的文件类型">
              <el-select
                v-model="scanConfig.exclude_extensions"
                multiple
                filterable
                allow-create
                placeholder="选择或输入要排除的扩展名"
                style="width: 100%"
              >
                <el-option
                  v-for="ext in commonExtensions"
                  :key="ext"
                  :label="`.${ext}`"
                  :value="ext"
                />
              </el-select>
              <div class="form-item-hint">排除指定的文件类型，这些文件将不会被扫描</div>
            </el-form-item>
          </el-card>

          <!-- 排除模式 -->
          <el-card shadow="never" style="margin-bottom: 16px;">
            <template #header>
              <span>排除模式</span>
            </template>
            <el-form-item label="排除的目录或文件模式">
              <el-select
                v-model="scanConfig.exclude_patterns"
                multiple
                filterable
                allow-create
                placeholder="输入要排除的目录或文件模式"
                style="width: 100%"
              >
                <el-option label="node_modules" value="node_modules" />
                <el-option label=".git" value=".git" />
                <el-option label="target" value="target" />
                <el-option label="dist" value="dist" />
                <el-option label="build" value="build" />
                <el-option label=".idea" value=".idea" />
                <el-option label=".vscode" value=".vscode" />
                <el-option label="__pycache__" value="__pycache__" />
                <el-option label="*.tmp" value="*.tmp" />
                <el-option label="*.log" value="*.log" />
              </el-select>
              <div class="form-item-hint">匹配的目录或文件将被跳过</div>
            </el-form-item>
          </el-card>

          <!-- 哈希算法选择 -->
          <el-card shadow="never" style="margin-bottom: 16px;">
            <template #header>
              <div style="display: flex; align-items: center; gap: 8px;">
                <span>哈希算法</span>
                <el-tooltip content="选择用于文件内容校验的哈希算法">
                  <el-icon style="color: #909399;"><InfoFilled /></el-icon>
                </el-tooltip>
              </div>
            </template>
            <el-form-item>
              <el-select
                v-model="scanConfig.hash_algorithm"
                style="width: 100%"
                placeholder="选择哈希算法"
              >
                <el-option
                  v-for="algo in hashAlgorithmOptions"
                  :key="algo.value"
                  :label="algo.label"
                  :value="algo.value"
                >
                  <div style="display: flex; flex-direction: column; gap: 4px; padding: 4px 0;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <span style="font-weight: 500;">{{ algo.label }}</span>
                      <el-tag size="small" type="info">{{ algo.speed }}</el-tag>
                      <el-tag size="small" :type="algo.security.includes('高') ? 'success' : 'danger'">{{ algo.security }}</el-tag>
                    </div>
                    <span style="font-size: 12px; color: #909399;">{{ algo.description }}</span>
                    <span style="font-size: 11px; color: #409EFF;">💡 {{ algo.useCase }}</span>
                  </div>
                </el-option>
              </el-select>
            </el-form-item>
            <el-alert
              v-if="scanConfig.hash_algorithm === 'md5'"
              title="安全警告"
              description="MD5 算法已被破解，存在哈希碰撞风险，不建议用于安全校验场景"
              type="warning"
              :closable="false"
              style="margin-top: 8px;"
            />
            <el-alert
              v-else-if="scanConfig.hash_algorithm === 'xxhash3' || scanConfig.hash_algorithm === 'xxhash64'"
              title="性能提示"
              description="XXH 系列算法专为高性能设计，是重复文件检测的最佳选择"
              type="success"
              :closable="false"
              style="margin-top: 8px;"
            />
          </el-card>

          <!-- 文件属性 -->
          <el-card shadow="never" style="margin-bottom: 16px;">
            <template #header>
              <span>文件属性</span>
            </template>
            <el-form-item>
              <el-checkbox v-model="scanConfig.exclude_hidden">
                <span>排除隐藏文件</span>
                <el-tooltip content="跳过以点开头的隐藏文件和目录">
                  <el-icon style="margin-left: 4px; color: #909399;"><InfoFilled /></el-icon>
                </el-tooltip>
              </el-checkbox>
            </el-form-item>
            <el-form-item>
              <el-checkbox v-model="scanConfig.exclude_system">
                <span>排除系统文件</span>
                <el-tooltip content="跳过系统保护的文件">
                  <el-icon style="margin-left: 4px; color: #909399;"><InfoFilled /></el-icon>
                </el-tooltip>
              </el-checkbox>
            </el-form-item>
          </el-card>
        </el-form>

        <!-- 底部操作按钮 -->
        <div class="config-drawer-footer">
          <el-button @click="resetConfig">恢复默认</el-button>
          <el-button type="primary" @click="saveConfig">保存配置</el-button>
        </div>
      </div>
    </el-drawer>

    <!-- 扫描进度 -->
    <el-card v-if="isScanning" class="progress-card" shadow="never">
      <div class="progress-header">
        <div class="progress-status">
          <el-icon v-if="isPaused" :size="20" color="#E6A23C"><VideoPause /></el-icon>
          <el-icon v-else :size="20" color="#409EFF" class="spinning"><Loading /></el-icon>
          <span>{{ isPaused ? '扫描已暂停' : scanStatus }}</span>
        </div>
        <div class="progress-time" v-if="scanStartTime">
          已用 {{ formatDuration(scanStartTime) }}
        </div>
      </div>

      <el-progress 
        :percentage="scanProgress" 
        :stroke-width="12"
        :status="isPaused ? 'warning' : ''"
        striped
        striped-flow
      />

      <!-- 当前扫描信息 -->
      <div class="progress-current-info">
        <div v-if="currentDirectory" class="current-directory">
          <el-icon><Folder /></el-icon>
          <span class="directory-label">当前目录:</span>
          <span class="directory-name">{{ currentDirectory }}</span>
        </div>
        <div v-if="scanCurrentFile" class="current-file">
          <el-icon><Document /></el-icon>
          <span class="file-label">正在扫描:</span>
          <span class="file-name">{{ scanCurrentFile }}</span>
        </div>
      </div>

      <!-- 统计信息 -->
      <div class="progress-stats">
        <div class="stat-item">
          <span class="stat-label">已扫描目录</span>
          <span class="stat-value">{{ scannedDirectories }}</span>
        </div>
        <div class="stat-item">
          <span class="stat-label">已扫描文件</span>
          <span class="stat-value">{{ scannedFiles }}</span>
        </div>
        <div class="stat-item">
          <span class="stat-label">总计文件</span>
          <span class="stat-value">{{ totalFiles || '计算中...' }}</span>
        </div>
        <div class="stat-item" v-if="estimatedTimeRemaining">
          <span class="stat-label">预计剩余</span>
          <span class="stat-value">{{ estimatedTimeRemaining }}</span>
        </div>
      </div>

      <!-- 最近扫描的文件列表 -->
      <div v-if="recentFiles.length > 0" class="recent-files-section">
        <div class="recent-files-header">
          <el-icon><Clock /></el-icon>
          <span>最近扫描的文件</span>
        </div>
        <div class="recent-files-list">
          <div 
            v-for="(file, index) in recentFiles.slice(0, 10)" 
            :key="index"
            class="recent-file-item"
            :class="{ 'is-latest': index === 0 }"
          >
            <el-icon :size="14"><Document /></el-icon>
            <span class="recent-file-name">{{ file }}</span>
          </div>
        </div>
      </div>

      <div class="progress-actions">
        <el-button v-if="!isPaused" type="warning" @click="pauseScan">
          <el-icon><VideoPause /></el-icon>
          暂停
        </el-button>
        <el-button v-else type="success" @click="resumeScan">
          <el-icon><VideoPlay /></el-icon>
          恢复
        </el-button>
        <el-button type="danger" plain @click="stopScan">
          <el-icon><Close /></el-icon>
          停止
        </el-button>
      </div>
    </el-card>

    <!-- 扫描操作 -->
    <div class="scan-actions-bar">
      <el-button 
        type="primary" 
        size="large"
        :loading="isScanning && !isPaused"
        :disabled="directories.length === 0 || (isScanning && isPaused)"
        @click="isScanning ? resumeScan() : startScan()"
        class="scan-btn"
      >
        <el-icon v-if="!isScanning"><Refresh /></el-icon>
        <el-icon v-else-if="isPaused"><VideoPlay /></el-icon>
        <el-icon v-else class="spinning"><Loading /></el-icon>
        <span>{{ isScanning ? (isPaused ? '恢复扫描' : '扫描中...') : '开始扫描' }}</span>
      </el-button>
    </div>

    <!-- 扫描历史抽屉 -->
    <el-drawer
      v-model="showHistoryPanel"
      title="扫描历史"
      size="650px"
      :append-to-body="true"
      :modal="true"
      :close-on-click-modal="true"
      destroy-on-close
    >
      <!-- 统计概览 -->
      <el-card v-if="historyStatistics" class="history-stats-card" shadow="never" style="margin-bottom: 16px;">
        <template #header>
          <div class="card-header">
            <span>统计概览</span>
            <el-button text size="small" @click="showHistoryStats = !showHistoryStats">
              {{ showHistoryStats ? '收起' : '展开' }}
            </el-button>
          </div>
        </template>
        <el-row :gutter="16" v-show="showHistoryStats">
          <el-col :span="6">
            <div class="stat-box">
              <div class="stat-number">{{ historyStatistics.total_scans || 0 }}</div>
              <div class="stat-label">总扫描次数</div>
            </div>
          </el-col>
          <el-col :span="6">
            <div class="stat-box">
              <div class="stat-number">{{ historyStatistics.completed_scans || 0 }}</div>
              <div class="stat-label">完成次数</div>
            </div>
          </el-col>
          <el-col :span="6">
            <div class="stat-box">
              <div class="stat-number">{{ formatFileSize(historyStatistics.total_wasted_space || 0) }}</div>
              <div class="stat-label">发现浪费空间</div>
            </div>
          </el-col>
          <el-col :span="6">
            <div class="stat-box">
              <div class="stat-number">{{ formatDurationSeconds(historyStatistics.average_scan_duration || 0) }}</div>
              <div class="stat-label">平均扫描时长</div>
            </div>
          </el-col>
        </el-row>
      </el-card>

      <!-- 搜索和筛选工具栏 -->
      <el-card shadow="never" style="margin-bottom: 16px;">
        <el-form :inline="true" class="history-search-form">
          <el-form-item label="关键词">
            <el-input
              v-model="historySearchKeyword"
              placeholder="搜索目录路径"
              clearable
              style="width: 150px"
              @keyup.enter="searchHistory"
            />
          </el-form-item>
          <el-form-item label="日期范围">
            <el-date-picker
              v-model="historyDateRange"
              type="daterange"
              range-separator="至"
              start-placeholder="开始日期"
              end-placeholder="结束日期"
              style="width: 220px"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
          <el-form-item label="状态">
            <el-select v-model="historyStatus" placeholder="全部状态" clearable style="width: 100px">
              <el-option label="完成" value="completed" />
              <el-option label="中断" value="interrupted" />
            </el-select>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="searchHistory">
              <el-icon><Search /></el-icon>
              搜索
            </el-button>
            <el-button @click="resetHistorySearch">重置</el-button>
          </el-form-item>
        </el-form>

        <!-- 排序选项 -->
        <div class="history-sort-bar">
          <span class="sort-label">排序方式</span>
          <el-radio-group v-model="historySortBy" size="small" @change="searchHistory">
            <el-radio-button label="scan_time">时间</el-radio-button>
            <el-radio-button label="total_files">文件数</el-radio-button>
            <el-radio-button label="duplicate_files">重复数</el-radio-button>
            <el-radio-button label="wasted_space">浪费空间</el-radio-button>
          </el-radio-group>
          <el-button
            text
            size="small"
            @click="toggleSort(historySortBy)"
            style="margin-left: 8px"
          >
            {{ historySortOrder === 'ASC' ? '升序 ↑' : '降序 ↓' }}
          </el-button>
        </div>
      </el-card>

      <!-- 批量操作工具栏 -->
      <div class="history-toolbar" style="margin-bottom: 16px;">
        <el-dropdown @command="handleClearCommand">
          <el-button type="danger" plain>
            <el-icon><Delete /></el-icon>
            清除历史
            <el-icon class="el-icon--right"><ArrowDown /></el-icon>
          </el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="all">清空所有历史</el-dropdown-item>
              <el-dropdown-item command="beforeDate">清除指定日期之前</el-dropdown-item>
              <el-dropdown-item command="keepRecent">保留最近N条</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
        <span class="history-count" style="margin-left: auto; color: #909399; font-size: 14px;">
          {{ historyTotal }} 条记录
        </span>
      </div>

      <!-- 历史列表 -->
      <div class="history-list" v-loading="historyLoading">
        <el-timeline>
          <el-timeline-item
            v-for="(item, index) in scanHistory"
            :key="index"
            :type="item.status === 'completed' ? 'success' : 'warning'"
            :icon="item.status === 'completed' ? Check : Warning"
            :timestamp="item.scan_time"
          >
            <el-card shadow="hover" class="history-item">
              <div class="history-header">
                <span class="history-mode">{{ item.scan_mode === 'incremental' ? '增量扫描' : '全量扫描' }}</span>
                <el-tag :type="item.status === 'completed' ? 'success' : 'warning'" size="small">
                  {{ item.status === 'completed' ? '完成' : '中断' }}
                </el-tag>
              </div>
              <div class="history-directories" v-if="item.directories && item.directories.length > 0">
                <el-icon><Folder /></el-icon>
                <span class="directory-text">{{ item.directories.join(', ') }}</span>
              </div>
              <div class="history-stats">
                <div class="history-stat">
                  <span class="stat-label">扫描文件</span>
                  <span class="stat-value">{{ item.total_files || 0 }} 个</span>
                </div>
                <div class="history-stat">
                  <span class="stat-label">重复组</span>
                  <span class="stat-value">{{ item.duplicate_groups || 0 }} 组</span>
                </div>
                <div class="history-stat">
                  <span class="stat-label">重复文件</span>
                  <span class="stat-value">{{ item.duplicate_files || 0 }} 个</span>
                </div>
                <div class="history-stat">
                  <span class="stat-label">浪费空间</span>
                  <span class="stat-value">{{ formatFileSize(item.wasted_space || 0) }}</span>
                </div>
                <div class="history-stat">
                  <span class="stat-label">耗时</span>
                  <span class="stat-value">{{ formatDurationSeconds(item.duration_seconds || 0) }}</span>
                </div>
              </div>
            </el-card>
          </el-timeline-item>
        </el-timeline>

        <el-empty v-if="scanHistory.length === 0 && !historyLoading" description="暂无扫描历史" />

        <!-- 分页 -->
        <el-pagination
          v-if="historyTotal > 0"
          v-model:current-page="historyPage"
          v-model:page-size="historyPageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="historyTotal"
          layout="total, sizes, prev, pager, next"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
          style="margin-top: 20px; justify-content: center;"
        />
      </div>
    </el-drawer>
  </div>
</template>

<style scoped>
.scan-center {
  width: 100%;
  min-width: 100%;
  max-width: 100%;
  height: 100%;
  box-sizing: border-box;
}

/* 确保所有直接子元素填满宽度 */
.scan-center > * {
  width: 100% !important;
  min-width: 100% !important;
  max-width: 100% !important;
  box-sizing: border-box;
}

/* 确保所有卡片填满宽度 */
.scan-center :deep(.el-card) {
  width: 100% !important;
  min-width: 100% !important;
  max-width: 100% !important;
}

/* 页面头部 */
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
}

.header-actions {
  display: flex;
  gap: 12px;
}

/* 扫描模式选择 */
.mode-card {
  margin-bottom: 20px;
}

.mode-selector {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 16px;
}

.mode-option {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px;
  border: 2px solid #e4e7ed;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.3s ease;
}

.mode-option:hover {
  border-color: #c6e2ff;
  background: #f5f7fa;
}

.mode-option.is-active {
  border-color: #409EFF;
  background: linear-gradient(135deg, #ecf5ff 0%, #e3f2fd 100%);
}

.mode-icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #ffffff;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  color: #409EFF;
}

.mode-option.is-active .mode-icon {
  background: #409EFF;
  color: #ffffff;
}

.mode-info {
  flex: 1;
}

.mode-info h4 {
  margin: 0 0 4px 0;
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.mode-info p {
  margin: 0;
  font-size: 13px;
  color: #909399;
}

/* 目录卡片 */
.directory-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
}

.directory-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 60px 20px;
  text-align: center;
}

.directory-empty p {
  margin: 16px 0 24px 0;
  color: #909399;
}

.directory-list {
  padding: 8px 0;
}

.directory-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  margin-bottom: 8px;
  background: #f5f7fa;
  border-radius: 8px;
  transition: all 0.2s ease;
}

.directory-item:hover {
  background: #e4e7ed;
}

.item-icon {
  color: #409EFF;
}

.item-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow: hidden;
}

.item-name {
  font-weight: 500;
  color: #303133;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-path {
  font-size: 12px;
  color: #909399;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-remove {
  opacity: 0;
  transition: opacity 0.2s;
}

.directory-item:hover .item-remove {
  opacity: 1;
}

.directory-actions {
  display: flex;
  gap: 12px;
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid #e4e7ed;
}

/* 配置卡片 */
.config-card {
  margin-bottom: 20px;
}

.config-form {
  padding: 8px 0;
}

.range-separator {
  margin: 0 12px;
  color: #909399;
}

/* 进度卡片 */
.progress-card {
  margin-bottom: 20px;
  background: linear-gradient(135deg, #f5f7fa 0%, #ffffff 100%);
}

.progress-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.progress-status {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 500;
  color: #303133;
}

.spinning {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.progress-time {
  font-size: 13px;
  color: #909399;
}

.progress-current-info {
  margin-top: 12px;
  padding: 12px;
  background: #ffffff;
  border-radius: 8px;
  border: 1px solid #e4e7ed;
}

.current-directory {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  font-size: 13px;
  color: #606266;
}

.current-directory:last-child {
  margin-bottom: 0;
}

.directory-label {
  color: #909399;
  font-weight: 500;
}

.directory-name {
  flex: 1;
  color: #409EFF;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.current-file {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: #606266;
}

.file-label {
  color: #909399;
  font-weight: 500;
}

.file-name {
  flex: 1;
  color: #67C23A;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 最近扫描的文件列表 */
.recent-files-section {
  margin-top: 16px;
  padding: 12px;
  background: #f5f7fa;
  border-radius: 8px;
  border: 1px solid #e4e7ed;
}

.recent-files-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
  font-size: 14px;
  font-weight: 600;
  color: #303133;
}

.recent-files-list {
  max-height: 200px;
  overflow-y: auto;
}

.recent-file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  margin-bottom: 4px;
  background: #ffffff;
  border-radius: 4px;
  font-size: 12px;
  color: #606266;
  transition: all 0.2s ease;
}

.recent-file-item:hover {
  background: #ecf5ff;
}

.recent-file-item.is-latest {
  background: #ecf5ff;
  border-left: 3px solid #409EFF;
  font-weight: 500;
  color: #409EFF;
}

.recent-file-name {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.progress-stats {
  display: flex;
  gap: 24px;
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid #e4e7ed;
}

.stat-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.stat-label {
  font-size: 12px;
  color: #909399;
}

.stat-value {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}

.progress-actions {
  display: flex;
  gap: 12px;
  margin-top: 16px;
}

/* 扫描操作 */
.scan-actions-bar {
  display: flex;
  gap: 12px;
  padding: 20px;
  background: #ffffff;
  border-radius: 12px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
}

.scan-btn {
  min-width: 160px;
}

/* 历史记录 */
.history-list {
  padding: 20px 0;
}

.history-item {
  margin-bottom: 8px;
}

.history-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.history-mode {
  font-weight: 600;
  color: #303133;
}

.history-stats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}

.history-stat {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

/* 过渡动画 */
.directory-enter-active,
.directory-leave-active {
  transition: all 0.3s ease;
}

.directory-enter-from,
.directory-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}

/* 响应式 */
@media (max-width: 768px) {
  .mode-selector {
    grid-template-columns: 1fr;
  }
  
  .scan-actions-bar {
    flex-wrap: wrap;
  }
  
  .scan-btn {
    width: 100%;
  }
}

/* USN 快速入口卡片 */
.usn-fast-entry {
  width: 100%;
  max-width: 500px;
  background: linear-gradient(135deg, rgba(64, 158, 255, 0.08) 0%, rgba(103, 194, 58, 0.04) 100%);
  border: 1px dashed rgba(64, 158, 255, 0.35);
  border-radius: 12px;
  padding: 16px 20px;
  display: flex;
  align-items: center;
  gap: 16px;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  margin-bottom: 20px;
  text-align: left;
}

.usn-fast-entry:hover {
  background: linear-gradient(135deg, rgba(64, 158, 255, 0.12) 0%, rgba(103, 194, 58, 0.08) 100%);
  border-color: #409EFF;
  box-shadow: 0 8px 24px rgba(64, 158, 255, 0.08);
  transform: translateY(-2px);
}

.usn-icon {
  font-size: 32px;
  color: #409EFF;
  background: rgba(255, 255, 255, 0.8);
  padding: 10px;
  border-radius: 10px;
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.12);
  animation: pulse 2s infinite;
}

.usn-info h3 {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 4px 0;
}

.usn-info p {
  font-size: 12px;
  color: #909399;
  margin: 0;
  line-height: 1.4;
}

.divider-line {
  display: flex;
  align-items: center;
  width: 100%;
  max-width: 300px;
  margin: 15px 0;
  color: #c0c4cc;
  font-size: 13px;
}

.divider-line::before,
.divider-line::after {
  content: '';
  flex: 1;
  height: 1px;
  background: #e4e7ed;
}

.divider-line span {
  padding: 0 10px;
}

@keyframes pulse {
  0% {
    box-shadow: 0 0 0 0 rgba(64, 158, 255, 0.3);
  }
  70% {
    box-shadow: 0 0 0 10px rgba(64, 158, 255, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(64, 158, 255, 0);
  }
}
</style>
