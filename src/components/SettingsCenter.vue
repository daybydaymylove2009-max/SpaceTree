<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { 
  Setting, Brush, Document, Bell, 
  Refresh, Check, Moon, Sunny
} from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { useTheme } from '../composables/useTheme';

const props = defineProps<{
  db_path: string;
}>()

// 使用主题管理
const { currentTheme, setTheme, isDark } = useTheme();

// 引入性能和特效管理
import { usePerformance } from '../composables/usePerformance';
const { enableGlassEffect, setGlassEffect } = usePerformance();

// 设置分类
const activeCategory = ref('general');

// 通用设置
const generalSettings = ref({
  autoStart: false,
  minimizeToTray: true,
  language: 'zh-CN',
  theme: currentTheme.value,
  enableDeleteFullAudit: true
});

// 监听主题变化
watch(currentTheme, (newTheme) => {
  generalSettings.value.theme = newTheme;
});

// 扫描设置
const scanSettings = ref({
  defaultMode: 'incremental',
  autoScan: false,
  autoScanInterval: 24,
  excludeHidden: true,
  excludeSystem: true,
  hashAlgorithm: 'xxhash3'
});

// 报告设置
const reportSettings = ref({
  defaultFormat: 'markdown',
  autoGenerate: true,
  complianceStandard: 'iso27001',
  includeCharts: true
});

// 通知设置
const notificationSettings = ref({
  scanComplete: true,
  duplicatesFound: true,
  largeFiles: true,
  errors: true
});

// 存储统计
const storageStats = ref({
  databaseSize: 0,
  scanHistoryCount: 0,
  filesCount: 0,
  logFileSize: 0
});

// 保存设置
async function saveSettings() {
  try {
    // 保存到后端数据库
    await invoke('save_app_settings', {
      db_path: props.db_path,
      general: {
        auto_start: generalSettings.value.autoStart,
        minimize_to_tray: generalSettings.value.minimizeToTray,
        language: generalSettings.value.language,
        theme: generalSettings.value.theme
      },
      scan: {
        default_mode: scanSettings.value.defaultMode,
        auto_scan: scanSettings.value.autoScan,
        auto_scan_interval: scanSettings.value.autoScanInterval,
        exclude_hidden: scanSettings.value.excludeHidden,
        exclude_system: scanSettings.value.excludeSystem,
        hash_algorithm: scanSettings.value.hashAlgorithm
      },
      report: {
        default_format: reportSettings.value.defaultFormat,
        auto_generate: reportSettings.value.autoGenerate,
        compliance_standard: reportSettings.value.complianceStandard,
        include_charts: reportSettings.value.includeCharts
      },
      notification: {
        scan_complete: notificationSettings.value.scanComplete,
        duplicates_found: notificationSettings.value.duplicatesFound,
        large_files: notificationSettings.value.largeFiles,
        errors: notificationSettings.value.errors
      }
    });

    // 同时保存到本地存储作为缓存
    localStorage.setItem('dfh_settings', JSON.stringify({
      general: generalSettings.value,
      scan: scanSettings.value,
      report: reportSettings.value,
      notification: notificationSettings.value
    }));

    ElMessage.success('设置已保存');
  } catch (error) {
    ElMessage.error(`保存失败: ${error}`);
  }
}

// 加载设置
async function loadSettings() {
  try {
    // 优先从后端数据库加载
    const settings = await invoke('load_app_settings', { db_path: props.db_path }) as any;

    if (settings) {
      generalSettings.value = {
        autoStart: settings.general.auto_start,
        minimizeToTray: settings.general.minimize_to_tray,
        language: settings.general.language,
        theme: settings.general.theme,
        enableDeleteFullAudit: generalSettings.value.enableDeleteFullAudit
      };
      scanSettings.value = {
        defaultMode: settings.scan.default_mode,
        autoScan: settings.scan.auto_scan,
        autoScanInterval: settings.scan.auto_scan_interval,
        excludeHidden: settings.scan.exclude_hidden,
        excludeSystem: settings.scan.exclude_system,
        hashAlgorithm: settings.scan.hash_algorithm
      };
      reportSettings.value = {
        defaultFormat: settings.report.default_format,
        autoGenerate: settings.report.auto_generate,
        complianceStandard: settings.report.compliance_standard,
        includeCharts: settings.report.include_charts
      };
      notificationSettings.value = {
        scanComplete: settings.notification.scan_complete,
        duplicatesFound: settings.notification.duplicates_found,
        largeFiles: settings.notification.large_files,
        errors: settings.notification.errors
      };
    }
  } catch (error) {
    console.error('从后端加载设置失败，尝试从本地存储加载', error);
    // 如果后端加载失败，尝试从本地存储加载
    try {
      const saved = localStorage.getItem('dfh_settings');
      if (saved) {
        const settings = JSON.parse(saved);
        generalSettings.value = { ...generalSettings.value, ...settings.general };
        scanSettings.value = { ...scanSettings.value, ...settings.scan };
        reportSettings.value = { ...reportSettings.value, ...settings.report };
        notificationSettings.value = { ...notificationSettings.value, ...settings.notification };
      }
    } catch (localError) {
      console.error('从本地存储加载设置失败', localError);
    }
  }
}

