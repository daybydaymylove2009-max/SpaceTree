<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { 
  Setting, Brush, Document, Bell, 
  Refresh, Check, Moon, Sunny
} from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { useTheme } from '../composables/useTheme';
import { t, setLanguage, getLanguage, type Language } from '../utils/i18n';

const props = defineProps<{
  db_path: string;
}>()

// 强制组件刷新 key，配合手写语言包达成无刷新秒切
const forceUpdateKey = ref(0);
const onLanguageChange = () => {
  forceUpdateKey.value++;
};

// 使用主题管理
const { currentTheme, setTheme, isDark } = useTheme();

// 性能和特效管理
import { usePerformance } from '../composables/usePerformance';
const { enableGlassEffect, setGlassEffect } = usePerformance();

// 设置分类
const activeCategory = ref('general');

// 通用设置
const generalSettings = ref({
  autoStart: false,
  minimizeToTray: true,
  language: getLanguage(),
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

// 处理语言切换
function handleLanguageChange(lang: string) {
  setLanguage(lang as Language);
}

// 保存设置
async function saveSettings() {
  try {
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

    localStorage.setItem('dfh_settings', JSON.stringify({
      general: generalSettings.value,
      scan: scanSettings.value,
      report: reportSettings.value,
      notification: notificationSettings.value
    }));

    ElMessage.success(t('common.success'));
  } catch (error) {
    ElMessage.error(`${t('common.error')}: ${error}`);
  }
}

// 加载设置
async function loadSettings() {
  try {
    const settings = await invoke('load_app_settings', { db_path: props.db_path }) as any;
    if (settings) {
      generalSettings.value = {
        autoStart: settings.general.auto_start,
        minimizeToTray: settings.general.minimize_to_tray,
        language: settings.general.language || getLanguage(),
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
      t('zh-CN' === getLanguage() ? '确定要清空数据库吗？所有扫描记录将被删除，此操作不可恢复！' : 'Are you sure you want to clear the database? All records will be deleted permanently!'),
      t('zh-CN' === getLanguage() ? '清空数据库' : 'Clear Database'),
      {
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
        type: 'error'
      }
    );

    await invoke('clear_database', { db_path: props.db_path });
    ElMessage.success(t('common.success'));
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`${t('common.error')}: ${error}`);
    }
  }
}

// 清理历史记录
async function clearHistory() {
  try {
    await ElMessageBox.confirm(
      t('zh-CN' === getLanguage() ? '确定要清空所有扫描历史吗?' : 'Are you sure you want to clear all scan history?'),
      t('zh-CN' === getLanguage() ? '清空历史' : 'Clear History'),
      {
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
        type: 'warning'
      }
    );

    await invoke('clear_scan_history', { db_path: props.db_path });
    ElMessage.success(t('common.success'));
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error(`${t('common.error')}: ${error}`);
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

const lastError = ref<string>('');

async function loadStorageStats() {
  try {
    lastError.value = '';
    if (!props.db_path) {
      lastError.value = 'Database path is empty';
      return;
    }

    const stats = await invoke('get_storage_stats', { db_path: props.db_path }) as any;
    storageStats.value = {
      databaseSize: Number(stats.database_size) || 0,
      scanHistoryCount: Number(stats.scan_history_count) || 0,
      filesCount: Number(stats.files_count) || 0,
      logFileSize: Number(stats.log_file_size) || 0
    };
  } catch (error: any) {
    console.error('加载存储统计失败', error);
    lastError.value = String(error);
  }
}

async function handleStorageCategoryClick() {
  activeCategory.value = 'storage';
  await loadStorageStats();
}

onMounted(async () => {
  window.addEventListener('app-lang-change', onLanguageChange);
  await loadSettings();
  await loadStorageStats();
});

onUnmounted(() => {
  window.removeEventListener('app-lang-change', onLanguageChange);
});
</script>

<template>
  <div class="settings-center" :key="forceUpdateKey">
    <!-- 页面头部 -->
    <div class="page-header">
      <div class="header-title">
        <h2>{{ t('menu.settings') }}</h2>
        <p class="header-subtitle">{{ t('settings.subtitle') }}</p>
      </div>
      <el-button type="primary" @click="saveSettings">
        <el-icon><Check /></el-icon>
        {{ t('common.confirm') }}
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
          <span>{{ t('settings.title') }}</span>
        </div>
        <div 
          class="settings-category" 
          :class="{ 'is-active': activeCategory === 'scan' }"
          @click="activeCategory = 'scan'"
        >
          <el-icon><Refresh /></el-icon>
          <span>{{ t('menu.scan') }}</span>
        </div>
        <div 
          class="settings-category" 
          :class="{ 'is-active': activeCategory === 'report' }"
          @click="activeCategory = 'report'"
        >
          <el-icon><Document /></el-icon>
          <span>{{ t('analysis.title') }}</span>
        </div>
        <div 
          class="settings-category" 
          :class="{ 'is-active': activeCategory === 'notification' }"
          @click="activeCategory = 'notification'"
        >
          <el-icon><Bell /></el-icon>
          <span>{{ t('zh-CN' === getLanguage() ? '通知设置' : 'Notifications') }}</span>
        </div>
        <div
          class="settings-category"
          :class="{ 'is-active': activeCategory === 'storage' }"
          @click="handleStorageCategoryClick"
        >
          <el-icon><Document /></el-icon>
          <span>{{ t('zh-CN' === getLanguage() ? '存储管理' : 'Storage') }}</span>
        </div>
      </aside>

      <!-- 右侧设置内容 -->
      <main class="settings-content">
        <!-- 通用设置 -->
        <div v-show="activeCategory === 'general'" class="settings-panel">
          <h3>{{ t('settings.title') }}</h3>
          
          <el-form :model="generalSettings" label-position="top" class="settings-form">
            <el-form-item :label="t('settings.langTitle')">
              <el-select v-model="generalSettings.language" style="width: 200px" @change="handleLanguageChange">
                <el-option :label="t('settings.zh')" value="zh-CN" />
                <el-option :label="t('settings.en')" value="en-US" />
              </el-select>
              <div class="form-hint">{{ t('settings.langDesc') }}</div>
            </el-form-item>

            <el-form-item :label="t('settings.themeTitle')">
              <el-radio-group v-model="generalSettings.theme" @change="(val: string) => setTheme(val as any)">
                <el-radio-button label="light">
                  <el-icon><Sunny /></el-icon> {{ t('zh-CN' === getLanguage() ? '浅色' : 'Light') }}
                </el-radio-button>
                <el-radio-button label="dark">
                  <el-icon><Moon /></el-icon> {{ t('zh-CN' === getLanguage() ? '深色' : 'Dark') }}
                </el-radio-button>
                <el-radio-group label="auto">{{ t('zh-CN' === getLanguage() ? '自动' : 'Auto') }}</el-radio-group>
              </el-radio-group>
              <div class="theme-preview" style="margin-top: 8px;">
                <el-tag v-if="isDark" type="info" size="small">{{ t('zh-CN' === getLanguage() ? '当前: 深色模式' : 'Active: Dark') }}</el-tag>
                <el-tag v-else type="success" size="small">{{ t('zh-CN' === getLanguage() ? '当前: 浅色模式' : 'Active: Light') }}</el-tag>
              </div>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="generalSettings.minimizeToTray">
                {{ t('zh-CN' === getLanguage() ? '最小化到系统托盘' : 'Minimize to system tray') }}
              </el-checkbox>
            </el-form-item>

            <el-form-item :label="t('zh-CN' === getLanguage() ? '视觉特效' : 'Visual Effects')">
              <el-checkbox :model-value="enableGlassEffect" @update:model-value="setGlassEffect">
                {{ t('zh-CN' === getLanguage() ? '启用高级磨砂玻璃特效 (若老旧电脑界面卡顿建议关闭)' : 'Enable glassmorphism window effects (disable if laggy)') }}
              </el-checkbox>
            </el-form-item>

            <el-form-item :label="t('zh-CN' === getLanguage() ? '去重安全审计' : 'Deduplication Safety')">
              <el-checkbox v-model="generalSettings.enableDeleteFullAudit">
                {{ t('zh-CN' === getLanguage() ? '物理删除前启用全文件哈希安全审计 (默认开启，对齐标杆防误删底线)' : 'Perform byte-by-byte verification before file deletion (highly recommended)') }}
              </el-checkbox>
            </el-form-item>
          </el-form>
        </div>

        <!-- 扫描设置 -->
        <div v-show="activeCategory === 'scan'" class="settings-panel">
          <h3>{{ t('menu.scan') }}</h3>
          
          <el-form :model="scanSettings" label-position="top" class="settings-form">
            <el-form-item :label="t('zh-CN' === getLanguage() ? '默认扫描模式' : 'Default Scan Mode')">
              <el-radio-group v-model="scanSettings.defaultMode">
                <el-radio-button label="incremental">{{ t('zh-CN' === getLanguage() ? '增量扫描' : 'Incremental') }}</el-radio-button>
                <el-radio-button label="full">{{ t('zh-CN' === getLanguage() ? '全量扫描' : 'Full Scan') }}</el-radio-button>
              </el-radio-group>
            </el-form-item>

            <el-form-item :label="t('zh-CN' === getLanguage() ? '哈希算法' : 'Hash Algorithm')">
              <el-select v-model="scanSettings.hashAlgorithm" style="width: 200px">
                <el-option label="XXH3 (Fastest)" value="xxhash3" />
                <el-option label="MD5 (Legacy)" value="md5" />
                <el-option label="SHA256 (Safest)" value="sha256" />
              </el-select>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="scanSettings.excludeHidden">
                {{ t('scan.excludeHidden') }}
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="scanSettings.excludeSystem">
                {{ t('zh-CN' === getLanguage() ? '默认排除系统文件' : 'Exclude system directories') }}
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="scanSettings.autoScan">
                {{ t('zh-CN' === getLanguage() ? '启用自动扫描' : 'Enable scheduled background scanning') }}
              </el-checkbox>
              <div class="form-hint">{{ t('zh-CN' === getLanguage() ? '每隔指定时间自动执行增量扫描' : 'Scans specified directories periodically') }}</div>
            </el-form-item>

            <el-form-item v-if="scanSettings.autoScan" :label="t('zh-CN' === getLanguage() ? '自动扫描间隔(小时)' : 'Schedule Interval (Hours)')">
              <el-slider v-model="scanSettings.autoScanInterval" :min="1" :max="168" show-stops />
            </el-form-item>
          </el-form>
        </div>

        <!-- 报告设置 -->
        <div v-show="activeCategory === 'report'" class="settings-panel">
          <h3>{{ t('analysis.title') }}</h3>
          
          <el-form :model="reportSettings" label-position="top" class="settings-form">
            <el-form-item :label="t('zh-CN' === getLanguage() ? '默认导出格式' : 'Default Export Format')">
              <el-radio-group v-model="reportSettings.defaultFormat">
                <el-radio-button label="markdown">Markdown</el-radio-button>
                <el-radio-button label="csv">CSV</el-radio-button>
                <el-radio-button label="json">JSON</el-radio-button>
              </el-radio-group>
            </el-form-item>

            <el-form-item :label="t('zh-CN' === getLanguage() ? '合规标准' : 'Compliance Standards')">
              <el-select v-model="reportSettings.complianceStandard" style="width: 250px">
                <el-option label="ISO 27001" value="iso27001" />
                <el-option label="GDPR" value="gdpr" />
                <el-option label="SOX" value="sox" />
                <el-option label="HIPAA" value="hipaa" />
              </el-select>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="reportSettings.autoGenerate">
                {{ t('zh-CN' === getLanguage() ? '扫描完成后自动生成报告' : 'Automatically generate reports after scan') }}
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="reportSettings.includeCharts">
                {{ t('zh-CN' === getLanguage() ? '报告中包含图表' : 'Include charts in report') }}
              </el-checkbox>
            </el-form-item>
          </el-form>
        </div>

        <!-- 通知设置 -->
        <div v-show="activeCategory === 'notification'" class="settings-panel">
          <h3>{{ t('zh-CN' === getLanguage() ? '通知设置' : 'Notification Triggers') }}</h3>
          
          <el-form :model="notificationSettings" label-position="top" class="settings-form">
            <el-form-item>
              <el-checkbox v-model="notificationSettings.scanComplete">
                {{ t('zh-CN' === getLanguage() ? '扫描完成通知' : 'Notify when scanning completes') }}
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="notificationSettings.duplicatesFound">
                {{ t('zh-CN' === getLanguage() ? '发现重复文件时通知' : 'Notify when duplicate blocks found') }}
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="notificationSettings.largeFiles">
                {{ t('zh-CN' === getLanguage() ? '发现大文件时通知' : 'Notify when large file detected') }}
              </el-checkbox>
            </el-form-item>

            <el-form-item>
              <el-checkbox v-model="notificationSettings.errors">
                {{ t('zh-CN' === getLanguage() ? '发生错误时通知' : 'Notify on errors and warnings') }}
              </el-checkbox>
            </el-form-item>
          </el-form>
        </div>

        <!-- 存储管理 -->
        <div v-show="activeCategory === 'storage'" class="settings-panel">
          <div class="storage-header">
            <h3>{{ t('zh-CN' === getLanguage() ? '存储管理' : 'Storage Management') }}</h3>
            <el-button type="primary" size="small" @click="loadStorageStats">
              <el-icon><Refresh /></el-icon>
              {{ t('zh-CN' === getLanguage() ? '刷新' : 'Refresh') }}
            </el-button>
          </div>

          <div class="storage-stats">
            <el-descriptions :column="1" border>
              <el-descriptions-item :label="t('zh-CN' === getLanguage() ? '数据库大小' : 'Database Size')">
                {{ formatSize(storageStats.databaseSize) }}
              </el-descriptions-item>
              <el-descriptions-item :label="t('zh-CN' === getLanguage() ? '文件记录数' : 'File Records Count')">
                {{ storageStats.filesCount }}
              </el-descriptions-item>
              <el-descriptions-item :label="t('zh-CN' === getLanguage() ? '扫描历史记录' : 'Scan History count')">
                {{ storageStats.scanHistoryCount }}
              </el-descriptions-item>
              <el-descriptions-item :label="t('zh-CN' === getLanguage() ? '日志文件大小' : 'Log Size')">
                {{ formatSize(storageStats.logFileSize) }}
              </el-descriptions-item>
            </el-descriptions>
          </div>

          <div class="storage-actions">
            <h4>{{ t('zh-CN' === getLanguage() ? '数据清理' : 'Data Pruning') }}</h4>
            <div class="action-list">
              <div class="action-item">
                <div class="action-info">
                  <span class="action-title">{{ t('zh-CN' === getLanguage() ? '清空数据库' : 'Reset Database') }}</span>
                  <span class="action-desc">{{ t('zh-CN' === getLanguage() ? '删除所有扫描记录和文件信息' : 'Clears all file index tables permanently') }}</span>
                </div>
                <el-button type="danger" plain @click="clearDatabase">
                  <el-icon><Brush /></el-icon>
                  {{ t('zh-CN' === getLanguage() ? '清空' : 'Reset') }}
                </el-button>
              </div>

              <div class="action-item">
                <div class="action-info">
                  <span class="action-title">{{ t('zh-CN' === getLanguage() ? '清空扫描历史' : 'Prune Scan History') }}</span>
                  <span class="action-desc">{{ t('zh-CN' === getLanguage() ? '保留文件数据，仅删除历史记录' : 'Removes timestamps, keeping indexes intact') }}</span>
                </div>
                <el-button type="warning" plain @click="clearHistory">
                  <el-icon><Refresh /></el-icon>
                  {{ t('zh-CN' === getLanguage() ? '清空' : 'Prune') }}
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

.settings-center > * {
  width: 100% !important;
  min-width: 100% !important;
  max-width: 100% !important;
  box-sizing: border-box;
}

.settings-center :deep(.el-card) {
  width: 100% !important;
  min-width: 100% !important;
  max-width: 100% !important;
}

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
  color: var(--el-text-color-primary);
}

.header-subtitle {
  color: var(--el-text-color-secondary);
  font-size: 14px;
}

.settings-layout {
  display: flex;
  gap: 24px;
  background: var(--el-bg-color-overlay);
  border: 1px solid var(--el-border-color-light);
  border-radius: 12px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.04);
  overflow: hidden;
}

.settings-sidebar {
  width: 200px;
  background: var(--el-fill-color-light);
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
  color: var(--el-text-color-regular);
}

.settings-category:hover {
  background: var(--el-fill-color-darker);
  color: var(--el-color-primary);
}

.settings-category.is-active {
  background: var(--el-bg-color-overlay);
  color: var(--el-color-primary);
  border-right: 3px solid var(--el-color-primary);
}

.settings-content {
  flex: 1;
  padding: 24px;
  min-height: 500px;
}

.settings-panel h3 {
  margin: 0 0 24px 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  padding-bottom: 12px;
  border-bottom: 1px solid var(--el-border-color-light);
}

.settings-form {
  max-width: 500px;
}

.form-hint {
  font-size: 12px;
  color: var(--el-text-color-placeholder);
  margin-top: 4px;
}

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
  color: var(--el-text-color-primary);
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
  background: var(--el-fill-color-blank);
  border: 1px solid var(--el-border-color-light);
  border-radius: 8px;
}

.action-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  text-align: left;
}

.action-title {
  font-weight: 500;
  color: var(--el-text-color-primary);
}

.action-desc {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

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
    border-bottom: 3px solid var(--el-color-primary);
  }
}
</style>
