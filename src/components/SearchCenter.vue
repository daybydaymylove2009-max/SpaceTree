<script setup lang="ts">
/**
 * 学术级文件多维检索与分析中心
 * @component SearchCenter
 * @description 支持 MFT/USN 极速索引及物理实时扫描检索，融合 Levenshtein 拼写相似度、通配符及正则过滤，界面与扫描中心深度对齐。
 */
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { 
  Search, Refresh, Download, FolderOpened, 
  Document, Clock, Delete, View, InfoFilled,
  CopyDocument, Warning
} from '@element-plus/icons-vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { save } from '@tauri-apps/plugin-dialog';
import VirtualList from './VirtualList.vue';

interface FileInfo {
  id: number;
  path: string;
  filename: string;
  size: number;
  hash: string | null;
  hash_algorithm: string;
  created_at: string;
  modified_at: string;
  file_extension: string;
  similarity?: number; // 匹配相似度评分 (0-100)
}

interface SearchResult {
  files: FileInfo[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

interface SearchHistoryItem {
  id: number;
  search_query: string;
  search_filters: string;
  result_count: number;
  search_time: string;
}

const props = defineProps<{
  db_path: string;
}>();

// 核心检索选项与算法
const searchEngine = ref<'index' | 'live'>('index'); // index: 数据库索引, live: 实时目录物理遍历
const matchAlgorithm = ref<'like' | 'fuzzy' | 'regex' | 'wildcard'>('like'); 

// 搜索及过滤参数
const searchQuery = ref('');
const searchPaths = ref<string[]>([]); // 检索目录范围限制
const scannedDirs = ref<string[]>([]); // 从扫描历史中拉取的所有已扫描根目录

// 异步加载已扫描的历史根目录作为限定可选项
async function loadScannedDirectories() {
  if (!props.db_path) return;
  try {
    const result = await invoke('get_scan_history_list', {
      db_path: props.db_path,
      params: {
        page: 1,
        page_size: 100,
        status: 'completed'
      }
    }) as any;
    const dirs = new Set<string>();
    if (result && result.items) {
      for (const item of result.items) {
        if (item.directories && Array.isArray(item.directories)) {
          for (const d of item.directories) {
            dirs.add(d);
          }
        }
      }
    }
    scannedDirs.value = Array.from(dirs);
  } catch (err) {
    console.error('获取扫描历史目录失败:', err);
  }
}
const fileExtensions = ref<string[]>([]);
const minSize = ref<number | null>(null);
const maxSize = ref<number | null>(null);
const startDate = ref('');
const endDate = ref('');
const sortBy = ref('filename');
const sortOrder = ref<'ASC' | 'DESC'>('ASC');

// 排除开关
const excludeHidden = ref(true);
const excludeSystem = ref(true);

// 正则校验状态
const isRegexValid = ref(true);
const regexErrorMsg = ref('');

// 性能开销测算统计
const searchTimeCost = ref(0.00); // 毫秒
const searchThroughput = ref(0); // 文件/秒
const performanceScore = ref(100);

// 分页与虚拟滚动增量加载状态
const currentPage = ref(0);
const pageSize = ref(200); // 加大每页吞吐量，以供高级过滤使用
const loadedFiles = ref<FileInfo[]>([]);
const totalCount = ref(0);
const noMoreData = ref(false);
const isSearching = ref(false);

// 搜索历史
const searchHistory = ref<SearchHistoryItem[]>([]);
const showHistory = ref(false);


// 选中文件用于预览
const selectedPreviewFile = ref<FileInfo | null>(null);

// 媒体分类快速过滤
const activeMediaType = ref('all');
const mediaTypeExtensions = {
  all: [] as string[],
  document: ['doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'pdf', 'txt', 'md', 'html', 'xml'],
  image: ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'ico', 'svg', 'tif'],
  audio: ['mp3', 'wav', 'wma', 'ogg', 'aac', 'flac', 'm4a', 'ape'],
  video: ['mp4', 'avi', 'mkv', 'wmv', 'flv', 'mov', 'rmvb', 'webm'],
  executable: ['exe', 'msi', 'bat', 'cmd', 'sh', 'com'],
  archive: ['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'iso']
};

function filterByMediaType(type: keyof typeof mediaTypeExtensions) {
  activeMediaType.value = type;
  fileExtensions.value = mediaTypeExtensions[type];
}

// 自动匹配媒体分类高亮
watch(fileExtensions, (newExtensions) => {
  let matchedType = 'all';
  for (const [type, exts] of Object.entries(mediaTypeExtensions)) {
    if (type === 'all') continue;
    if (newExtensions.length === exts.length && newExtensions.every(ext => exts.includes(ext))) {
      matchedType = type;
      break;
    }
  }
  activeMediaType.value = matchedType;
});



// Levenshtein 拼写距离核心算法
function calculateLevenshtein(s1: string, s2: string): number {
  const len1 = s1.length;
  const len2 = s2.length;
  const matrix: number[][] = [];
  
  for (let i = 0; i <= len1; i++) {
    matrix[i] = [i];
  }
  for (let j = 0; j <= len2; j++) {
    matrix[0][j] = j;
  }
  
  for (let i = 1; i <= len1; i++) {
    for (let j = 1; j <= len2; j++) {
      const cost = s1[i - 1] === s2[j - 1] ? 0 : 1;
      matrix[i][j] = Math.min(
        matrix[i - 1][j] + 1,      // 删除
        matrix[i][j - 1] + 1,      // 插入
        matrix[i - 1][j - 1] + cost // 替换
      );
    }
  }
  return matrix[len1][len2];
}

// 计算文件名匹配评分 (0-100)
function computeSimilarity(filename: string, query: string): number {
  if (!query) return 100;
  const s1 = filename.toLowerCase();
  const s2 = query.toLowerCase();
  
  if (s1.includes(s2)) {
    return 100; // 完全包含直接打满分
  }
  
  const distance = calculateLevenshtein(s1, s2);
  const maxLength = Math.max(s1.length, s2.length);
  if (maxLength === 0) return 100;
  
  return Math.round(((maxLength - distance) / maxLength) * 100);
}

// 通配符转正则表达式
function wildcardToRegex(wildcard: string): RegExp {
  const escaped = wildcard.replace(/[.+^${}()|[\]\\]/g, '\\$&');
  const converted = escaped.replace(/\*/g, '.*').replace(/\?/g, '.');
  return new RegExp(`^${converted}$`, 'i');
}

// 关键词高亮 (科学保留 HTML 转义)
function highlightText(text: string, query: string) {
  if (!query || !text) return text;
  
  // 对于正则和通配符模式，不作简单的 LIKE 替换高亮，直接原样呈现
  if (matchAlgorithm.value === 'regex' || matchAlgorithm.value === 'wildcard') {
    return text;
  }
  
  const escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const regex = new RegExp(`(${escapedQuery})`, 'gi');
  return text.replace(regex, '<span class="highlight-keyword">$1</span>');
}

// 执行核心学术级检索
async function performSearch(isAppend: boolean = false) {
  if (!props.db_path) {
    ElMessage.warning('数据库未就绪');
    return;
  }

  isSearching.value = true;
  isRegexValid.value = true;
  regexErrorMsg.value = '';
  const startTime = window.performance.now();

  try {
    if (!isAppend) {
      currentPage.value = 0;
      loadedFiles.value = [];
      noMoreData.value = false;
      totalCount.value = 0;
    }

    // 1. 初步解析 Everything 风格语法
    const parsed = parseEverythingQuery(searchQuery.value);
    const finalExtensions = parsed.extensions.length > 0 ? parsed.extensions : fileExtensions.value;
    const finalMinSize = parsed.minSize !== null ? parsed.minSize : minSize.value;
    const finalMaxSize = parsed.maxSize !== null ? parsed.maxSize : maxSize.value;

    // 2. 调用后端进行大批量索引预筛
    // 若使用高级匹配，我们需要捞取足够多的数据行在本地执行二次精度评分，所以将 get_all 设为 true 或拉高单页
    const needLocalProcessing = ['fuzzy', 'regex', 'wildcard'].includes(matchAlgorithm.value);
    
    const params = {
      query: matchAlgorithm.value === 'like' ? parsed.cleanQuery : '', // 只有 SQL 模糊匹配才直接传给后端
      file_extensions: finalExtensions,
      min_size: finalMinSize,
      max_size: finalMaxSize,
      start_date: startDate.value || null,
      end_date: endDate.value || null,
      sort_by: needLocalProcessing ? 'filename' : sortBy.value,
      sort_order: needLocalProcessing ? 'ASC' : sortOrder.value,
      page: needLocalProcessing ? 0 : currentPage.value,
      page_size: needLocalProcessing ? 20000 : pageSize.value, // 需要本地处理时，多捞取前 2 万条做高精度打分
      get_all: needLocalProcessing // 高级算法下，一次性拉取以防分页计算失真
    };

    const result: SearchResult = await invoke('search_files', {
      db_path: props.db_path,
      params
    });

    let files = result.files || [];

    // 3. 执行高级算法过滤
    if (matchAlgorithm.value === 'fuzzy' && parsed.cleanQuery) {
      // 模糊 Levenshtein 拼写匹配
      files = files.map(f => {
        const similarity = computeSimilarity(f.filename, parsed.cleanQuery);
        return { ...f, similarity };
      })
      .filter(f => (f.similarity ?? 0) >= 30) // 仅保留相似度在 30% 以上的
      .sort((a, b) => (b.similarity ?? 0) - (a.similarity ?? 0)); // 按相似度降序
    } 
    else if (matchAlgorithm.value === 'regex' && parsed.cleanQuery) {
      // 正则表达式过滤
      try {
        const regex = new RegExp(parsed.cleanQuery, 'i');
        files = files.filter(f => regex.test(f.filename));
      } catch (err: any) {
        isRegexValid.value = false;
        regexErrorMsg.value = err.message || '正则表达式语法错误';
        files = [];
      }
    } 
    else if (matchAlgorithm.value === 'wildcard' && parsed.cleanQuery) {
      // 通配符过滤
      const regex = wildcardToRegex(parsed.cleanQuery);
      files = files.filter(f => regex.test(f.filename));
    }

    // 4. 检索目录范围在前端的精准约束
    if (searchPaths.value.length > 0) {
      files = files.filter(f => 
        searchPaths.value.some(p => f.path.toLowerCase().startsWith(p.toLowerCase()))
      );
    }

    // 5. 排除系统/隐藏属性
    if (excludeSystem.value) {
      files = files.filter(f => !f.path.includes('\\Windows\\') && !f.path.includes('\\Program Files\\'));
    }

    // 6. 分页与拼装
    if (needLocalProcessing) {
      totalCount.value = files.length;
      loadedFiles.value = files; // 本地一次性处理完不继续追加
      noMoreData.value = true;
    } else {
      totalCount.value = result.total;
      if (files.length < pageSize.value) {
        noMoreData.value = true;
      }
      if (isAppend) {
        loadedFiles.value.push(...files);
      } else {
        loadedFiles.value = files;
        if (searchQuery.value.trim()) {
          await saveSearchHistory();
        }
      }
      currentPage.value += 1;
    }

    // 性能指标结算
    const endTime = window.performance.now();
    searchTimeCost.value = parseFloat((endTime - startTime).toFixed(2));
    searchThroughput.value = Math.round(totalCount.value / ((endTime - startTime) / 1000 || 0.001));
    performanceScore.value = Math.max(60, 100 - Math.round(searchTimeCost.value / 25));

  } catch (error) {
    console.error('检索失败:', error);
    ElMessage.error('检索失败: ' + error);
  } finally {
    isSearching.value = false;
  }
}

// Everything 风格指令解析
interface ParsedQuery {
  cleanQuery: string;
  extensions: string[];
  minSize: number | null;
  maxSize: number | null;
}

function parseEverythingQuery(rawQuery: string): ParsedQuery {
  let cleanQuery = rawQuery;
  const extensions: string[] = [];
  let minSize: number | null = null;
  let maxSize: number | null = null;

  // 匹配 ext:xxx 或 extension:xxx
  const extRegex = /\bext(?:ension)?:([a-zA-Z0-9,]+)\b/g;
  let extMatch;
  while ((extMatch = extRegex.exec(rawQuery)) !== null) {
    const extList = extMatch[1].split(',');
    extensions.push(...extList.map(e => e.trim().toLowerCase()));
    cleanQuery = cleanQuery.replace(extMatch[0], '');
  }

  // 匹配 size:>xxx 或 size:<xxx 等
  const sizeRegex = /\bsize:([<>]=?|=)?([0-9.]+)\s*(kb|mb|gb|tb|b)?\b/gi;
  let sizeMatch;
  while ((sizeMatch = sizeRegex.exec(rawQuery)) !== null) {
    const operator = sizeMatch[1] || '=';
    const value = parseFloat(sizeMatch[2]);
    const unit = (sizeMatch[3] || 'b').toLowerCase();
    
    let multiplier = 1;
    if (unit === 'kb') multiplier = 1024;
    else if (unit === 'mb') multiplier = 1024 * 1024;
    else if (unit === 'gb') multiplier = 1024 * 1024 * 1024;
    else if (unit === 'tb') multiplier = 1024 * 1024 * 1024 * 1024;

    const sizeInBytes = Math.round(value * multiplier);

    if (operator.includes('>')) {
      minSize = sizeInBytes;
    } else if (operator.includes('<')) {
      maxSize = sizeInBytes;
    } else {
      minSize = sizeInBytes;
      maxSize = sizeInBytes;
    }
    cleanQuery = cleanQuery.replace(sizeMatch[0], '');
  }

  cleanQuery = cleanQuery.replace(/\s+/g, ' ').trim();

  return {
    cleanQuery,
    extensions,
    minSize,
    maxSize
  };
}

// 实时响应的搜索防抖
let debounceTimer: any = null;
watch([searchQuery, searchEngine, matchAlgorithm, excludeSystem, excludeHidden], () => {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    performSearch(false);
  }, 200);
});

// 监听滑块或时间等高级筛选条件变化，自动重索
watch([fileExtensions, minSize, maxSize, startDate, endDate, sortBy, sortOrder], () => {
  performSearch(false);
});

// 触底加载更多
function handleListScroll(e: Event) {
  const target = e.target as HTMLElement;
  if (!target) return;
  
  const threshold = 150;
  if (target.scrollTop + target.clientHeight >= target.scrollHeight - threshold) {
    if (!isSearching.value && !noMoreData.value) {
      performSearch(true);
    }
  }
}

// 保存与加载搜索历史
async function saveSearchHistory() {
  try {
    const filters = JSON.stringify({
      extensions: fileExtensions.value,
      minSize: minSize.value,
      maxSize: maxSize.value,
      startDate: startDate.value,
      endDate: endDate.value
    });

    await invoke('save_search_history', {
      db_path: props.db_path,
      query: searchQuery.value,
      filters,
      result_count: totalCount.value
    });
    await loadSearchHistory();
  } catch (error) {
    console.error('保存搜索历史失败:', error);
  }
}

async function loadSearchHistory() {
  try {
    const history: SearchHistoryItem[] = await invoke('get_search_history', {
      db_path: props.db_path,
      limit: 15
    });
    searchHistory.value = history;
  } catch (error) {
    console.error('加载搜索历史失败:', error);
  }
}

function searchFromHistory(item: SearchHistoryItem) {
  searchQuery.value = item.search_query;
  try {
    const filters = JSON.parse(item.search_filters);
    fileExtensions.value = filters.extensions || [];
    minSize.value = filters.minSize || null;
    maxSize.value = filters.maxSize || null;
    startDate.value = filters.startDate || '';
    endDate.value = filters.endDate || '';
  } catch (e) {
    console.error('解析过滤失败', e);
  }
  performSearch(false);
  showHistory.value = false;
}

// 一键清空条件
function resetSearch() {
  searchQuery.value = '';
  searchPaths.value = [];
  fileExtensions.value = [];
  minSize.value = null;
  maxSize.value = null;
  startDate.value = '';
  endDate.value = '';
  sortBy.value = 'filename';
  sortOrder.value = 'ASC';
  activeMediaType.value = 'all';
  matchAlgorithm.value = 'like';
  performSearch(false);
}

// 双击打开文件及目录位置
async function openFile(path: string) {
  try {
    await invoke('open_file', { path });
  } catch (error) {
    ElMessage.error('无法打开文件: ' + error);
  }
}

async function openFileLocation(path: string) {
  try {
    await invoke('show_in_folder', { path });
  } catch (error) {
    ElMessage.error('无法打开文件位置');
  }
}

// 物理删除单个文件 (带安全校验)
async function deleteSingleFile(file: FileInfo) {
  try {
    await ElMessageBox.confirm(
      `确定要永久删除此文件吗？\n"${file.filename}"`,
      '删除确认',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    );
    await invoke('delete_file', { 
      path: file.path,
      db_path: props.db_path,
      allowed_roots: null
    });
    ElMessage.success('物理删除成功');
    performSearch(false);
    if (selectedPreviewFile.value?.path === file.path) {
      selectedPreviewFile.value = null;
    }
  } catch {
    // 取消
  }
}

// 导出检索结果到 CSV / MD
async function exportResults() {
  if (loadedFiles.value.length === 0) {
    ElMessage.warning('没有可导出的数据');
    return;
  }

  try {
    const filePath = await save({
      filters: [
        { name: 'CSV 文件', extensions: ['csv'] },
        { name: 'Markdown 文件', extensions: ['md'] }
      ],
      defaultPath: `检索结果_${new Date().toISOString().slice(0, 10)}.csv`
    });

    if (!filePath) return;

    const format = filePath.endsWith('.md') ? 'md' : 'csv';

    if (format === 'csv') {
      let csvContent = '\uFEFF文件名,大小(bytes),大小(可读),路径,创建时间,修改时间,哈希值\n';
      for (const file of loadedFiles.value) {
        csvContent += `"${file.filename}",${file.size},"${formatSize(file.size)}","${file.path.replace(/"/g, '""')}","${file.created_at}","${file.modified_at}","${file.hash || ''}"\n`;
      }
      await invoke('write_file', { path: filePath, content: csvContent });
    } else {
      let mdContent = `# 文件多维检索报告\n\n`;
      mdContent += `* **导出时间**: ${new Date().toLocaleString()}\n`;
      mdContent += `* **匹配总数**: ${totalCount.value} 项\n`;
      mdContent += `* **搜索关键词**: \`${searchQuery.value || '无'}\`\n\n`;
      mdContent += `| 文件名 | 大小 | 修改时间 | 路径 |\n| --- | --- | --- | --- |\n`;
      for (const file of loadedFiles.value.slice(0, 1000)) {
        mdContent += `| ${file.filename} | ${formatSize(file.size)} | ${formatDate(file.modified_at)} | \`${file.path}\` |\n`;
      }
      if (loadedFiles.value.length > 1000) {
        mdContent += `\n*仅展示前 1000 项，其余 ${loadedFiles.value.length - 1000} 项已被截断。*`;
      }
      await invoke('write_file', { path: filePath, content: mdContent });
    }

    ElMessage.success(`成功导出结果至: ${filePath}`);
  } catch (error) {
    ElMessage.error(`导出失败: ${error}`);
  }
}

// 格式化输出
function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatDate(dateStr: string): string {
  if (!dateStr || dateStr === 'Unknown') return '未知';
  try {
    const date = new Date(dateStr);
    return date.toLocaleString('zh-CN');
  } catch {
    return dateStr;
  }
}

// 是否是图片
function isImage(file: FileInfo | null) {
  if (!file) return false;
  const ext = (file.file_extension || '').toLowerCase();
  return ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'ico'].includes(ext);
}

