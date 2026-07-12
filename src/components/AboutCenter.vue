<script setup lang="ts">
/**
 * 关于中心 - SpaceTree v3.40.0
 * @component AboutCenter
 * @description 展示版本变化、学术架构特性、技术栈亮点以及更新日志，整体采用现代玻璃拟态（Glassmorphism）视觉设计
 */
import { ref, onMounted, onUnmounted } from 'vue';
import { ElMessage } from 'element-plus';
import {
  InfoFilled, CopyDocument, Document,
  Calendar, Cpu, Star, Trophy, Lightning,
  Medal, EditPen, Coin, Files, Refresh
} from '@element-plus/icons-vue';
import { getVersion, getTauriVersion } from '@tauri-apps/api/app';
import { t, getLanguage } from '../utils/i18n';

// 强制更新 key
const forceUpdateKey = ref(0);
const onLanguageChange = () => {
  forceUpdateKey.value++;
};

// 版本信息
const appVersion = ref('3.50.0');
const tauriVersion = ref('');
const rustVersion = ref('1.77.2');
const buildDate = ref('2026-07-12');

// 应用信息
const appInfo = {
  name: 'SpaceTree',
  fullName: 'SpaceTree - Extreme Disk Treemap Utility',
  license: 'MIT License',
  copyright: '© 2026 呆若木鸡. All rights reserved.'
};

// 技术栈
const techStack = [
  { name: 'Tauri FFI', version: '2.10.3', description: 'Rust 驱动的桌面应用内核', icon: '🦀', color: '#FF6B6B' },
  { name: 'Vue.js 3', version: '3.5.30', description: '声明式前端视图开发框架', icon: '💚', color: '#4FC08D' },
  { name: 'Element Plus', version: '2.13.6', description: '高颜值 Vue3 组件库', icon: '🔷', color: '#409EFF' },
  { name: 'Rust Win-FFI', version: '1.77.2', description: '物理驱动器底层 FFI 模块', icon: '⚙️', color: '#DEA584' },
  { name: 'SQLite WAL', version: '3.45', description: '开启 WAL 的高并发数据库', icon: '🗄️', color: '#003B57' },
  { name: 'TypeScript', version: '5.9', description: '强类型 JavaScript 开发超集', icon: '🔷', color: '#3178C6' }
];

// 主要功能
const features = [
  { title: 'zh-CN' === getLanguage() ? 'USN 级枚举' : 'MFT USN speedup', description: 'zh-CN' === getLanguage() ? '支持特权级物理磁盘 USN 日志秒级枚举，扫描提速 100 倍' : 'Instant MFT enumeration on NTFS filesystem', icon: Lightning, color: '#409EFF' },
  { title: 'zh-CN' === getLanguage() ? '部分哈希查重' : 'Partial Match', description: 'zh-CN' === getLanguage() ? '支持设置前 K 字节哈希比对，完美兼容截断或损毁文件' : 'Allows matching first K bytes of files', icon: Star, color: '#E6A23C' },
  { title: 'zh-CN' === getLanguage() ? '盘符漂移自愈' : 'Path Healing', description: 'zh-CN' === getLanguage() ? '自动比对卷 GUID，毫秒级快速纠正拔插U盘的断联路径' : 'Self-heals USB/HDD paths on GUID mismatch', icon: Refresh, color: '#67C23A' },
  { title: 'zh-CN' === getLanguage() ? '自研虚拟滚动' : '1D Virtual Scroll', description: 'zh-CN' === getLanguage() ? '扁平一维虚拟列表渲染，百万数据下保持 60 FPS 顺滑' : 'Flattens duplicate groups in a 60 FPS list', icon: Files, color: '#909399' },
  { title: 'zh-CN' === getLanguage() ? '无锁规约哈希' : 'Parallel Rayon', description: 'zh-CN' === getLanguage() ? 'Rayon 并行规约引擎无锁计算哈希，榨干多核多线程性能' : 'Zero lock path hashing maximizing CPU throughput', icon: Cpu, color: '#F56C6C' },
  { title: 'zh-CN' === getLanguage() ? '实时去抖检索' : 'Debounced Search', description: 'zh-CN' === getLanguage() ? '150ms 动态防抖搜索，支持正则过滤与关键词多重高亮' : '150ms delay indexing with regex support', icon: Document, color: '#8E44AD' }
];