// 清理数据库
async function clearDatabase() {
  try {
    await ElMessageBox.confirm(
      '确定要清空数据库吗？所有扫描记录将被删除，此操作不可恢复！',
      '清空数据库',
      {
        confirmButtonText: '清空',
        cancelButtonText: '取消',
        type: 'error'
      }
    );

    await invoke('clear_database', { db_path: props.db_path });
    ElMessage.success('数据库已清空');
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`清空失败: ${error}`);
    }
  }
}

// 清理历史记录
async function clearHistory() {
  try {
    await ElMessageBox.confirm(
      '确定要清空所有扫描历史吗?',
      '清空历史',
      {
        confirmButtonText: '清空',
        cancelButtonText: '取消',
        type: 'warning'
      }
    );

    await invoke('clear_scan_history', { db_path: props.db_path });
    ElMessage.success('历史记录已清空');
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`清空失败: ${error}`);
    }
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

// 加载存储统计
const lastError = ref<string>('');

async function loadStorageStats() {
  try {
    lastError.value = '';
    console.log('正在加载存储统计, db_path:', props.db_path);

    if (!props.db_path) {
      lastError.value = '数据库路径为空';
      return;
    }

    const stats = await invoke('get_storage_stats', { db_path: props.db_path }) as any;
    console.log('获取到存储统计:', stats);

    // 确保数值正确转换
    storageStats.value = {
      databaseSize: Number(stats.database_size) || 0,
      scanHistoryCount: Number(stats.scan_history_count) || 0,
      filesCount: Number(stats.files_count) || 0,
      logFileSize: Number(stats.log_file_size) || 0
    };

    console.log('更新后的存储统计:', storageStats.value);
  } catch (error: any) {
    console.error('加载存储统计失败', error);
    lastError.value = String(error);
    ElMessage.error('加载存储统计失败: ' + error);
  }
}

// 处理存储管理分类点击
async function handleStorageCategoryClick() {
  activeCategory.value = 'storage';
  // 切换到存储管理时刷新数据
  await loadStorageStats();
}

onMounted(async () => {
  await loadSettings();
  await loadStorageStats();
});
</script>

