<script setup lang="ts">
/**
 * SpaceTree 空间树 - 主控制中心（MFT/USN 驱动极速版）
 * @description 采用极客三栏玻璃拟态布局，内置物理设备状态感知、一键热定位重映射、多维度部分匹配算法以及自研百万级扁平虚拟滚动查重工作台
 */
import { ref, onMounted, defineAsyncComponent, computed, watch } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { 
  Delete, Folder, Cpu, Document, FolderOpened, View, InfoFilled, Refresh,
  ArrowDown, ArrowUp, Link
} from '@element-plus/icons-vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { formatSize, formatDate } from './utils/formatters';
import { useFileOperations } from './composables/useFileOperations';

// 导入基础组件
import AppLayout from './components/AppLayout.vue';
import VirtualList from './components/VirtualList.vue';

// 异步加载其他子页面组件
const ScanCenter = defineAsyncComponent(() => import('./components/ScanCenter.vue'));
const SearchCenter = defineAsyncComponent(() => import('./components/SearchCenter.vue'));
const DirectoryTree = defineAsyncComponent(() => import('./components/DirectoryTree.vue'));
const ImageArchiveCenter = defineAsyncComponent(() => import('./components/ImageArchiveCenter.vue'));
const AnalysisCenter = defineAsyncComponent(() => import('./components/AnalysisCenter.vue'));
const ToolsCenter = defineAsyncComponent(() => import('./components/ToolsCenter.vue'));
const SettingsCenter = defineAsyncComponent(() => import('./components/SettingsCenter.vue'));
const UpdateCenter = defineAsyncComponent(() => import('./components/UpdateCenter.vue'));
const AboutCenter = defineAsyncComponent(() => import('./components/AboutCenter.vue'));

// 声明全局变量
declare global {
  interface Window {
    __TAURI__?: any;
  }
}

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
}

interface DuplicateGroup {
  hash: string;
  files: FileInfo[];
  total_size: number;
  wasted_space: number;
}

interface DuplicateClassification {
  complete_duplicates: DuplicateGroup[];
  name_duplicates: DuplicateGroup[];
  content_duplicates: DuplicateGroup[];
}

// 激活的菜单
const activeMenu = ref('search');

// 数据库路径
const dbPath = ref('');

// 数据库中是否有文件数据
const hasDatabaseFiles = ref<boolean | null>(null);

// 重复文件分类结果
const duplicateClassification = ref<DuplicateClassification | null>(null);

// 查重策略配置
const matchStrategy = ref<'hash' | 'partial' | 'name' | 'content'>('hash');
const partialSizeKB = ref(100); // 默认前 100 KB 计算部分哈希
const activeSubTab = ref<'complete' | 'name' | 'content'>('complete');

// 物理设备与盘符状态感知数据
const systemDrives = ref([
  { letter: 'C:\\', name: '系统盘 (C:)', status: 'active' },
  { letter: 'D:\\', name: '数据盘 (D:)', status: 'active' },
  { letter: 'E:\\', name: '本地磁盘 (E:)', status: 'active' },
  { letter: 'F:\\', name: '本地磁盘 (F:)', status: 'active' },
  { letter: 'G:\\', name: '移动卷 (G:)', status: 'inactive' },
]);

// 扁平虚拟化所需要的状态
const expandedGroups = ref<Record<string, boolean>>({});
const selectedPaths = ref<Record<string, boolean>>({});
const selectedPreviewFile = ref<FileInfo | null>(null);

const isRemapping = ref(false);
const isBatchDeleting = ref(false);

// 使用文件操作 Hook
const { openFile, openFolder, deleteFile } = useFileOperations({
  dbPath: dbPath.value,
  onDeleteSuccess: () => queryDuplicates()
});

// 重置选中状态
function resetSelections() {
  selectedPaths.value = {};
  selectedPreviewFile.value = null;
}

// 物理盘符热重定位与更新映射
async function runVolumeRemap() {
  if (!dbPath.value) {
    ElMessage.warning('数据库未就绪');
    return;
  }
  isRemapping.value = true;
  try {
    const result = await invoke('check_and_remap_volumes', { db_path: dbPath.value }) as any;
    if (result.success) {
      ElMessage.success(`盘符校验完成！共重新映射了 ${result.remapped_files} 个失效文件路径。`);
      if (result.remapped_files > 0) {
        await queryDuplicates();
      }
    }
  } catch (error) {
    console.error('物理路径重映射失败:', error);
    ElMessage.error('重映射失败: ' + error);
  } finally {
    isRemapping.value = false;
  }
}

// 检查数据库是否有数据
async function checkDatabaseFiles() {
  if (!dbPath.value) {
    console.log("checkDatabaseFiles: dbPath is empty, deferring check.");
    return;
  }
  try {
    const hasFiles = await invoke('check_database_has_files', { db_path: dbPath.value }) as boolean;
    hasDatabaseFiles.value = hasFiles;
    if (hasFiles) {
      await queryDuplicates();
    }
  } catch (e) {
    console.error('检查数据库文件失败:', e);
    ElMessage.error('读取文件库索引状态失败: ' + e);
    hasDatabaseFiles.value = false;
  }
}

// 执行查重分析
async function queryDuplicates() {
  if (!dbPath.value) return;
  try {
    resetSelections();
    
    if (matchStrategy.value === 'partial') {
      ElMessage.info('正在执行前 K 字节部分哈希查重...');
      const sizeBytes = partialSizeKB.value * 1024;
      const result = await invoke('find_duplicates_partial', { 
        db_path: dbPath.value, 
        partial_size: sizeBytes 
      }) as any;
      
      duplicateClassification.value = {
        complete_duplicates: result.groups || [],
        name_duplicates: [],
        content_duplicates: []
      };
      
      activeSubTab.value = 'complete'; // 部分匹配仅有这一个组
      ElMessage.success(`查重成功！找到 ${result.total_groups || 0} 组头部哈希匹配文件。`);
    } else {
      ElMessage.info('正在分类检索重复文件...');
      const result = await invoke('find_duplicates', { db_path: dbPath.value }) as DuplicateClassification;
      duplicateClassification.value = result;
      
      const totalGroups = result.complete_duplicates.length + 
                          result.name_duplicates.length + 
                          result.content_duplicates.length;
      ElMessage.success(`检索成功！共找到 ${totalGroups} 组重复文件。`);
      
      // 优化：如果当前选中的 Tab 长度为 0，自动切往有数据的子分类 Tab
      if (result.complete_duplicates.length === 0) {
        if (result.content_duplicates.length > 0) {
          activeSubTab.value = 'content';
        } else if (result.name_duplicates.length > 0) {
          activeSubTab.value = 'name';
        }
      }
    }
  } catch (e) {
    console.error('查询重复文件失败:', e);
    ElMessage.error('分析失败: ' + e);
  }
}