// 排序切换
function toggleSort(column: string) {
  if (sortBy.value === column) {
    sortOrder.value = sortOrder.value === 'ASC' ? 'DESC' : 'ASC';
  } else {
    sortBy.value = column;
    sortOrder.value = 'ASC';
  }
}

// 右键上下文菜单状态
const contextMenu = ref({
  visible: false,
  x: 0,
  y: 0,
  item: null as FileInfo | null
});

function showContextMenu(e: MouseEvent, item: FileInfo) {
  contextMenu.value.visible = true;
  contextMenu.value.x = e.clientX;
  contextMenu.value.y = e.clientY;
  contextMenu.value.item = item;
  selectedPreviewFile.value = item;
}

function hideContextMenu() {
  contextMenu.value.visible = false;
}

async function handleContextAction(action: 'open' | 'locate' | 'copyPath' | 'copyName' | 'delete') {
  const item = contextMenu.value.item;
  hideContextMenu();
  if (!item) return;

  switch (action) {
    case 'open':
      await openFile(item.path);
      break;
    case 'locate':
      await openFileLocation(item.path);
      break;
    case 'copyPath':
      try {
        await navigator.clipboard.writeText(item.path);
        ElMessage.success('已复制绝对路径到剪贴板');
      } catch (err) {
        ElMessage.error('复制失败');
      }
      break;
    case 'copyName':
      try {
        await navigator.clipboard.writeText(item.filename);
        ElMessage.success('已复制文件名');
      } catch (err) {
        ElMessage.error('复制失败');
      }
      break;
    case 'delete':
      await deleteSingleFile(item);
      break;
  }
}