<template>
  <div class="settings-center">
    <!-- 页面头部 -->
    <div class="page-header">
      <div class="header-title">
        <h2>设置</h2>
        <p class="header-subtitle">配置应用程序偏好</p>
      </div>
      <el-button type="primary" @click="saveSettings">
        <el-icon><Check /></el-icon>
        保存设置
      </el-button>
    </div>

    <div class="settings-layout">
      <!-- 左侧分类菜单 -->
      <aside class="settings-sidebar">
        <div 
          class="settings-category" 
          :class="{ 'is-active': activeCategory === 'general' }"
          @click="activeCategory = 'general'"
        >
          <el-icon><Setting /></el-icon>
          <span>通用设置</span>
        </div>
        <div 
          class="settings-category" 
          :class="{ 'is-active': activeCategory === 'scan' }"
          @click="activeCategory = 'scan'"
        >
          <el-icon><Refresh /></el-icon>
          <span>扫描设置</span>
        </div>
        <div 
          class="settings-category" 
          :class="{ 'is-active': activeCategory === 'report' }"
          @click="activeCategory = 'report'"
        >
          <el-icon><Document /></el-icon>
          <span>报告设置</span>
        </div>
        <div 
          class="settings-category" 
          :class="{ 'is-active': activeCategory === 'notification' }"
          @click="activeCategory = 'notification'"
        >
          <el-icon><Bell /></el-icon>
          <span>通知设置</span>
        </div>
        <div
          class="settings-category"
          :class="{ 'is-active': activeCategory === 'storage' }"
          @click="handleStorageCategoryClick"
        >
          <el-icon><Document /></el-icon>
          <span>存储管理</span>
        </div>
      </aside>

      <!-- 右侧设置内容 -->
      <main class="settings-content">
        <!-- 通用设置 -->
        <div v-show="activeCategory === 'general'" class="settings-panel">
          <h3>通用设置</h3>
          
          <el-form :model="generalSettings" label-position="top" class="settings-form">
            <el-form-item label="语言">
              <el-select v-model="generalSettings.language" style="width: 200px">
                <el-option label="简体中文" value="zh-CN" />
                <el-option label="English" value="en-US" />
              </el-select>
            </el-form-item>

            <el-form-item label="主题">
              <el-radio-group v-model="generalSettings.theme" @change="(val: string) => setTheme(val as any)">
                <el-radio-button label="light">
                  <el-icon><Sunny /></el-icon> 浅色
                </el-radio-button>
                <el-radio-button label="dark">
                  <el-icon><Moon /></el-icon> 深色
                </el-radio-button>
                <el-radio-button label="auto">自动</el-radio-button>
              </el-radio-group>
              <div class="theme-preview" style="margin-top: 8px;">
                <el-tag v-if="isDark" type="info" size="small">当前: 深色模式</el-tag>
                <el-tag v-else type="success" size="small">当前: 浅色模式</el-tag>
              </div>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="generalSettings.minimizeToTray">
                最小化到系统托盘
              </el-checkbox>
            </el-form-item>

            <el-form-item label="视觉特效">
              <el-checkbox :model-value="enableGlassEffect" @update:model-value="setGlassEffect">
                启用高级磨砂玻璃特效 (若老旧电脑界面卡顿建议关闭)
              </el-checkbox>
            </el-form-item>

            <el-form-item label="去重安全审计">
              <el-checkbox v-model="generalSettings.enableDeleteFullAudit">
                物理删除前启用全文件哈希安全审计 (默认开启，对齐标杆防误删底线)
              </el-checkbox>
            </el-form-item>
          </el-form>
        </div>

        <!-- 扫描设置 -->
        <div v-show="activeCategory === 'scan'" class="settings-panel">
          <h3>扫描设置</h3>
          
          <el-form :model="scanSettings" label-position="top" class="settings-form">
            <el-form-item label="默认扫描模式">
              <el-radio-group v-model="scanSettings.defaultMode">
                <el-radio-button label="incremental">增量扫描</el-radio-button>
                <el-radio-button label="full">全量扫描</el-radio-button>
              </el-radio-group>
            </el-form-item>

            <el-form-item label="哈希算法">
              <el-select v-model="scanSettings.hashAlgorithm" style="width: 200px">
                <el-option label="XXH3 (推荐，最快)" value="xxhash3" />
                <el-option label="MD5 (标准)" value="md5" />
                <el-option label="SHA256 (最安全)" value="sha256" />
              </el-select>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="scanSettings.excludeHidden">
                默认排除隐藏文件
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="scanSettings.excludeSystem">
                默认排除系统文件
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="scanSettings.autoScan">
                启用自动扫描
              </el-checkbox>
              <div class="form-hint">每隔指定时间自动执行增量扫描</div>
            </el-form-item>

            <el-form-item v-if="scanSettings.autoScan" label="自动扫描间隔(小时)">
              <el-slider v-model="scanSettings.autoScanInterval" :min="1" :max="168" show-stops />
            </el-form-item>
          </el-form>
        </div>

        <!-- 报告设置 -->
        <div v-show="activeCategory === 'report'" class="settings-panel">
          <h3>报告设置</h3>
          
          <el-form :model="reportSettings" label-position="top" class="settings-form">
            <el-form-item label="默认导出格式">
              <el-radio-group v-model="reportSettings.defaultFormat">
                <el-radio-button label="markdown">Markdown</el-radio-button>
                <el-radio-button label="csv">CSV</el-radio-button>
                <el-radio-button label="json">JSON</el-radio-button>
              </el-radio-group>
            </el-form-item>

            <el-form-item label="合规标准">
              <el-select v-model="reportSettings.complianceStandard" style="width: 250px">
                <el-option label="ISO 27001" value="iso27001" />
                <el-option label="GDPR" value="gdpr" />
                <el-option label="SOX" value="sox" />
                <el-option label="HIPAA" value="hipaa" />
              </el-select>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="reportSettings.autoGenerate">
                扫描完成后自动生成报告
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="reportSettings.includeCharts">
                报告中包含图表
              </el-checkbox>
            </el-form-item>
          </el-form>
        </div>

        <!-- 通知设置 -->
        <div v-show="activeCategory === 'notification'" class="settings-panel">
          <h3>通知设置</h3>
          
          <el-form :model="notificationSettings" label-position="top" class="settings-form">
            <el-form-item>
              <el-checkbox v-model="notificationSettings.scanComplete">
                扫描完成通知
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="notificationSettings.duplicatesFound">
                发现重复文件时通知
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="notificationSettings.largeFiles">
                发现大文件时通知
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="notificationSettings.errors">
                发生错误时通知
              </el-checkbox>
            </el-form-item>
          </el-form>
        </div>

        <!-- 存储管理 -->
        <div v-show="activeCategory === 'storage'" class="settings-panel">
          <div class="storage-header">
            <h3>存储管理</h3>
            <el-button type="primary" size="small" @click="loadStorageStats">
              <el-icon><Refresh /></el-icon>
              刷新
            </el-button>
          </div>



          <div class="storage-stats">
            <el-descriptions :column="1" border>
              <el-descriptions-item label="数据库大小">
                {{ formatSize(storageStats.databaseSize) }}
              </el-descriptions-item>
              <el-descriptions-item label="文件记录数">
                {{ storageStats.filesCount }} 条
              </el-descriptions-item>
              <el-descriptions-item label="扫描历史记录">
                {{ storageStats.scanHistoryCount }} 条
              </el-descriptions-item>
              <el-descriptions-item label="日志文件大小">
                {{ formatSize(storageStats.logFileSize) }}
              </el-descriptions-item>
            </el-descriptions>
          </div>

          <div class="storage-actions">
            <h4>数据清理</h4>
            <div class="action-list">
              <div class="action-item">
                <div class="action-info">
                  <span class="action-title">清空数据库</span>
                  <span class="action-desc">删除所有扫描记录和文件信息</span>
                </div>
                <el-button type="danger" plain @click="clearDatabase">
                  <el-icon><Brush /></el-icon>
                  清空
                </el-button>
              </div>

              <div class="action-item">
                <div class="action-info">
                  <span class="action-title">清空扫描历史</span>
                  <span class="action-desc">保留文件数据，仅删除历史记录</span>
                </div>
                <el-button type="warning" plain @click="clearHistory">
                  <el-icon><Refresh /></el-icon>
                  清空
                </el-button>
              </div>
            </div>
          </div>
        </div>
      </main>
    </div>
  </div>