// 核心亮点
const highlights = [
  { label: 'zh-CN' === getLanguage() ? '扫描处理速度' : 'Scan Speed', value: 'zh-CN' === getLanguage() ? '百万/秒' : '1M+ files/s', desc: 'MFT/USN Indexer', icon: Trophy },
  { label: 'zh-CN' === getLanguage() ? '物理路径纠错' : 'Path Resolution', value: 'zh-CN' === getLanguage() ? '毫秒级' : 'ms level', desc: 'GUID volume mapper', icon: Refresh },
  { label: 'zh-CN' === getLanguage() ? '界面滚动帧率' : 'Scroll FPS', value: '60 FPS', desc: '1D Flatten list', icon: Lightning },
  { label: 'zh-CN' === getLanguage() ? '本地数据库' : 'Database engine', value: 'WAL Mode', desc: 'SQLite concurrency', icon: Medal }
];

// 复制版本信息
async function copyVersionInfo() {
  const info = `Application: ${appInfo.name}
Version: ${appVersion.value}
Tauri Core: ${tauriVersion.value || '2.10.3'}
Rust Toolchain: ${rustVersion.value}
Build Date: ${buildDate.value}`;

  try {
    await navigator.clipboard.writeText(info);
    ElMessage.success(t('common.copySuccess'));
  } catch (error) {
    ElMessage.error(t('common.error') + ': ' + error);
  }
}

// 打开外部链接
function openExternalLink(url: string) {
  window.open(url, '_blank');
}

// 获取版本信息
async function loadVersionInfo() {
  try {
    appVersion.value = await getVersion();
    tauriVersion.value = await getTauriVersion();
  } catch (error) {
    console.error('获取版本信息失败:', error);
  }
}

onMounted(() => {
  window.addEventListener('app-lang-change', onLanguageChange);
  loadVersionInfo();
});

onUnmounted(() => {
  window.removeEventListener('app-lang-change', onLanguageChange);
});
</script>