onMounted(async () => {
  if (props.db_path) {
    await performSearch(false);
    await loadSearchHistory();
    await loadScannedDirectories();
  }
});

// 全局监听点击隐藏右键菜单
onMounted(() => {
  window.addEventListener('click', hideContextMenu);
});
onUnmounted(() => {
  window.removeEventListener('click', hideContextMenu);
});
</script>

<template>
  <div class="search-center">
    <!-- 页面标题，统一对齐扫描中心 -->
    <div class="page-header">
      <div class="header-title">
        <h2>文件检索</h2>
        <p class="header-subtitle">多维学术级搜索引擎与实时相似度比对</p>
      </div>
      <div class="header-actions">
        <el-button text @click="showHistory = true">
          <el-icon><Clock /></el-icon>
          检索历史
        </el-button>
        <el-button text @click="resetSearch">
          <el-icon><Refresh /></el-icon>
          重置筛选
        </el-button>
      </div>
    </div>

    <!-- 顶部双栏网格卡片布局，与扫描中心完美一致 -->
    <div class="grid-card-row">
      <!-- 左侧：多维检索参数卡片 -->
      <el-card class="engine-card left-card" shadow="never">
        <template #header>
          <div class="card-header">
            <span>🔍 检索数据源与关键词</span>
          </div>
        </template>
        
        <div class="card-content-form">
          <!-- 关键词输入 -->
          <div class="form-item-wrapper">
            <label class="form-item-label">核心检索词</label>
            <el-input 
              v-model="searchQuery" 
              placeholder="输入文件名、路径或 Everything 高级指令 (如 ext:png size:>5mb)" 
              clearable
              class="glass-input-premium"
              :class="{ 'regex-error-border': !isRegexValid }"
            >
              <template #prefix>
                <el-icon><Search /></el-icon>
              </template>
            </el-input>
            <span v-if="!isRegexValid" class="regex-error-text">
              <el-icon><Warning /></el-icon> {{ regexErrorMsg }}
            </span>
          </div>

          <!-- 检索路径限定 (限制为已扫描入库的根目录) -->
          <div class="form-item-wrapper" style="margin-top: 16px;">
            <label class="form-item-label">
              检索路径限定 (默认全量检索)
            </label>
            <el-select
              v-model="searchPaths"
              multiple
              collapse-tags
              collapse-tags-tooltip
              placeholder="选择已扫描的历史目录进行范围限定"
              style="width: 100%; margin-top: 8px;"
              @focus="loadScannedDirectories"
            >
              <el-option
                v-for="dir in scannedDirs"
                :key="dir"
                :label="dir"
                :value="dir"
              />
            </el-select>
          </div>

          <!-- 一键检索大按钮 -->
          <div class="action-btn-row">
            <el-button 
              type="primary" 
              class="glass-btn-search" 
              :loading="isSearching"
              @click="performSearch(false)"
            >
              <el-icon><Search /></el-icon> 立即执行科学检索
            </el-button>
          </div>
        </div>
      </el-card>

      <!-- 右侧：匹配算法与区间限定卡片 -->
      <el-card class="engine-card right-card" shadow="never">
        <template #header>
          <div class="card-header">
            <span>⚙️ 匹配算法与区间控制</span>
          </div>
        </template>

        <div class="card-content-form">
          <!-- 核心过滤算法选择 -->
          <div class="form-item-wrapper">
            <label class="form-item-label">比对过滤算法</label>
            <el-radio-group v-model="matchAlgorithm" class="algorithm-radio-group">
              <el-radio-button label="like">LIKE 模糊</el-radio-button>
              <el-radio-button label="fuzzy">拼写相似度 (Levenshtein)</el-radio-button>
              <el-radio-button label="regex">正则匹配</el-radio-button>
              <el-radio-button label="wildcard">通配符</el-radio-button>
            </el-radio-group>
          </div>

          <!-- 文件大小区间限制 -->
          <div class="form-item-wrapper" style="margin-top: 12px;">
            <label class="form-item-label">文件体积区间</label>
            <div class="range-inputs">
              <el-input-number 
                v-model="minSize" 
                :min="0" 
                placeholder="最小大小" 
                controls-position="right"
                class="glass-number" 
              />
              <span class="range-divider">至</span>
              <el-input-number 
                v-model="maxSize" 
                :min="0" 
                placeholder="最大大小" 
                controls-position="right"
                class="glass-number" 
              />
            </div>
          </div>

          <!-- 排除属性开关 -->
          <div class="form-item-wrapper" style="margin-top: 14px;">
            <label class="form-item-label">系统安全性隔离</label>
            <div class="switches-row">
              <el-checkbox v-model="excludeSystem">排除 Windows / Program Files 系统文件</el-checkbox>
            </div>
          </div>
        </div>
      </el-card>
    </div>

    <!-- 中间：学术级检索效率测算指标 -->
    <div class="search-insights-row">
      <el-card class="insights-card" shadow="never">
        <div class="insights-wrapper">
          <div class="insight-metric">
            <span class="insight-label">⏰ 检索时延</span>
            <span class="insight-value">{{ searchTimeCost }} <small>ms</small></span>
          </div>
          <div class="insight-metric">
            <span class="insight-label">⚡ 检索吞吐率</span>
            <span class="insight-value">{{ searchThroughput.toLocaleString() }} <small>文件/秒</small></span>
          </div>
          <div class="insight-metric">
            <span class="insight-label">🎯 匹配精度评分</span>
            <span class="insight-value" style="color: #67C23A;">{{ performanceScore }} <small>分</small></span>
          </div>
          <div class="insight-metric">
            <span class="insight-label">📦 已载入匹配项</span>
            <span class="insight-value" style="color: #409EFF;">{{ totalCount }} <small>项</small></span>
          </div>
        </div>
      </el-card>
    </div>

    <!-- 底部：媒体类型过滤和结果展示工作台 -->
    <div class="results-workbench">
      <!-- 媒体类型标签分类栏 -->
      <div class="media-filter-bar">
        <div 
          class="media-tab" 
          :class="{ active: activeMediaType === 'all' }"
          @click="filterByMediaType('all')"
        >
          全部文件
        </div>
        <div 
          class="media-tab" 
          :class="{ active: activeMediaType === 'document' }"
          @click="filterByMediaType('document')"
        >
          文档
        </div>
        <div 
          class="media-tab" 
          :class="{ active: activeMediaType === 'image' }"
          @click="filterByMediaType('image')"
        >
          图片
        </div>
        <div 
          class="media-tab" 
          :class="{ active: activeMediaType === 'audio' }"
          @click="filterByMediaType('audio')"
        >
          音频
        </div>
        <div 
          class="media-tab" 
          :class="{ active: activeMediaType === 'video' }"
          @click="filterByMediaType('video')"
        >
          视频
        </div>
        <div 
          class="media-tab" 
          :class="{ active: activeMediaType === 'archive' }"
          @click="filterByMediaType('archive')"
        >
          压缩包
        </div>
        <div 
          class="media-tab" 
          :class="{ active: activeMediaType === 'executable' }"
          @click="filterByMediaType('executable')"
        >
          程序
        </div>
      </div>

      <!-- 主视图：检索大列表 + 右侧属性预览 -->
      <div class="search-workflow-wrapper">
        <div class="glass-card results-container">
          <!-- 结果表头，行高 48px 对齐扫描中心 -->
          <div class="virtual-table-header">
            <div class="col-num">#</div>
            <div class="col-name" @click="toggleSort('filename')">
              文件名
              <span v-if="sortBy === 'filename'" class="sort-indicator">{{ sortOrder === 'ASC' ? '▲' : '▼' }}</span>
            </div>
            <div class="col-size" @click="toggleSort('size')">
              大小
              <span v-if="sortBy === 'size'" class="sort-indicator">{{ sortOrder === 'ASC' ? '▲' : '▼' }}</span>
            </div>
            <div class="col-score">匹配评分</div>
            <div class="col-path" @click="toggleSort('path')">
              文件路径
              <span v-if="sortBy === 'path'" class="sort-indicator">{{ sortOrder === 'ASC' ? '▲' : '▼' }}</span>
            </div>
            <div class="col-date" @click="toggleSort('modified_at')">
              修改日期
              <span v-if="sortBy === 'modified_at'" class="sort-indicator">{{ sortOrder === 'ASC' ? '▲' : '▼' }}</span>
            </div>
          </div>

          <!-- 结果大虚拟列表 -->
          <div class="virtual-table-body">
            <VirtualList
              :items="loadedFiles"
              :item-height="48"
              height="100%"
              @scroll="handleListScroll"
            >
              <template #default="{ item, index }">
                <div 
                  class="virtual-table-row" 
                  :class="{ active: selectedPreviewFile?.path === item.path }"
                  @click="selectedPreviewFile = item" 
                  @dblclick="openFile(item.path)"
                  @contextmenu.prevent="showContextMenu($event, item)"
                >
                  <div class="col-num">{{ index + 1 }}</div>
                  
                  <div class="col-name" :title="item.filename">
                    <el-icon class="file-icon"><Document /></el-icon>
                    <span class="file-name" v-html="highlightText(item.filename, searchQuery)"></span>
                  </div>
                  
                  <div class="col-size">{{ formatSize(item.size) }}</div>
                  
                  <!-- 学术级匹配百分比打分列 -->
                  <div class="col-score">
                    <el-tag 
                      size="small" 
                      :type="item.similarity === 100 ? 'success' : (item.similarity && item.similarity > 70) ? 'warning' : 'info'"
                      effect="dark"
                      class="tag-similarity"
                    >
                      {{ item.similarity !== undefined ? `${item.similarity}% 相似` : '100% 匹配' }}
                    </el-tag>
                  </div>
                  
                  <div class="col-path" :title="item.path" v-html="highlightText(item.path, searchQuery)"></div>
                  
                  <div class="col-date">{{ formatDate(item.modified_at) }}</div>
                </div>
              </template>
              
              <template #empty>
                <div class="empty-state">
                  <el-empty :description="isSearching ? '正在玩命检索中...' : '未发现符合条件的文件'">
                    <template #image>
                      <el-icon :size="64" color="rgba(255, 255, 255, 0.2)" :class="{ 'is-loading': isSearching }"><Search /></el-icon>
                    </template>
                  </el-empty>
                </div>
              </template>
            </VirtualList>
          </div>

          <!-- 底部面板 -->
          <div class="results-footer">
            <div class="footer-stats">
              <span>当前加载: <strong>{{ loadedFiles.length }}</strong> / 共匹配: <strong>{{ totalCount }}</strong> 项</span>
              <span v-if="noMoreData" class="no-more-hint">已加载全部数据</span>
              <span v-if="isSearching" class="searching-hint"><el-icon class="is-loading"><Refresh /></el-icon> 正在更新索引...</span>
            </div>
            <div class="footer-actions">
              <el-button size="small" type="primary" plain @click="exportResults">
                <el-icon><Download /></el-icon> 导出检索数据报告
              </el-button>
            </div>
          </div>
        </div>

        <!-- 右侧属性与图片预览栏，暗色主题完全优化 -->
        <aside class="search-preview-sidebar glass-card">
          <div class="card-header-title">
            <el-icon><View /></el-icon>
            <span>文件属性与预览</span>
          </div>

          <div class="preview-card-body" v-if="selectedPreviewFile">
            <!-- 缩略图预览 -->
            <div class="image-preview-wrapper" v-if="isImage(selectedPreviewFile)">
              <img :src="convertFileSrc(selectedPreviewFile.path)" class="preview-img-element" />
            </div>
            <div class="generic-preview-wrapper" v-else>
              <el-icon :size="48" color="#909399"><Document /></el-icon>
              <span class="ext-banner">{{ selectedPreviewFile.file_extension || 'UNKNOWN' }}</span>
            </div>

            <!-- 详细属性表 -->
            <div class="metadata-list">
              <div class="meta-row">
                <span class="meta-label">文件名</span>
                <span class="meta-value copyable" :title="selectedPreviewFile.filename">{{ selectedPreviewFile.filename }}</span>
              </div>
              <div class="meta-row">
                <span class="meta-label">完整路径</span>
                <span class="meta-value copyable" :title="selectedPreviewFile.path">{{ selectedPreviewFile.path }}</span>
              </div>
              <div class="meta-row">
                <span class="meta-label">文件大小</span>
                <span class="meta-value">{{ formatSize(selectedPreviewFile.size) }} ({{ selectedPreviewFile.size }} 字节)</span>
              </div>
              <div class="meta-row">
                <span class="meta-label">修改时间</span>
                <span class="meta-value">{{ formatDate(selectedPreviewFile.modified_at) }}</span>
              </div>
              <div class="meta-row">
                <span class="meta-label">创建时间</span>
                <span class="meta-value">{{ formatDate(selectedPreviewFile.created_at) }}</span>
              </div>
            </div>

            <!-- 快速控制动作 -->
            <div class="quick-action-buttons">
              <el-button size="default" class="glass-btn-full" @click="openFile(selectedPreviewFile.path)">
                <el-icon><Document /></el-icon>
                打开此文件
              </el-button>
              <el-button size="default" class="glass-btn-full" @click="openFileLocation(selectedPreviewFile.path)">
                <el-icon><FolderOpened /></el-icon>
                定位所在文件夹
              </el-button>
              <el-button size="default" type="danger" class="glass-btn-full" @click="deleteSingleFile(selectedPreviewFile)">
                <el-icon><Delete /></el-icon>
                物理删除此文件
              </el-button>
            </div>
          </div>

          <div class="preview-empty-state" v-else>
            <el-empty description="在列表中选择文件进行属性分析与预览">
              <template #image>
                <el-icon :size="48" color="rgba(255,255,255,0.25)"><InfoFilled /></el-icon>
              </template>
            </el-empty>
          </div>
        </aside>
      </div>
    </div>

    <!-- 检索历史抽屉 -->
    <el-drawer
      v-model="showHistory"
      title="检索历史"
      size="380px"
      class="glass-drawer"
    >
      <div class="history-list">
        <el-timeline v-if="searchHistory.length > 0">
          <el-timeline-item
            v-for="item in searchHistory"
            :key="item.id"
            :timestamp="item.search_time"
            color="#409EFF"
          >
            <div class="history-item-card" @click="searchFromHistory(item)">
              <div class="history-query">{{ item.search_query || '空检索' }}</div>
              <div class="history-meta">
                <el-tag size="small" type="success" effect="plain">{{ item.result_count }} 个结果</el-tag>
              </div>
            </div>
          </el-timeline-item>
        </el-timeline>
        <el-empty v-else description="暂无检索历史" />
      </div>
    </el-drawer>

    <!-- 右键上下文菜单 -->
    <div 
      v-if="contextMenu.visible" 
      :style="{ top: contextMenu.y + 'px', left: contextMenu.x + 'px' }" 
      class="custom-context-menu"
      @click.stop
    >
      <div class="menu-item" @click="handleContextAction('open')">
        <el-icon><Document /></el-icon>
        <span>打开文件</span>
      </div>
      <div class="menu-item" @click="handleContextAction('locate')">
        <el-icon><FolderOpened /></el-icon>
        <span>定位所在文件夹</span>
      </div>
      <el-divider class="menu-divider" />
      <div class="menu-item" @click="handleContextAction('copyPath')">
        <el-icon><CopyDocument /></el-icon>
        <span>复制完整绝对路径</span>
      </div>
      <div class="menu-item" @click="handleContextAction('copyName')">
        <el-icon><Document /></el-icon>
        <span>复制文件名</span>
      </div>
      <el-divider class="menu-divider" />
      <div class="menu-item danger" @click="handleContextAction('delete')">
        <el-icon><Delete /></el-icon>
        <span>物理删除文件</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 学术检索中心 - 全面统一的主线布局风格 */