</template>

<style scoped>
.settings-center {
  width: 100%;
  min-width: 100%;
  max-width: 100%;
  height: 100%;
  box-sizing: border-box;
}

/* 确保所有直接子元素填满宽度 */
.settings-center > * {
  width: 100% !important;
  min-width: 100% !important;
  max-width: 100% !important;
  box-sizing: border-box;
}

/* 确保所有卡片填满宽?*/
.settings-center :deep(.el-card) {
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

/* 设置布局 */
.settings-layout {
  display: flex;
  gap: 24px;
  background: #ffffff;
  border-radius: 12px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
  overflow: hidden;
}

/* 侧边栏 */
.settings-sidebar {
  width: 200px;
  background: #f5f7fa;
  padding: 16px 0;
  flex-shrink: 0;
}

.settings-category {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 20px;
  cursor: pointer;
  transition: all 0.2s ease;
  color: #606266;
}

.settings-category:hover {
  background: #e4e7ed;
  color: #409EFF;
}

.settings-category.is-active {
  background: #ffffff;
  color: #409EFF;
  border-right: 3px solid #409EFF;
}

/* 设置内容 */
.settings-content {
  flex: 1;
  padding: 24px;
  min-height: 500px;
}

.settings-panel h3 {
  margin: 0 0 24px 0;
  font-size: 18px;
  font-weight: 600;
  color: #303133;
  padding-bottom: 12px;
  border-bottom: 1px solid #e4e7ed;
}

.settings-form {
  max-width: 500px;
}

.form-hint {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}

/* 存储统计 */
.storage-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.storage-header h3 {
  margin: 0;
}

.storage-stats {
  margin-bottom: 32px;
}

.storage-actions h4 {
  margin: 0 0 16px 0;
  font-size: 14px;
  color: #303133;
}

.action-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.action-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  background: #f5f7fa;
  border-radius: 8px;
}

.action-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.action-title {
  font-weight: 500;
  color: #303133;
}

.action-desc {
  font-size: 12px;
  color: #909399;
}

/* 响应式 */
@media (max-width: 768px) {
  .settings-layout {
    flex-direction: column;
  }
  
  .settings-sidebar {
    width: 100%;
    display: flex;
    overflow-x: auto;
    padding: 8px;
  }
  
  .settings-category {
    white-space: nowrap;
    padding: 8px 16px;
  }
  
  .settings-category.is-active {
    border-right: none;
    border-bottom: 3px solid #409EFF;
  }
}
</style>