// 切换查重匹配策略
watch(matchStrategy, () => {
  queryDuplicates();
});

// 监听 dbPath，只要从空变为有效就立即主动触发状态校验（防异步竞态）
watch(dbPath, (newVal) => {
  if (newVal) {
    console.log("watch(dbPath): dbPath updated, running checkDatabaseFiles.");
    checkDatabaseFiles();
  }
});

// 处理菜单切换
async function handleMenuChange(key: string) {
  activeMenu.value = key;
  if (key === 'duplicates') {
    await checkDatabaseFiles();
  }
}

// 物理驱动器与查重分类的过滤关联
const currentGroups = computed(() => {
  if (!duplicateClassification.value) return [];
  if (matchStrategy.value === 'partial') {
    return duplicateClassification.value.complete_duplicates;
  }
  if (activeSubTab.value === 'content') {
    return duplicateClassification.value.content_duplicates;
  }
  if (activeSubTab.value === 'name') {
    return duplicateClassification.value.name_duplicates;
  }
  return duplicateClassification.value.complete_duplicates;
});

// 百万级数据打平为一维数组，用于扁平化虚拟滚动渲染
const flattenedItems = computed(() => {
  const list: any[] = [];
  currentGroups.value.forEach((group, gIdx) => {
    const isExpanded = expandedGroups.value[group.hash] !== false;
    list.push({
      type: 'header',
      hash: group.hash,
      gIdx,
      group,
      isExpanded
    });
    if (isExpanded) {
      group.files.forEach((file, fIdx) => {
        list.push({
          type: 'file',
          hash: group.hash,
          file,
          fIdx,
          group
        });
      });
    }
  });
  return list;
});

// 统计被勾选的文件
const selectedCount = computed(() => {
  return Object.values(selectedPaths.value).filter(Boolean).length;
});

// 统计总浪费空间大小
const wastedSpaceStats = computed(() => {
  if (!duplicateClassification.value) return 0;
  let total = 0;
  if (matchStrategy.value === 'partial') {
    duplicateClassification.value.complete_duplicates.forEach(g => total += g.wasted_space);
  } else {
    duplicateClassification.value.complete_duplicates.forEach(g => total += g.wasted_space);
    duplicateClassification.value.name_duplicates.forEach(g => total += g.wasted_space);
    duplicateClassification.value.content_duplicates.forEach(g => total += g.wasted_space);
  }
  return total;
});

const filterKeyword = ref('');
const keywordStrategy = ref<'keyword_include' | 'keyword_exclude'>('keyword_include');
const duplicateMode = ref<'wizard' | 'pro'>('pro');

// 智能保留策略选择算法
function applyKeepStrategy(strategy: 'earliest' | 'latest' | 'shortest_path' | 'shortest_name' | 'all_but_one' | 'keyword_include' | 'keyword_exclude', keyword?: string) {
  selectedPaths.value = {};
  currentGroups.value.forEach((group) => {
    if (group.files.length <= 1) return;
    
    let keepIndex = 0;
    
    if (strategy === 'earliest') {
      let earliestTime = Infinity;
      group.files.forEach((f, idx) => {
        const t = new Date(f.modified_at).getTime() || Infinity;
        if (t < earliestTime) {
          earliestTime = t;
          keepIndex = idx;
        }
      });
    } else if (strategy === 'latest') {
      let latestTime = -Infinity;
      group.files.forEach((f, idx) => {
        const t = new Date(f.modified_at).getTime() || -Infinity;
        if (t > latestTime) {
          latestTime = t;
          keepIndex = idx;
        }
      });
    } else if (strategy === 'shortest_path') {
      let minLen = Infinity;
      group.files.forEach((f, idx) => {
        if (f.path.length < minLen) {
          minLen = f.path.length;
          keepIndex = idx;
        }
      });
    } else if (strategy === 'shortest_name') {
      let minLen = Infinity;
      group.files.forEach((f, idx) => {
        if (f.filename.length < minLen) {
          minLen = f.filename.length;
          keepIndex = idx;
        }
      });
    } else if (strategy === 'keyword_include' && keyword) {
      const kw = keyword.toLowerCase();
      let found = -1;
      group.files.forEach((f, idx) => {
        if (f.path.toLowerCase().includes(kw)) {
          found = idx;
        }
      });
      if (found !== -1) keepIndex = found;
    } else if (strategy === 'keyword_exclude' && keyword) {
      const kw = keyword.toLowerCase();
      let found = -1;
      group.files.forEach((f, idx) => {
        if (!f.path.toLowerCase().includes(kw)) {
          found = idx;
        }
      });
      if (found !== -1) keepIndex = found;
    } else if (strategy === 'all_but_one') {
      keepIndex = 0; // 默认保留第一个
    }
    
    // 除保留项外，其余打勾
    group.files.forEach((f, idx) => {
      if (idx !== keepIndex) {
        selectedPaths.value[f.path] = true;
      }
    });
  });
  
  ElMessage.success(`智能策略已应用！已为您勾选 ${Object.keys(selectedPaths.value).length} 个重复文件。`);
}

function isDeleteFullAuditEnabled(): boolean {
  try {
    const saved = localStorage.getItem('dfh_settings');
    if (saved) {
      const settings = JSON.parse(saved);
      if (settings.general && settings.general.enableDeleteFullAudit !== undefined) {
        return settings.general.enableDeleteFullAudit;
      }
    }
  } catch (e) {
    console.error('获取安全审计配置失败:', e);
  }
  return true; // 默认开启
}