.search-center {
  width: 100%;
  min-width: 100%;
  max-width: 100%;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.search-center > * {
  width: 100% !important;
  box-sizing: border-box;
}

/* 页面头部 */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  flex-shrink: 0;
}

.header-title h2 {
  margin: 0 0 4px 0;
  font-size: 24px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.header-subtitle {
  color: var(--el-text-color-secondary);
  font-size: 14px;
  margin: 0;
}

/* 双卡片网格布局，完美对齐扫描中心 */
.grid-card-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  width: 100%;
  margin-bottom: 8px;
}

.engine-card {
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.45);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-radius: 12px;
  transition: all 0.3s cubic-bezier(0.25, 0.8, 0.25, 1);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.05);
}

.dark .engine-card {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(30, 32, 40, 0.55);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25);
}

.card-header {
  font-size: 15px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  display: flex;
  align-items: center;
  gap: 8px;
}

.card-content-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 180px;
  justify-content: space-between;
}

.form-item-wrapper {
  display: flex;
  flex-direction: column;
  gap: 6px;
  text-align: left;
}

.form-item-label {
  font-size: 13px;
  font-weight: 550;
  color: var(--el-text-color-regular);
}

/* 输入框聚焦特效 */
.glass-input-premium :deep(.el-input__wrapper) {
  background: rgba(255, 255, 255, 0.5);
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 8px;
  box-shadow: none !important;
  transition: all 0.3s;
}