<template>
  <div class="about-center-glass" :key="forceUpdateKey">
    <!-- 玻璃英雄头部 -->
    <div class="about-hero-glass">
      <div class="hero-background">
        <div class="gradient-orb orb-1"></div>
        <div class="gradient-orb orb-2"></div>
      </div>

      <div class="hero-content">
        <div class="app-logo-container">
          <div class="app-logo">
            <el-icon :size="56" color="#fff"><InfoFilled /></el-icon>
          </div>
          <div class="version-badge">
            <el-tag type="danger" effect="dark" size="large" round>v{{ appVersion }}</el-tag>
          </div>
        </div>

        <h1 class="app-name">{{ appInfo.name }}</h1>
        <p class="app-fullname">{{ appInfo.fullName }}</p>
        <p class="app-description">{{ t('about.subtitle') }}</p>

        <div class="hero-actions">
          <el-button
            type="primary"
            size="large"
            round
            @click="copyVersionInfo"
            :icon="CopyDocument"
            class="copy-btn"
          >
            {{ t('about.copyInfo') }}
          </el-button>
          <el-button
            size="large"
            round
            @click="openExternalLink('https://github.com/daybydaymylove2009-max/SpaceTree')"
            class="github-btn"
          >
            <template #icon><span>🐙</span></template>
            {{ t('about.website') }}
          </el-button>
        </div>
      </div>
    </div>

    <!-- 极客技术仪表盘（核心亮点） -->
    <div class="highlights-section">
      <div class="highlights-dashboard-grid">
        <div 
          v-for="(item, index) in highlights" 
          :key="index" 
          class="glass-card tech-stat-widget"
          :style="{ animationDelay: `${index * 0.08}s` }"
        >
          <div class="widget-icon-wrapper" :style="{ background: `linear-gradient(135deg, ${['#409EFF', '#67C23A', '#E6A23C', '#F56C6C'][index]}12, ${['#409EFF', '#67C23A', '#E6A23C', '#F56C6C'][index]}25)` }">
            <el-icon :size="20" :color="['#409EFF', '#67C23A', '#E6A23C', '#F56C6C'][index]">
              <component :is="item.icon" />
            </el-icon>
          </div>
          <div class="widget-body">
            <div class="widget-value" :class="`value-color-${index}`">{{ item.value }}</div>
            <div class="widget-label">{{ item.label }}</div>
            <div class="widget-desc" :title="item.desc">{{ item.desc }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 版本环境状态（仪表盘扁平网格） -->
    <div class="version-section">
      <div class="section-title-glass">
        <el-icon><Coin /></el-icon>
        <span>{{ t('about.techDashboard') }}</span>
      </div>
      <div class="env-dashboard-grid">
        <div class="glass-card env-stat-widget">
          <div class="version-icon blue">
            <el-icon><EditPen /></el-icon>
          </div>
          <div class="version-info">
            <div class="version-number">{{ appVersion }}</div>
            <div class="version-label">{{ t('zh-CN' === getLanguage() ? '软件版本' : 'App Version') }}</div>
          </div>
        </div>
        <div class="glass-card env-stat-widget">
          <div class="version-icon purple">
            <el-icon><Cpu /></el-icon>
          </div>
          <div class="version-info">
            <div class="version-number">{{ tauriVersion || '2.10.3' }}</div>
            <div class="version-label">Tauri Engine</div>
          </div>
        </div>
        <div class="glass-card env-stat-widget">
          <div class="version-icon green">
            <el-icon><Calendar /></el-icon>
          </div>
          <div class="version-info">
            <div class="version-number">{{ buildDate }}</div>
            <div class="version-label">Build Date</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 主要核心亮点功能 -->
    <div class="features-section">
      <div class="section-title-glass">
        <el-icon><Trophy /></el-icon>
        <span>{{ t('zh-CN' === getLanguage() ? '优化核心特性' : 'Key Advantages') }}</span>
      </div>
      <el-row :gutter="16">
        <el-col :span="8" v-for="(feature, index) in features" :key="feature.title">
          <div class="glass-card feature-card" :style="{ animationDelay: `${index * 0.05}s` }">
            <div class="feature-icon-wrapper" :style="{ background: `${feature.color}15` }">
              <el-icon :size="24" :style="{ color: feature.color }">
                <component :is="feature.icon" />
              </el-icon>
            </div>
            <div class="feature-content">
              <h4>{{ feature.title }}</h4>
              <p>{{ feature.description }}</p>
            </div>
          </div>
        </el-col>
      </el-row>
    </div>

    <!-- 技术栈 -->
    <div class="tech-section">
      <div class="section-title-glass">
        <el-icon><Files /></el-icon>
        <span>{{ t('about.techDashboard') }}</span>
      </div>
      <div class="tech-grid">
        <div
          v-for="(tech, index) in techStack"
          :key="tech.name"
          class="glass-card tech-item"
          :style="{ animationDelay: `${index * 0.05}s` }"
        >
          <div class="tech-icon">{{ tech.icon }}</div>
          <div class="tech-info">
            <div class="tech-name">{{ tech.name }}</div>
            <div class="tech-version">{{ tech.version }}</div>
          </div>
          <div class="tech-desc">{{ tech.description }}</div>
        </div>
      </div>
    </div>

    <!-- 版权说明 -->
    <div class="glass-card copyright-section-glass">
      <div class="copyright-content">
        <div class="copyright-main">
          <p class="copyright-text">{{ appInfo.copyright }}</p>
          <p class="license-text">License: {{ appInfo.license }}</p>
        </div>
        <div class="copyright-actions">
          <el-button
            text
            type="primary"
            @click="openExternalLink('https://github.com/daybydaymylove2009-max/SpaceTree')"
          >
            <template #icon><span>🐙</span></template>
            {{ t('about.website') }}
          </el-button>
          <el-button
            text
            type="primary"
            @click="openExternalLink('https://github.com/daybydaymylove2009-max/SpaceTree/issues')"
          >
            <template #icon><span>💬</span></template>
            {{ t('about.issue') }}
          </el-button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.about-center-glass {
  max-width: 1000px;
  margin: 0 auto;
  padding: 0 16px 40px;
}

.about-hero-glass {
  position: relative;
  padding: 50px 30px;
  margin-bottom: 30px;
  background: linear-gradient(135deg, rgba(64, 158, 255, 0.8) 0%, rgba(118, 75, 162, 0.8) 100%);
  overflow: hidden;
  border-radius: 0 0 24px 24px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
  backdrop-filter: blur(8px);
}

.hero-background {
  position: absolute;
  inset: 0;
  overflow: hidden;
  z-index: 0;
}

.gradient-orb {
  position: absolute;
  border-radius: 50%;
  filter: blur(60px);
  opacity: 0.55;
}

.orb-1 {
  width: 250px;
  height: 250px;
  background: #ff7675;
  top: -80px;
  right: -40px;
  animation: float 7s ease-in-out infinite;
}

.orb-2 {
  width: 200px;
  height: 200px;
  background: #74b9ff;
  bottom: -60px;
  left: -20px;
  animation: float 9s ease-in-out infinite reverse;
}

@keyframes float {
  0%, 100% { transform: translate(0, 0) scale(1); }
  50% { transform: translate(20px, -20px) scale(1.08); }
}

.hero-content {
  position: relative;
  z-index: 1;
  text-align: center;
  color: white;
}

.app-logo-container {
  position: relative;
  display: inline-block;
  margin-bottom: 16px;
}

.app-logo {
  width: 100px;
  height: 100px;
  background: rgba(255, 255, 255, 0.25);
  backdrop-filter: blur(12px);
  border-radius: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
  border: 1px solid rgba(255, 255, 255, 0.35);
}

.version-badge {
  position: absolute;
  bottom: -8px;
  left: 50%;
  transform: translateX(-50%);
}

.app-name {
  font-size: 30px;
  font-weight: 700;
  margin: 0 0 4px 0;
  text-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
}

.app-fullname {
  font-size: 14px;
  opacity: 0.9;
  margin: 0 0 12px 0;
  font-weight: 300;
  letter-spacing: 1.5px;
}

.app-description {
  font-size: 13.5px;
  opacity: 0.85;
  max-width: 550px;
  margin: 0 auto 24px;
  line-height: 1.6;
}

.hero-actions {
  display: flex;
  gap: 12px;
  justify-content: center;
}

.copy-btn {
  background: rgba(255, 255, 255, 0.95) !important;
  color: #764ba2 !important;
  border: none !important;
  font-weight: 600;
}

.copy-btn:hover {
  background: #ffffff !important;
  box-shadow: 0 4px 12px rgba(255,255,255,0.3);
}

.github-btn {
  background: rgba(255, 255, 255, 0.15) !important;
  border-color: rgba(255, 255, 255, 0.35) !important;
  color: white !important;
}

.github-btn:hover {
  background: rgba(255,255,255,0.25) !important;
}

/* 玻璃卡片 */
.glass-card {
  background: var(--el-bg-color-overlay);
  backdrop-filter: blur(16px) saturate(160%);
  -webkit-backdrop-filter: blur(16px) saturate(160%);
  border: 1px solid var(--el-border-color-light);
  border-radius: 14px;
  box-shadow: 0 8px 32px 0 rgba(31, 38, 135, 0.02);
  transition: all 0.3s ease;
}

.glass-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 10px 24px rgba(31, 38, 135, 0.05);
}