// 批量清理选中文件
async function executeBatchDelete() {
  const pathsToDelete = Object.keys(selectedPaths.value).filter(p => selectedPaths.value[p]);
  if (pathsToDelete.length === 0) {
    ElMessage.warning('请先勾选需要删除的文件');
    return;
  }
  
  try {
    await ElMessageBox.confirm(
      `确定要批量永久删除所选的 ${pathsToDelete.length} 个重复文件吗？\n该操作不可撤销，请务必仔细核对路径！`,
      '批量删除确认',
      {
        confirmButtonText: '立即删除',
        cancelButtonText: '取消',
        type: 'warning'
      }
    );
  } catch {
    return; // 用户取消
  }

  // 物理删除前安全审计拦截
  if (isDeleteFullAuditEnabled()) {
    try {
      isBatchDeleting.value = true;
      ElMessage.info('正在对即将删除的文件进行全文件哈希安全审计...');
      
      for (const group of currentGroups.value) {
        if (group.files.length <= 1) continue;
        
        const toDeleteInGroup = group.files.filter(f => selectedPaths.value[f.path]);
        if (toDeleteInGroup.length === 0) continue;
        
        const keepInGroup = group.files.find(f => !selectedPaths.value[f.path]);
        if (!keepInGroup) {
          ElMessage.error(`警告：检测到某组中所有重复项均被勾选删除。已自动保护拦截。`);
          isBatchDeleting.value = false;
          return;
        }
        
        const match = await invoke('verify_file_hashes_match', {
          keep_path: keepInGroup.path,
          delete_paths: toDeleteInGroup.map(f => f.path)
        });
        
        if (!match) {
          ElMessageBox.alert(
            `🚨 安全审计拦截：\n发现文件存在哈希碰撞（文件大小相同，头中尾局部哈希相同，但全文件字节内容不同）！\n\n冲突组首个文件: ${keepInGroup.path}\n\n系统已为您自动拦截该删除动作，防止误删珍贵数据！`,
            '防误删安全警告',
            { type: 'error', confirmButtonText: '我知道了' }
          );
          isBatchDeleting.value = false;
          return;
        }
      }
    } catch (auditErr) {
      console.error('安全审计执行失败:', auditErr);
      ElMessage.error('安全审计出错: ' + auditErr);
      isBatchDeleting.value = false;
      return;
    } finally {
      isBatchDeleting.value = false;
    }
  }
  
  isBatchDeleting.value = true;
  try {
    const deletedCount = await invoke('batch_delete_files', {
      paths: pathsToDelete,
      db_path: dbPath.value,
      allowed_roots: null
    }) as number;
    
    ElMessage.success(`批量删除成功！共清理 ${deletedCount} 个重复文件，对齐标杆防误删。`);
    selectedPaths.value = {};
    await queryDuplicates();
  } catch (error) {
    console.error('批量清理失败:', error);
    ElMessage.error('清理失败: ' + error);
  } finally {
    isBatchDeleting.value = false;
  }
}

// 单个文件删除
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
  } catch {
    return; // 用户取消
  }

  // 物理删除前安全审计拦截
  if (isDeleteFullAuditEnabled()) {
    try {
      let keepFile: FileInfo | null = null;
      for (const group of currentGroups.value) {
        if (group.files.some(f => f.path === file.path)) {
          const other = group.files.find(f => f.path !== file.path);
          if (other) {
            keepFile = other;
            break;
          }
        }
      }
      
      if (keepFile) {
        ElMessage.info('正在进行全文件内容哈希校验...');
        const match = await invoke('verify_file_hashes_match', {
          keep_path: keepFile.path,
          delete_paths: [file.path]
        });
        
        if (!match) {
          ElMessageBox.alert(
            `🚨 安全审计拦截：\n物理删除已被拦截！该文件与保留的重复组文件大小相同、局部哈希一致，但全文件字节哈希不符合（内容有差异）。\n\n对比源文件: ${keepFile.path}`,
            '内容冲突安全警告',
            { type: 'error', confirmButtonText: '我知道了' }
          );
          return;
        }
      }
    } catch (auditErr) {
      console.error('单文件删除安全审计出错:', auditErr);
      ElMessage.error('校验失败: ' + auditErr);
      return;
    }
  }

  try {
    const success = await deleteFile(file);
    if (success) {
      ElMessage.success('删除成功');
      await queryDuplicates();
    }
  } catch (error) {
    console.error('删除单文件失败:', error);
  }
}

// 批量替换为硬链接（无损节省空间）
const isBatchHardlinking = ref(false);

async function executeBatchHardlink() {
  const pathsToLink = Object.keys(selectedPaths.value).filter(p => selectedPaths.value[p]);
  if (pathsToLink.length === 0) {
    ElMessage.warning('请先勾选需要替换的文件');
    return;
  }
  
  try {
    await ElMessageBox.confirm(
      `确定要把这 ${pathsToLink.length} 个重复文件替换为物理硬链接吗？\n硬链接将在文件系统层指向同一份数据，彻底回收重复体积，同时保留各处文件路径的正常访问。`,
      '批量硬链接替换确认',
      {
        confirmButtonText: '立即替换',
        cancelButtonText: '取消',
        type: 'warning'
      }
    );
  } catch {
    return;
  }
  
  isBatchHardlinking.value = true;
  try {
    let replacedTotal = 0;
    
    for (const group of currentGroups.value) {
      const groupToDelete = group.files.filter(f => selectedPaths.value[f.path]);
      if (groupToDelete.length === 0) continue;
      
      const keepFile = group.files.find(f => !selectedPaths.value[f.path]);
      if (!keepFile) {
        ElMessage.error(`错误：检测到某组中所有重复项均被勾选，请至少保留一个文件！`);
        isBatchHardlinking.value = false;
        return;
      }
      
      const count = await invoke('replace_files_with_hardlinks', {
        keep_path: keepFile.path,
        replace_paths: groupToDelete.map(f => f.path),
        db_path: dbPath.value,
        allowed_roots: null
      }) as number;
      
      replacedTotal += count;
    }
    
    ElMessage.success(`🎉 硬链接去重成功！共无损替换 ${replacedTotal} 个文件，重复空间已被彻底释放！`);
    selectedPaths.value = {};
    await queryDuplicates();
  } catch (error) {
    console.error('硬链接替换失败:', error);
    ElMessage.error('硬链接建立失败 (硬链接不能跨盘卷建立): ' + error);
  } finally {
    isBatchHardlinking.value = false;
  }
}

// 单个文件硬链接无损替换
async function executeSingleHardlink(file: FileInfo) {
  try {
    let keepFile: FileInfo | null = null;
    for (const group of currentGroups.value) {
      if (group.files.some(f => f.path === file.path)) {
        const other = group.files.find(f => f.path !== file.path);
        if (other) {
          keepFile = other;
          break;
        }
      }
    }
    
    if (!keepFile) {
      ElMessage.warning('未能找到重复组中用于链接的保留文件');
      return;
    }

    await ElMessageBox.confirm(
      `确定要把此文件替换为指向 "${keepFile.filename}" 的物理硬链接吗？`,
      '硬链接替换确认',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning'
      }
    );
    
    const count = await invoke('replace_files_with_hardlinks', {
      keep_path: keepFile.path,
      replace_paths: [file.path],
      db_path: dbPath.value,
      allowed_roots: null
    }) as number;
    
    if (count > 0) {
      ElMessage.success('硬链接建立成功');
      await queryDuplicates();
    }
  } catch (error) {
    if (error !== 'cancel') {
      console.error('单个硬链接建立失败:', error);
      ElMessage.error('替换失败 (硬链接不能跨分区卷建立): ' + error);
    }
  }
}

// 折叠或展开单个重复组
function toggleGroupCollapse(hash: string) {
  const isExpanded = expandedGroups.value[hash] !== false;
  expandedGroups.value[hash] = !isExpanded;
}