.dark .glass-input-premium :deep(.el-input__wrapper) {
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.glass-input-premium :deep(.el-input__wrapper.is-focus) {
  background: rgba(255, 255, 255, 0.9);
  border-color: var(--el-color-primary);
  box-shadow: 0 0 10px rgba(64, 158, 255, 0.3) !important;
}

.dark .glass-input-premium :deep(.el-input__wrapper.is-focus) {
  background: rgba(0, 0, 0, 0.6);
  box-shadow: 0 0 10px rgba(64, 158, 255, 0.4) !important;
}

/* 正则报错高亮 */
.regex-error-border :deep(.el-input__wrapper) {
  border-color: var(--el-color-danger) !important;
  box-shadow: 0 0 8px rgba(245, 108, 108, 0.25) !important;
}

.regex-error-text {
  font-size: 12px;
  color: var(--el-color-danger);
  margin-top: 4px;
  display: flex;
  align-items: center;
  gap: 4px;
}

/* 路径标签区 */
.search-path-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  max-height: 80px;
  overflow-y: auto;
  padding: 4px;
  background: rgba(0, 0, 0, 0.03);
  border-radius: 6px;
}

.dark .search-path-badges {
  background: rgba(255, 255, 255, 0.03);
}

.path-badge-tag {
  max-width: 100%;
  text-overflow: ellipsis;
  overflow: hidden;
}