/* 极客技术仪表盘（核心亮点） */
.highlights-section {
  margin-bottom: 30px;
}

.highlights-dashboard-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
  width: 100%;
  box-sizing: border-box;
}

@media (max-width: 1080px) {
  .highlights-dashboard-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

.tech-stat-widget {
  padding: 14px 16px;
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 80px;
}

.widget-icon-wrapper {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.widget-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  min-width: 0;
  overflow: hidden;
  text-align: left;
}

.widget-value {
  font-size: 15px;
  font-weight: 850;
  letter-spacing: -0.5px;
  line-height: 1.25;
  margin-bottom: 1px;
  display: block;
  width: 100%;
  font-family: 'Segoe UI', system-ui, sans-serif;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.value-color-0 {
  background: linear-gradient(135deg, #409EFF 30%, #00d2ff 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}
.value-color-1 {
  background: linear-gradient(135deg, #67C23A 30%, #b8e994 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}
.value-color-2 {
  background: linear-gradient(135deg, #e6a23c 30%, #f1c40f 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}
.value-color-3 {
  background: linear-gradient(135deg, #f56c6c 30%, #ff7675 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.widget-label {
  font-size: 11.5px;
  font-weight: 700;
  color: var(--el-text-color-primary);
  line-height: 1.25;
  display: block;
  width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.widget-desc {
  font-size: 11px;
  color: var(--el-text-color-placeholder);
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 版本与系统 */
.version-section {
  margin-bottom: 30px;
}

.section-title-glass {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  margin-bottom: 16px;
}

.section-title-glass .el-icon {
  color: var(--el-color-primary);
}

.env-dashboard-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
}

.env-stat-widget {
  padding: 12px 14px;
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 68px;
  box-sizing: border-box;
  overflow: hidden;
}

.version-icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}

.version-icon.blue {
  background: rgba(64, 158, 255, 0.1);
  color: #409EFF;
}

.version-icon.purple {
  background: rgba(142, 68, 173, 0.1);
  color: #8e44ad;
}

.version-icon.green {
  background: rgba(103, 194, 58, 0.1);
  color: #67C23A;
}

.version-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  min-width: 0;
  overflow: hidden;
  text-align: left;
}

.version-number {
  font-size: 14.5px;
  font-weight: 800;
  color: var(--el-text-color-primary);
  line-height: 1.2;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.version-label {
  font-size: 11px;
  color: var(--el-text-color-placeholder);
  margin-top: 2px;
}

/* 主要功能 */
.features-section {
  margin-bottom: 30px;
}

.feature-card {
  padding: 16px;
  display: flex;
  align-items: flex-start;
  gap: 12px;
  margin-bottom: 16px;
}

.feature-icon-wrapper {
  width: 48px;
  height: 48px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.feature-content {
  text-align: left;
}

.feature-content h4 {
  margin: 0 0 4px 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.feature-content p {
  margin: 0;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  line-height: 1.5;
}

/* 技术栈 */
.tech-section {
  margin-bottom: 30px;
}

.tech-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
}

.tech-item {
  padding: 16px;
}

.tech-icon {
  font-size: 28px;
  margin-bottom: 8px;
}

.tech-info {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.tech-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.tech-version {
  font-size: 11px;
  color: var(--el-color-primary);
  background: var(--el-color-primary-light-9);
  padding: 1px 6px;
  border-radius: 8px;
}

.tech-desc {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  text-align: left;
}

/* 版权与链接 */
.copyright-section-glass {
  padding: 20px;
  text-align: center;
}

.copyright-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.copyright-main {
  text-align: center;
}

.copyright-text {
  font-size: 13px;
  color: var(--el-text-color-regular);
  margin: 0 0 2px 0;
}

.license-text {
  font-size: 12px;
  color: var(--el-text-color-placeholder);
  margin: 0;
}

.copyright-actions {
  display: flex;
  gap: 8px;
}

@media (max-width: 900px) {
  .env-dashboard-grid {
    grid-template-columns: repeat(2, 1fr);
  }
  .highlights-dashboard-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 600px) {
  .env-dashboard-grid {
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }
  .env-stat-widget {
    padding: 10px;
    gap: 8px;
    min-height: 60px;
  }
  .version-number {
    font-size: 13px;
  }
  .version-label {
    font-size: 10px;
  }
  .tech-stat-widget {
    padding: 10px;
    gap: 8px;
    min-height: 70px;
  }
  .widget-value {
    font-size: 15px;
  }
}

@media (max-width: 768px) {
  .about-hero-glass {
    padding: 30px 16px;
    border-radius: 0 0 16px 16px;
  }

  .app-name {
    font-size: 24px;
  }

  .hero-actions {
    flex-direction: column;
    align-items: center;
  }

  .tech-grid {
    grid-template-columns: repeat(2, 1fr);
  }

  .tech-stat-widget,
  .env-stat-widget,
  .feature-card {
    margin-bottom: 12px;
  }
}
</style>