// 全部折叠或展开
function toggleAllGroups(expand: boolean) {
  currentGroups.value.forEach(g => {
    expandedGroups.value[g.hash] = expand;
  });
}

// 获取预览文件类型
function isImage(file: FileInfo | null) {
  if (!file) return false;
  const ext = (file.file_extension || '').toLowerCase();
  return ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'ico'].includes(ext);
}

// 初始化
async function initializeApp() {
  try {
    const { join } = await import('@tauri-apps/api/path');
    let exeDir = await invoke('get_exe_dir') as string;
    dbPath.value = await join(exeDir, 'spacetree.db');

    // 数据库不存在则自动创建
    const { exists } = await import('@tauri-apps/plugin-fs');
    const dbExists = await exists(dbPath.value);

    if (!dbExists) {
      await invoke('init_database_cmd', { db_path: dbPath.value });
      ElMessage.success('初次启动，数据库初始化完成');
    }
    
    // 关键：在数据库就绪之后，主动运行一次数据库文件状态校验
    await checkDatabaseFiles();

    // 体验优化：若数据库中无任何数据，默认进入扫描中心
    if (hasDatabaseFiles.value === false) {
      activeMenu.value = 'scan';
    }
  } catch (error) {
    console.error('启动初始化失败:', error);
  }
}

import { usePerformance } from './composables/usePerformance';
const { initPerformance } = usePerformance();

onMounted(() => {
  initPerformance();
  initializeApp();
});
</script>