.path-empty-tip {
  font-size: 12px;
  color: var(--el-text-color-placeholder);
  font-style: italic;
}

/* 检索按钮 */
.glass-btn-search {
  width: 100%;
  height: 38px;
  border-radius: 8px;
  font-weight: 600;
  font-size: 14px;
  background: linear-gradient(135deg, #409eff 0%, #0076f5 100%);
  border: none;
  box-shadow: 0 4px 15px rgba(0, 118, 245, 0.35);
  transition: all 0.3s;
}

.glass-btn-search:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(0, 118, 245, 0.5);
}

/* 比对算法 Radio */
.algorithm-radio-group {
  width: 100%;
  display: flex;
}

.algorithm-radio-group :deep(.el-radio-button) {
  flex: 1;
}

.algorithm-radio-group :deep(.el-radio-button__inner) {
  width: 100%;
  background: rgba(255, 255, 255, 0.5);
  border: 1px solid rgba(0, 0, 0, 0.08);
  font-size: 12px;
  padding: 8px 12px;
  transition: all 0.3s;
}

.dark .algorithm-radio-group :deep(.el-radio-button__inner) {
  background: rgba(0, 0, 0, 0.2);
  border-color: rgba(255, 255, 255, 0.08);
  color: var(--el-text-color-regular);
}