<template>
  <AppLayout :active-menu="activeMenu" @menu-change="handleMenuChange">
    <!-- 扫描中心 -->
    <ScanCenter
      v-if="activeMenu === 'scan'"
      :db_path="dbPath"
      @scan-complete="checkDatabaseFiles"
      @go-to-duplicates="handleMenuChange('duplicates')"
    />

    <!-- 文件搜索 -->
    <SearchCenter
      v-else-if="activeMenu === 'search'"
      :db_path="dbPath"
    />

    <!-- 目录浏览 -->
    <DirectoryTree
      v-else-if="activeMenu === 'directory'"
      :db_path="dbPath"
    />

    <!-- 图片打包 -->
    <ImageArchiveCenter
      v-else-if="activeMenu === 'image_archive'"
      :db_path="dbPath"
    />

    <!-- 重复文件（MFT/USN 驱动极速查重工作台） -->
    <div v-else-if="activeMenu === 'duplicates'" class="duplicates-page-container">
      
      <!-- 页面头部 -->
      <div class="page-header">
        <div class="header-title">
          <h2>重复文件</h2>
          <p class="header-subtitle">支持物理盘符漂移热重定位与前 K 字节部分哈希算法，提供百万级流畅虚拟滚动展示</p>
        </div>
        <div class="header-actions">
          <!-- 模式切换选择器 -->
          <el-radio-group v-model="duplicateMode" size="default" style="margin-right: 12px;" class="glass-radio-group">
            <el-radio-button label="pro">🛠️ 专业模式</el-radio-button>
            <el-radio-button label="wizard">🪄 向导模式</el-radio-button>
          </el-radio-group>
          
          <div class="dashboard-stats-row">
            <div class="stat-badge">
              <span class="label">可释放空间</span>
              <span class="val">{{ formatSize(wastedSpaceStats) }}</span>
            </div>
            <div class="stat-badge">
              <span class="label">当前过滤重复组</span>
              <span class="val">{{ currentGroups.length }} 组</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 状态 1：加载中 -->
      <div class="database-loading-card" v-if="hasDatabaseFiles === null">
        <el-card shadow="never" class="glass-card">
          <div class="loading-state-wrapper">
            <el-icon class="is-loading" :size="32" color="#409EFF"><Refresh /></el-icon>
            <span class="loading-text">正在载入极速文件库状态...</span>
          </div>
        </el-card>
      </div>

      <!-- 状态 2：就绪并显示工作台 -->
      <div class="duplicates-workbench" v-else-if="hasDatabaseFiles === true">
        
        <!-- 左侧控制栏 -->
        <aside class="workbench-left-sidebar" v-if="duplicateMode === 'pro'">
          <!-- 卡片 1：匹配算法策略 -->
          <div class="glass-card config-card">
            <div class="card-header-title">
              <el-icon><Cpu /></el-icon>
              <span>匹配算法策略</span>
            </div>
            
            <div class="strategy-selector-body">
              <el-radio-group v-model="matchStrategy" class="strategy-radios">
                <el-radio-button label="hash" value="hash">完全哈希</el-radio-button>
                <el-radio-button label="partial" value="partial">部分哈希 (K)</el-radio-button>
              </el-radio-group>
              
              <el-collapse-transition>
                <div>
                  <div v-if="matchStrategy === 'hash'" class="partial-kb-input-area">
                    <span class="helper-desc" style="margin-top: 4px; display: block; line-height: 1.6;">
                      采用全文件哈希（SHA-256）比对，提供 100% 安全且精准的去重机制，防止任何误删风险。
                    </span>
                  </div>
                  <div v-else-if="matchStrategy === 'partial'" class="partial-kb-input-area">
                    <label class="input-label">读取头部字节大小 (KB)</label>
                    <el-input-number 
                      v-model="partialSizeKB" 
                      :min="1" 
                      :max="102400" 
                      size="small" 
                      controls-position="right"
                      style="width: 100%;"
                    />
                    <span class="helper-desc">仅对比文件前 K 字节哈希。对截断、受损或海量超大文件具备极高对比效率。</span>
                  </div>
                </div>
              </el-collapse-transition>
            </div>
          </div>

          <!-- 卡片 2：物理磁盘热感知 -->
          <div class="glass-card volume-card">
            <div class="card-header-title">
              <el-icon><Folder /></el-icon>
              <span>设备状态热感知</span>
            </div>
            
            <div class="volume-list-body">
              <div v-for="drive in systemDrives" :key="drive.letter" class="volume-item">
                <div class="volume-info">
                  <span class="indicator" :class="drive.status"></span>
                  <span class="name">{{ drive.name }}</span>
                </div>
                <span class="letter">{{ drive.letter }}</span>
              </div>
              
              <el-button 
                type="primary" 
                class="remap-btn"
                :loading="isRemapping"
                @click="runVolumeRemap"
              >
                <el-icon><Refresh /></el-icon>
                盘符漂移热重定位
              </el-button>
              <span class="helper-desc">当拔插U盘导致盘符映射改变时，点击此按钮毫秒级重新对齐文件库路径</span>
            </div>
          </div>
        </aside>

        <!-- 中间列表与操作栏 -->
        <section class="workbench-middle-content">
          <!-- 常规查重时的子大类 Tab -->
          <div class="middle-tabs-row" v-if="matchStrategy !== 'partial'">
            <div 
              class="tab-item" 
              :class="{ active: activeSubTab === 'complete' }"
              @click="activeSubTab = 'complete'"
            >
              完全相同 ({{ duplicateClassification?.complete_duplicates?.length || 0 }})
            </div>
            <div 
              class="tab-item" 
              :class="{ active: activeSubTab === 'content' }"
              @click="activeSubTab = 'content'"
            >
              内容相同 ({{ duplicateClassification?.content_duplicates?.length || 0 }})
            </div>
            <div 
              class="tab-item" 
              :class="{ active: activeSubTab === 'name' }"
              @click="activeSubTab = 'name'"
            >
              名称相同 ({{ duplicateClassification?.name_duplicates?.length || 0 }})
            </div>
          </div>
          
          <div class="middle-tabs-row" v-else>
            <div class="tab-item active">部分哈希匹配结果 ({{ duplicateClassification?.complete_duplicates?.length || 0 }})</div>
          </div>

          <!-- 智能选择与保留规则向导 -->
          <div class="glass-card smart-wizard-panel" v-if="currentGroups.length > 0">
            <div class="wizard-header">
              <div class="wizard-title-area">
                <el-icon class="wizard-icon"><Cpu /></el-icon>
                <div class="wizard-text">
                  <span class="title">智能清理保留向导</span>
                  <span class="subtitle">推荐规则，一键自动选中不需要的冗余文件</span>
                </div>
              </div>
              <div class="wizard-status">
                <span>当前已选中：<strong class="highlight-count">{{ selectedCount }}</strong> 个</span>
              </div>
            </div>

            <!-- 分步向导步骤条与选择 -->
            <div class="wizard-steps-container">
              <!-- 步骤1: 推荐规则一键匹配 -->
              <div class="wizard-step-section">
                <div class="step-label">第一步：选择智能保留规则</div>
                <div class="wizard-buttons-row">
                  <el-button size="small" class="glass-btn" @click="applyKeepStrategy('earliest')">
                    🕒 优先保留最早修改
                  </el-button>
                  <el-button size="small" class="glass-btn" @click="applyKeepStrategy('latest')">
                    🆕 优先保留最新修改
                  </el-button>
                  <el-button size="small" class="glass-btn" @click="applyKeepStrategy('shortest_path')">
                    📁 优先保留最短路径
                  </el-button>
                  <el-button size="small" class="glass-btn" @click="applyKeepStrategy('shortest_name')">
                    🏷️ 优先保留最短名称
                  </el-button>
                  <el-button size="small" class="glass-btn" @click="applyKeepStrategy('all_but_one')">
                    🔢 默认保留组首个
                  </el-button>
                </div>
              </div>

              <!-- 步骤2: 关键字高级过滤 -->
              <div class="wizard-step-section" style="margin-top: 8px; border-top: 1px dashed rgba(220,223,230,0.15); padding-top: 8px;">
                <div class="step-label">第二步：路径/名称关键字深度筛选 (可选)</div>
                <div class="wizard-filter-row">
                  <el-select v-model="keywordStrategy" size="small" style="width: 160px;" class="glass-select-small">
                    <el-option label="包含关键字优先保留" value="keyword_include" />
                    <el-option label="不含关键字优先保留" value="keyword_exclude" />
                  </el-select>
                  <el-input 
                    v-model="filterKeyword" 
                    placeholder="输入路径中特定盘符、文件夹或名称关键字 (如: temp)..." 
                    size="small" 
                    clearable
                    style="flex: 1;"
                    class="custom-search-input"
                  />
                  <el-button 
                    size="small" 
                    type="primary" 
                    class="glass-btn primary"
                    :disabled="!filterKeyword"
                    @click="applyKeepStrategy(keywordStrategy, filterKeyword)"
                  >
                    🚀 应用高级筛选
                  </el-button>
                  <el-button size="small" class="glass-btn warning" @click="selectedPaths = {}">
                    🚫 取消全部标记
                  </el-button>
                </div>
              </div>
            </div>
          </div>

          <!-- 主体虚拟列表 -->
          <div class="glass-card list-box-card">
            <!-- 虚拟滚动表格表头 -->
            <div class="list-table-header">
              <div class="col-check">
                <el-dropdown trigger="click" size="small">
                  <span class="el-dropdown-link" style="cursor: pointer; color: #909399;">
                    选<el-icon class="el-icon--right"><ArrowDown /></el-icon>
                  </span>
                  <template #dropdown>
                    <el-dropdown-menu>
                      <el-dropdown-item @click="toggleAllGroups(true)">展开全部</el-dropdown-item>
                      <el-dropdown-item @click="toggleAllGroups(false)">折叠全部</el-dropdown-item>
                    </el-dropdown-menu>
                  </template>
                </el-dropdown>
              </div>
              <div class="col-name">文件名 / 分组信息</div>
              <div class="col-size">大小</div>
              <div class="col-date">修改时间</div>
              <div class="col-actions">单项操作</div>
            </div>

            <!-- 自研虚拟列表 -->
            <div class="list-table-body">
              <VirtualList
                :items="flattenedItems"
                :item-height="48"
                height="100%"
              >
                <template #default="{ item }">
                  <!-- 渲染组头部 -->
                  <div 
                    v-if="item.type === 'header'" 
                    class="row-header-group"
                    @click="toggleGroupCollapse(item.hash)"
                  >
                    <div class="group-toggle-indicator">
                      <el-icon><ArrowDown v-if="item.isExpanded" /><ArrowUp v-else /></el-icon>
                    </div>
                    <div class="group-desc-info">
                      <span class="group-badge">组 {{ item.gIdx + 1 }}</span>
                      <span class="group-title-filename">{{ item.group.files[0]?.filename }}</span>
                      <span class="group-detail-txt">（共 {{ item.group.files.length }} 份重复，已浪费 {{ formatSize(item.group.wasted_space) }} 空间）</span>
                    </div>
                  </div>

                  <!-- 渲染文件行 -->
                  <div 
                    v-else 
                    class="row-file-item"
                    :class="{ active: selectedPreviewFile?.path === item.file.path }"
                    @click="selectedPreviewFile = item.file"
                  >
                    <div class="col-check" @click.stop>
                      <el-checkbox v-model="selectedPaths[item.file.path]" />
                    </div>
                    
                    <div class="col-name" :title="item.file.path">
                      <el-icon class="file-icon"><Document /></el-icon>
                      <span class="file-path-span">{{ item.file.path }}</span>
                    </div>

                    <div class="col-size">{{ formatSize(item.file.size) }}</div>
                    
                    <div class="col-date">{{ formatDate(item.file.modified_at) }}</div>

                    <div class="col-actions" @click.stop>
                      <el-button-group>
                        <el-button size="small" type="primary" link @click="openFile(item.file.path)" title="打开">
                          <el-icon><Document /></el-icon>
                        </el-button>
                        <el-button size="small" type="info" link @click="openFolder(item.file.path)" title="定位">
                          <el-icon><FolderOpened /></el-icon>
                        </el-button>
                        <el-button size="small" type="danger" link @click="deleteSingleFile(item.file)" title="删除">
                          <el-icon><Delete /></el-icon>
                        </el-button>
                      </el-button-group>
                    </div>
                  </div>
                </template>
                
                <template #empty>
                  <div class="empty-state">
                    <el-empty description="当前分类无重复文件" />
                  </div>
                </template>
              </VirtualList>
            </div>
            
            <!-- 底部批量保留与清理控制栏 -->
            <div class="list-box-footer">
              <div class="strategy-actions">
                <span class="footer-label-text">智能清理保留策略:</span>
                <el-button size="small" class="glass-btn-sm" @click="applyKeepStrategy('shortest_path')">保留最短路径</el-button>
                <el-button size="small" class="glass-btn-sm" @click="applyKeepStrategy('earliest')">保留最早修改</el-button>
                <el-button size="small" class="glass-btn-sm" @click="applyKeepStrategy('latest')">保留最晚修改</el-button>
                <el-button size="small" class="glass-btn-sm" @click="applyKeepStrategy('shortest_name')">保留最短文件名</el-button>
                <el-button size="small" class="glass-btn-sm" @click="applyKeepStrategy('all_but_one')">保留第一个</el-button>
              </div>
              
              <div class="cleanup-actions">
                <span class="selected-summary">已勾选 <strong>{{ selectedCount }}</strong> 个</span>
                <el-button 
                  type="success" 
                  size="default" 
                  class="glass-hardlink-btn"
                  :disabled="selectedCount === 0"
                  :loading="isBatchHardlinking"
                  @click="executeBatchHardlink"
                  style="margin-right: 8px; background: linear-gradient(135deg, #67C23A 0%, #5daf34 100%); border: none; box-shadow: 0 4px 12px rgba(103,194,58,0.25);"
                >
                  <el-icon><Link /></el-icon>
                  替换为硬链接去重
                </el-button>
                <el-button 
                  type="danger" 
                  size="default" 
                  class="glass-delete-btn"
                  :disabled="selectedCount === 0"
                  :loading="isBatchDeleting"
                  @click="executeBatchDelete"
                >
                  <el-icon><Delete /></el-icon>
                  一键批量清理
                </el-button>
              </div>
            </div>
          </div>
        </section>

        <!-- 右侧预览与属性栏 -->
        <aside class="workbench-right-preview">
          <div class="glass-card preview-card">
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

              <!-- 详细属性展示 -->
              <div class="metadata-list">
                <div class="meta-row">
                  <span class="meta-label">文件名</span>
                  <span class="meta-value copyable" :title="selectedPreviewFile.filename">{{ selectedPreviewFile.filename }}</span>
                </div>
                <div class="meta-row">
                  <span class="meta-label">文件路径</span>
                  <span class="meta-value copyable" :title="selectedPreviewFile.path">{{ selectedPreviewFile.path }}</span>
                </div>
                <div class="meta-row">
                  <span class="meta-label">文件大小</span>
                  <span class="meta-value">{{ formatSize(selectedPreviewFile.size) }} ({{ selectedPreviewFile.size }} 字节)</span>
                </div>
                <div class="meta-row" v-if="selectedPreviewFile.hash">
                  <span class="meta-label">哈希签名</span>
                  <span class="meta-value copyable" :title="selectedPreviewFile.hash">{{ selectedPreviewFile.hash.slice(0, 16) }}...</span>
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
              
              <!-- 单项快速按钮 -->
              <div class="quick-action-buttons">
                <el-button size="default" class="glass-btn-full" @click="openFile(selectedPreviewFile.path)">
                  <el-icon><Document /></el-icon>
                  打开此文件
                </el-button>
                <el-button size="default" class="glass-btn-full" @click="openFolder(selectedPreviewFile.path)">
                  <el-icon><FolderOpened /></el-icon>
                  定位所在文件夹
                </el-button>
                <el-button 
                  size="default" 
                  class="glass-btn-full" 
                  style="border-color: var(--el-color-success); color: var(--el-color-success);"
                  @click="executeSingleHardlink(selectedPreviewFile)"
                >
                  <el-icon><Link /></el-icon>
                  替换为硬链接去重
                </el-button>
                <el-button size="default" type="danger" class="glass-btn-full" @click="deleteSingleFile(selectedPreviewFile)">
                  <el-icon><Delete /></el-icon>
                  物理删除此文件
                </el-button>
              </div>
            </div>

            <div class="preview-empty-state" v-else>
              <el-empty description="请在列表中选中文件以进行预览与属性分析">
                <template #image>
                  <el-icon :size="48" color="rgba(255,255,255,0.25)"><InfoFilled /></el-icon>
                </template>
              </el-empty>
            </div>
          </div>
        </aside>

      </div>

      <!-- 数据库空状态 -->
      <div v-else class="database-empty-card">
        <el-card shadow="never" class="glass-card">
          <el-empty description="数据库暂未索引文件，请先前往扫描中心">
            <el-button type="primary" @click="activeMenu = 'scan'" class="go-scan-btn">
              立即前往扫描
            </el-button>
          </el-empty>
        </el-card>
      </div>

    </div>

    <!-- 分析中心 -->
    <AnalysisCenter
      v-else-if="activeMenu === 'analysis'"
      :db_path="dbPath"
    />

    <!-- 工具 -->
    <ToolsCenter
      v-else-if="activeMenu === 'tools'"
      :db_path="dbPath"
    />

    <!-- 设置 -->
    <SettingsCenter
      v-else-if="activeMenu === 'settings'"
      :key="'settings-' + dbPath"
      :db_path="dbPath"
    />

    <!-- 检查更新 -->
    <UpdateCenter
      v-else-if="activeMenu === 'update'"
    />

    <!-- 关于 -->
    <AboutCenter
      v-else-if="activeMenu === 'about'"
    />
  </AppLayout>