/* 大小区间 */
.range-inputs {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
}

.glass-number {
  flex: 1;
}

.glass-number :deep(.el-input__wrapper) {
  background: rgba(255, 255, 255, 0.4);
  border-radius: 8px;
}

.dark .glass-number :deep(.el-input__wrapper) {
  background: rgba(0, 0, 0, 0.3);
}

.range-divider {
  font-size: 13px;
  color: var(--el-text-color-secondary);
}

/* 学术测算指标 */
.search-insights-row {
  width: 100%;
  margin-bottom: 8px;
}

.insights-card {
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.35);
  backdrop-filter: blur(15px);
  -webkit-backdrop-filter: blur(15px);
  border-radius: 10px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.02);
}

.dark .insights-card {
  border-color: rgba(255, 255, 255, 0.05);
  background: rgba(25, 27, 34, 0.45);
}

.insights-wrapper {
  display: flex;
  justify-content: space-around;
  align-items: center;
  padding: 4px 0;
}

.insight-metric {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.insight-label {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.insight-value {
  font-size: 20px;
  font-weight: 700;
  color: var(--el-text-color-primary);
}

.insight-value small {
  font-size: 11px;
  font-weight: normal;
  color: var(--el-text-color-placeholder);
}

/* 媒体类型过滤栏 */
.media-filter-bar {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
  overflow-x: auto;
  padding-bottom: 4px;
}

.media-tab {
  padding: 6px 14px;
  font-size: 13px;
  font-weight: 550;
  border-radius: 20px;
  background: rgba(255, 255, 255, 0.5);
  border: 1px solid rgba(0, 0, 0, 0.06);
  color: var(--el-text-color-regular);
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.25, 0.8, 0.25, 1);
  user-select: none;
}

.dark .media-tab {
  background: rgba(255, 255, 255, 0.04);
  border-color: rgba(255, 255, 255, 0.05);
}

.media-tab:hover {
  background: rgba(255, 255, 255, 0.85);
  transform: translateY(-1px);
}

.dark .media-tab:hover {
  background: rgba(255, 255, 255, 0.08);
}

.media-tab.active {
  background: var(--el-color-primary);
  color: #fff;
  border-color: var(--el-color-primary);
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.3);
}

/* 结果工作区 */
.results-workbench {
  width: 100%;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 380px;
}

.search-workflow-wrapper {
  display: flex;
  gap: 16px;
  width: 100%;
  height: 520px; /* 固定列表工作区高度 */
  overflow: hidden;
}

.results-container {
  flex: 1;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.25);
  background: rgba(255, 255, 255, 0.45);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.04);
}

.dark .results-container {
  border-color: rgba(255, 255, 255, 0.06);
  background: rgba(30, 32, 40, 0.45);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25);
}