</template>

<style>
/* 全局样式基准 */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

/* 全局磨砂玻璃风格一键覆盖系统 */
.el-card {
  background: rgba(255, 255, 255, 0.6) !important;
  backdrop-filter: blur(16px) saturate(180%) !important;
  -webkit-backdrop-filter: blur(16px) saturate(180%) !important;
  border: 1px solid rgba(255, 255, 255, 0.4) !important;
  border-radius: 12px !important;
  box-shadow: 0 8px 32px 0 rgba(31, 38, 135, 0.04) !important;
}

.el-card__header {
  border-bottom: 1px solid rgba(220, 223, 230, 0.3) !important;
  background: rgba(245, 247, 250, 0.2) !important;
}

.el-button:not(.el-button--primary):not(.el-button--danger):not(.el-button--warning):not(.el-button--success) {
  background: rgba(255, 255, 255, 0.6) !important;
  border-color: rgba(220, 223, 230, 0.5) !important;
  backdrop-filter: blur(4px) !important;
}

.el-button:not(.el-button--primary):not(.el-button--danger):not(.el-button--warning):not(.el-button--success):hover {
  background: rgba(255, 255, 255, 0.9) !important;
  border-color: #409EFF !important;
  color: #409EFF !important;
}

.tool-tab, .config-card, .volume-card {
  background: rgba(255, 255, 255, 0.55) !important;
  backdrop-filter: blur(16px) saturate(180%) !important;
  -webkit-backdrop-filter: blur(16px) saturate(180%) !important;
  border: 1px solid rgba(255, 255, 255, 0.45) !important;
  box-shadow: 0 8px 32px 0 rgba(31, 38, 135, 0.04) !important;
}