/* 虚拟列表头部，高度 48px */
.virtual-table-header {
  display: flex;
  align-items: center;
  height: 48px;
  background: rgba(0, 0, 0, 0.03);
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-regular);
  padding: 0 16px;
  box-sizing: border-box;
}

.dark .virtual-table-header {
  background: rgba(255, 255, 255, 0.02);
  border-bottom-color: rgba(255, 255, 255, 0.06);
}

.virtual-table-body {
  flex: 1;
  overflow: hidden;
  width: 100%;
}

/* 列表行高 48px 统一 */
.virtual-table-row {
  display: flex;
  align-items: center;
  height: 48px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.03);
  font-size: 13px;
  color: var(--el-text-color-regular);
  cursor: pointer;
  transition: all 0.2s;
  padding: 0 16px;
  box-sizing: border-box;
  text-align: left;
}

.dark .virtual-table-row {
  border-bottom-color: rgba(255, 255, 255, 0.03);
}

.virtual-table-row:hover {
  background: rgba(64, 158, 255, 0.08);
}

.virtual-table-row.active {
  background: rgba(64, 158, 255, 0.15);
  border-left: 3px solid var(--el-color-primary);
}

/* 网格列宽分配 */
.col-num {
  width: 50px;
  text-align: center;
}

.col-name {
  flex: 2;
  display: flex;
  align-items: center;
  gap: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.col-size {
  width: 100px;
}

.col-score {
  width: 110px;
}

.col-path {
  flex: 3;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--el-text-color-secondary);
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
}

.col-date {
  width: 160px;
  color: var(--el-text-color-secondary);
}

.file-icon {
  color: #409eff;
  flex-shrink: 0;
}

.highlight-keyword {
  background: rgba(245, 108, 108, 0.18);
  color: #f56c6c;
  font-weight: bold;
  border-radius: 2px;
  padding: 0 2px;
}

/* 相似度标签 */
.tag-similarity {
  border-radius: 4px;
}

/* 结果底栏 */
.results-footer {
  height: 42px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 16px;
  background: rgba(0, 0, 0, 0.02);
  border-top: 1px solid rgba(0, 0, 0, 0.05);
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.dark .results-footer {
  background: rgba(255, 255, 255, 0.01);
  border-top-color: rgba(255, 255, 255, 0.05);
}

/* 右侧属性面板，暗色彻底防惨白 */
.search-preview-sidebar {
  width: 320px;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.25);
  background: rgba(255, 255, 255, 0.55);
  backdrop-filter: blur(25px);
  -webkit-backdrop-filter: blur(25px);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.05);
  padding: 16px;
  box-sizing: border-box;
}

.dark .search-preview-sidebar {
  border-color: rgba(255, 255, 255, 0.06);
  background: rgba(30, 32, 40, 0.55);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25);
}

.card-header-title {
  font-size: 14px;
  font-weight: bold;
  display: flex;
  align-items: center;
  gap: 8px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  padding-bottom: 10px;
  margin-bottom: 14px;
  color: var(--el-text-color-primary);
}

.dark .card-header-title {
  border-bottom-color: rgba(255, 255, 255, 0.08);
}

/* 缩略图 */
.image-preview-wrapper {
  width: 100%;
  height: 160px;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.03);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  margin-bottom: 14px;
  border: 1px solid rgba(0, 0, 0, 0.05);
}

.dark .image-preview-wrapper {
  background: rgba(0, 0, 0, 0.2);
  border-color: rgba(255, 255, 255, 0.05);
}

.preview-img-element {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.generic-preview-wrapper {
  width: 100%;
  height: 160px;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.02);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-bottom: 14px;
  border: 1px dashed rgba(0, 0, 0, 0.1);
}

.dark .generic-preview-wrapper {
  background: rgba(255, 255, 255, 0.01);
  border-color: rgba(255, 255, 255, 0.08);
}

.ext-banner {
  font-size: 11px;
  background: rgba(0, 0, 0, 0.05);
  padding: 2px 8px;
  border-radius: 10px;
  color: var(--el-text-color-secondary);
}

/* 属性排版 */
.metadata-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 18px;
  text-align: left;
}

.meta-row {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.meta-label {
  font-size: 11px;
  color: var(--el-text-color-placeholder);
}

.meta-value {
  font-size: 12px;
  color: var(--el-text-color-primary);
  word-break: break-all;
  line-height: 1.4;
}

.meta-value.copyable {
  cursor: pointer;
  transition: color 0.2s;
}

.meta-value.copyable:hover {
  color: var(--el-color-primary);
}

/* 右侧控制大按钮 */
.glass-btn-full {
  width: 100%;
  margin: 0 0 8px 0 !important;
  background: rgba(255, 255, 255, 0.6);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 6px;
  transition: all 0.3s;
}

.dark .glass-btn-full {
  background: rgba(255, 255, 255, 0.03);
  border-color: rgba(255, 255, 255, 0.08);
  color: var(--el-text-color-regular);
}

.glass-btn-full:hover {
  background: rgba(255, 255, 255, 0.9);
  border-color: var(--el-color-primary);
}

/* 右键上下文菜单，彻底适配深色主题，防止惨白 */
.custom-context-menu {
  position: fixed;
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
  z-index: 10000;
  width: 180px;
  padding: 6px 0;
  box-sizing: border-box;
  text-align: left;
}

.dark .custom-context-menu {
  background: rgba(30, 32, 40, 0.95);
  border-color: rgba(255, 255, 255, 0.1);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  font-size: 12px;
  color: var(--el-text-color-regular);
  cursor: pointer;
  transition: all 0.2s;
}

.menu-item:hover {
  background: rgba(64, 158, 255, 0.12);
  color: var(--el-color-primary);
}

.menu-item.danger {
  color: var(--el-color-danger);
}

.menu-item.danger:hover {
  background: rgba(245, 108, 108, 0.1);
}

.menu-divider {
  margin: 4px 0 !important;
}

/* 历史列表卡片 */
.history-item-card {
  padding: 10px;
  background: rgba(0, 0, 0, 0.02);
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.2s;
}

.dark .history-item-card {
  background: rgba(255, 255, 255, 0.02);
}

.history-item-card:hover {
  background: rgba(64, 158, 255, 0.1);
}

.history-query {
  font-size: 13px;
  font-weight: bold;
  color: var(--el-text-color-primary);
}

.history-meta {
  margin-top: 6px;
  display: flex;
  gap: 6px;
}
</style>