.tool-tab:hover {
  background: rgba(255, 255, 255, 0.85) !important;
  border-color: #409EFF !important;
}

.tool-tab.is-active {
  background: #ecf5ff !important;
  border-color: #409EFF !important;
  color: #409EFF !important;
}

html, body, #app {
  height: 100%;
  width: 100%;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  background: #f4f6fa;
  overflow: hidden;
}

/* 智能保留向导卡片 */
.smart-wizard-panel {
  padding: 16px !important;
  margin-bottom: 4px;
  flex-shrink: 0;
}

.wizard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  border-bottom: 1px solid rgba(220, 223, 230, 0.4);
  padding-bottom: 8px;
}

.wizard-title-area {
  display: flex;
  align-items: center;
  gap: 8px;
}

.wizard-icon {
  font-size: 16px;
  color: #409EFF;
}

.wizard-text {
  display: flex;
  flex-direction: column;
}

.wizard-text .title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.wizard-text .subtitle {
  font-size: 11px;
  color: var(--text-tertiary);
}

.wizard-status {
  font-size: 12px;
  color: var(--text-secondary);
}

.highlight-count {
  color: #f56c6c;
  font-size: 13px;
}

.wizard-steps-container {
  display: flex;
  flex-direction: column;
}

.wizard-step-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.step-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-tertiary);
  margin-bottom: 4px;
}

.wizard-buttons-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.wizard-filter-row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.dark .smart-wizard-panel,
[data-theme="dark"] .smart-wizard-panel {
  border-color: rgba(255, 255, 255, 0.08) !important;
}

.dark .wizard-header,
[data-theme="dark"] .wizard-header {
  border-color: rgba(255, 255, 255, 0.08) !important;
}

/* 查重页面玻璃拟态控制 */
.duplicates-page-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow: hidden;
}

.duplicates-dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-shrink: 0;
}

.header-main-info h2 {
  font-size: 22px;
  font-weight: 600;
  color: #1f2f3d;
}

.header-main-info .subtext {
  font-size: 13px;
  color: #8492a6;
}

.dashboard-stats-row {
  display: flex;
  gap: 12px;
}

.stat-badge {
  background: rgba(255, 255, 255, 0.7);
  border: 1px solid rgba(255, 255, 255, 0.5);
  border-radius: 8px;
  padding: 8px 16px;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
}

.stat-badge .label {
  font-size: 11px;
  color: #909399;
}

.stat-badge .val {
  font-size: 16px;
  font-weight: bold;
  color: #409EFF;
}

/* 玻璃拟态卡片 */
.glass-card {
  background: rgba(255, 255, 255, 0.65);
  backdrop-filter: blur(16px) saturate(165%);
  -webkit-backdrop-filter: blur(16px) saturate(165%);
  border: 1px solid rgba(255, 255, 255, 0.45);
  border-radius: 12px;
  box-shadow: 0 8px 32px 0 rgba(31, 38, 135, 0.03);
}

/* 工作台布局 */
.duplicates-workbench {
  flex: 1;
  display: flex;
  gap: 16px;
  overflow: hidden;
  position: relative;
  width: 100%;
}

/* 左侧栏 */
.workbench-left-sidebar {
  width: 250px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.config-card, .volume-card {
  padding: 14px;
}

.card-header-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  padding-bottom: 12px;
  border-bottom: 1px solid rgba(220, 223, 230, 0.4);
  margin-bottom: 12px;
}

.strategy-selector-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.strategy-radios {
  width: 100%;
}

.strategy-radios .el-radio-button {
  flex: 1;
}

.strategy-radios .el-radio-button__inner {
  width: 100%;
  border-radius: 8px !important;
  border: 1px solid rgba(220,223,230,0.5);
  background: rgba(255,255,255,0.4);
  box-shadow: none !important;
}

.strategy-radios .el-radio-button.is-active .el-radio-button__inner {
  background: #409EFF !important;
  color: #ffffff !important;
  border-color: #409EFF !important;
}

.partial-kb-input-area {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px;
  background: rgba(255,255,255,0.4);
  border-radius: 8px;
}

.input-label {
  font-size: 12px;
  color: #606266;
}

.helper-desc {
  font-size: 11px;
  color: #909399;
  line-height: 1.4;
  margin-top: 4px;
}

/* 磁盘列表 */
.volume-list-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.volume-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.45) !important;
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  border: 1px solid rgba(255, 255, 255, 0.45) !important;
  border-radius: 8px;
  font-size: 12px;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.01);
}

.volume-item:hover {
  background: rgba(255, 255, 255, 0.75) !important;
  border-color: rgba(64, 158, 255, 0.3) !important;
  transform: translateY(-1.5px);
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.06);
}

.volume-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.volume-item .indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.volume-item .indicator.active {
  background: #67C23A;
  box-shadow: 0 0 8px #67C23A;
  animation: led-pulse 2s infinite ease-in-out;
}

.volume-item .indicator.inactive {
  background: #e6a23c;
  box-shadow: 0 0 6px #e6a23c;
}

@keyframes led-pulse {
  0% {
    opacity: 0.4;
    box-shadow: 0 0 2px #67C23A;
  }
  50% {
    opacity: 1;
    box-shadow: 0 0 8px #67C23A;
  }
  100% {
    opacity: 0.4;
    box-shadow: 0 0 2px #67C23A;
  }
}

.volume-item .name {
  color: #5e6d82;
  font-weight: 600;
}

.volume-item .letter {
  color: #909399;
  font-family: 'Consolas', 'Monaco', monospace;
  font-weight: 500;
}

.remap-btn {
  width: 100%;
  border-radius: 8px;
  font-weight: 500;
  margin-top: 4px;
  background: rgba(64, 158, 255, 0.1) !important;
  border: 1px solid rgba(64, 158, 255, 0.2) !important;
  color: #409EFF !important;
}

.remap-btn:hover {
  background: #409EFF !important;
  border-color: #409EFF !important;
  color: white !important;
}

/* 中间栏 */
.workbench-middle-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow: hidden;
}

.middle-tabs-row {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.middle-tabs-row .tab-item {
  padding: 8px 16px;
  background: rgba(255, 255, 255, 0.5) !important;
  backdrop-filter: blur(4px) !important;
  -webkit-backdrop-filter: blur(4px) !important;
  border: 1px solid rgba(255, 255, 255, 0.45) !important;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
  color: #606266;
  cursor: pointer;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.middle-tabs-row .tab-item:hover {
  background: rgba(255, 255, 255, 0.85) !important;
  border-color: rgba(64, 158, 255, 0.4) !important;
  color: #409EFF !important;
  transform: translateY(-1px);
}

.middle-tabs-row .tab-item.active {
  background: #409EFF !important;
  border-color: #409EFF !important;
  color: #ffffff !important;
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.2) !important;
}

.list-box-card {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.list-table-header {
  display: flex;
  align-items: center;
  padding: 0 16px;
  height: 40px;
  background: rgba(245, 247, 250, 0.6);
  border-bottom: 1px solid rgba(220, 223, 230, 0.4);
  font-size: 12px;
  font-weight: 600;
  color: #909399;
  flex-shrink: 0;
}

.list-table-body {
  flex: 1;
  overflow: hidden;
  position: relative;
}

/* 表格行样式 */
.row-header-group {
  display: flex;
  align-items: center;
  padding: 0 16px;
  height: 48px;
  background: rgba(240, 245, 255, 0.6);
  border-bottom: 1px solid rgba(64, 158, 255, 0.1);
  font-size: 13px;
  font-weight: 600;
  color: #303133;
  cursor: pointer;
  user-select: none;
  transition: background 0.2s;
}

.row-header-group:hover {
  background: rgba(64, 158, 255, 0.08);
}

.group-toggle-indicator {
  margin-right: 8px;
  color: #909399;
  display: flex;
  align-items: center;
}

.group-desc-info {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
}

.group-badge {
  background: #409EFF;
  color: #ffffff;
  padding: 2px 6px;
  font-size: 10px;
  border-radius: 4px;
  flex-shrink: 0;
}

.group-title-filename {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-detail-txt {
  color: #909399;
  font-weight: normal;
  font-size: 12px;
  white-space: nowrap;
}

.row-file-item {
  display: flex;
  align-items: center;
  padding: 0 16px;
  height: 48px;
  border-bottom: 1px solid rgba(220, 223, 230, 0.3);
  font-size: 12px;
  color: #606266;
  transition: background 0.15s;
  cursor: pointer;
}

.row-file-item:hover {
  background: rgba(64, 158, 255, 0.03);
}

.row-file-item.active {
  background: linear-gradient(90deg, rgba(64, 158, 255, 0.1) 0%, rgba(102, 177, 255, 0.01) 100%) !important;
  border-left: 3px solid #409EFF;
}

/* 列宽分配 */
.col-check {
  width: 40px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
}

.col-name {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  overflow: hidden;
}

.file-icon {
  color: #909399;
  flex-shrink: 0;
}

.file-path-span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #5e6d82;
}

.col-size {
  width: 90px;
  flex-shrink: 0;
  color: #409EFF;
  font-weight: 500;
}

.col-date {
  width: 140px;
  flex-shrink: 0;
  color: #909399;
}

.col-actions {
  width: 110px;
  flex-shrink: 0;
  text-align: right;
}

/* 列表底部控制 */
.list-box-footer {
  height: 46px;
  padding: 0 16px;
  border-top: 1px solid rgba(220, 223, 230, 0.4);
  background: rgba(245, 247, 250, 0.6);
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-shrink: 0;
}

.strategy-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.footer-label-text {
  font-size: 12px;
  color: #909399;
  font-weight: 600;
  margin-right: 4px;
}

.glass-btn-sm {
  background: rgba(255,255,255,0.7);
  border: 1px solid rgba(220,223,230,0.5);
  border-radius: 6px;
  font-size: 11px;
}

.cleanup-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.selected-summary {
  font-size: 12px;
  color: #606266;
}

.selected-summary strong {
  color: #f56c6c;
}

.glass-delete-btn {
  border-radius: 8px;
  font-weight: 600;
}

/* 右侧预览栏 */
.workbench-right-preview {
  width: 300px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
}

.preview-card {
  height: 100%;
  padding: 14px;
  display: flex;
  flex-direction: column;
}

.preview-card-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
}

.image-preview-wrapper {
  width: 100%;
  height: 160px;
  background: rgba(0,0,0,0.03);
  border-radius: 8px;
  display: flex;
  justify-content: center;
  align-items: center;
  overflow: hidden;
  border: 1px solid rgba(220,223,230,0.4);
}

.preview-img-element {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.generic-preview-wrapper {
  width: 100%;
  height: 160px;
  background: rgba(0,0,0,0.02);
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: 8px;
  border: 1px dashed rgba(220,223,230,0.4);
}

.ext-banner {
  font-size: 11px;
  background: #909399;
  color: #ffffff;
  padding: 2px 8px;
  border-radius: 4px;
  font-weight: bold;
}

.metadata-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.meta-row {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.meta-label {
  font-size: 11px;
  color: #909399;
  font-weight: 600;
}

.meta-value {
  font-size: 12px;
  color: #303133;
  word-break: break-all;
  line-height: 1.4;
}

.meta-value.copyable {
  background: rgba(0,0,0,0.02);
  padding: 4px 6px;
  border-radius: 4px;
  border: 1px solid rgba(220,223,230,0.25);
  font-family: monospace;
}

.quick-action-buttons {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: auto;
  padding-top: 12px;
}

.glass-btn-full {
  width: 100%;
  background: rgba(255,255,255,0.7);
  border: 1px solid rgba(220,223,230,0.5);
  border-radius: 8px;
}

.preview-empty-state, .empty-state {
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
  text-align: center;
}

/* 数据库空面板 */
.database-empty-card {
  flex: 1;
  display: flex;
  justify-content: center;
  align-items: center;
}

.go-scan-btn {
  margin-top: 16px;
  border-radius: 8px;
}

.loading-state-wrapper {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  padding: 60px 0;
}

.loading-text {
  margin-top: 12px;
  color: #909399;
  font-size: 13.5px;
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
