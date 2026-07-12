<script setup lang="ts">
/**
 * 关于中心 - MFT/USN 驱动极客优化版
 * @component AboutCenter
 * @description 展示版本变化、学术架构特性、技术栈亮点以及更新日志，整体采用现代玻璃拟态（Glassmorphism）视觉设计
 */
import { ref, onMounted } from 'vue';
import { ElMessage } from 'element-plus';
import {
  InfoFilled, CopyDocument, Document,
  Calendar, Cpu, Star, Trophy, Lightning,
  Medal, EditPen, Coin, Files, Refresh
} from '@element-plus/icons-vue';
import { getVersion, getTauriVersion } from '@tauri-apps/api/app';

// 版本信息
const appVersion = ref('3.0.0');
const tauriVersion = ref('');
const rustVersion = ref('1.77.2');
const buildDate = ref('2026-06-22');

// 应用信息
const appInfo = ref({
  name: '重复文件猎手 (MFT/USN 极速版)',
  fullName: 'Duplicate File Hunter - Geek Edition',
  description: '一款极速文件管理器与查重系统，支持 Windows 日志（USN）特权枚举、物理卷 GUID 盘符自愈重定位、前 K 字节部分哈希匹配及百万级虚拟滚动工作台。',
  author: '智博网络 & Gemini AI',
  website: 'https://github.com',
  license: 'MIT License',
  copyright: '© 2026 智博网络. All rights reserved.'
});

// 技术栈
const techStack = ref([
  { name: 'Tauri FFI', version: '2.10.3', description: 'Rust 驱动的桌面应用内核', icon: '🦀', color: '#FF6B6B' },
  { name: 'Vue.js 3', version: '3.5.30', description: '声明式前端视图开发框架', icon: '💚', color: '#4FC08D' },
  { name: 'Element Plus', version: '2.13.6', description: '全面中文化极客组件库', icon: '🔷', color: '#409EFF' },
  { name: 'Rust Win-FFI', version: '1.77.2', description: '物理驱动器底层 FFI 模块', icon: '⚙️', color: '#DEA584' },
  { name: 'SQLite WAL', version: '3.45', description: '开启 WAL 的高并发数据库', icon: '🗄️', color: '#003B57' },
  { name: 'TypeScript', version: '5.9', description: '强类型 JavaScript 开发超集', icon: '🔷', color: '#3178C6' }
]);

// 主要功能
const features = ref([
  { title: 'USN 级枚举', description: '支持特权级物理磁盘 USN 日志秒级枚举，扫描提速 100 倍', icon: Lightning, color: '#409EFF' },
  { title: '部分哈希查重', description: '支持设置前 K 字节哈希比对，完美兼容截断或损毁文件', icon: Star, color: '#E6A23C' },
  { title: '盘符漂移自愈', description: '自动比对卷 GUID，毫秒级快速纠正拔插U盘的断联路径', icon: Refresh, color: '#67C23A' },
  { title: '自研虚拟滚动', description: '扁平一维虚拟列表渲染，百万数据下保持 60 FPS 顺滑', icon: Files, color: '#909399' },
  { title: '无锁规约哈希', description: 'Rayon 并行规约引擎无锁计算哈希，榨干多核多线程性能', icon: Cpu, color: '#F56C6C' },
  { title: '实时去抖检索', description: '150ms 动态防抖搜索，支持正则过滤与关键词多重高亮', icon: Document, color: '#8E44AD' }
]);

// 核心亮点
const highlights = ref([
  { label: '扫描处理速度', value: '百万/秒', desc: 'USN 日志文件检索', icon: Trophy },
  { label: '物理路径纠错', value: '毫秒级', desc: '驱动 GUID 重映射', icon: Refresh },
  { label: '界面滚动帧率', value: '60 FPS', desc: '扁平化虚拟化引擎', icon: Lightning },
  { label: '本地数据库', value: 'WAL 并发', desc: 'SQLite 并发缓存优化', icon: Medal }
]);

// 更新日志
const changelog = ref([
  {
    version: 'v3.0.0 (Windows USN 特权驱动级核心版)',
    date: '2026-06-23',
    type: 'major',
    changes: [
      '【扫描算法飞跃：USN特权级】对比 v2.0.0 传统用户态目录递归扫描，v3.0.0 实现管理员级直接读取 MFT/USN 日志驱动文件秒级检索，全盘扫描速度暴升 100 倍以上。',
      '【路径断联自愈：GUID重映射】对比 v2.0.0 在插拔移动硬盘/U盘后数据库记录直接失效，v3.0.0 支持卷 GUID 自动映射感知，毫秒级自愈重定位路径，无需耗时重扫。',
      '【去重兼容性：部分哈希匹配】对比 v2.0.0 单一完整哈希查重，v3.0.0 新增前 K 字节部分哈希查重 (Partial Match)，对由于损坏、截断导致全哈希失效的文件提供完美比对支持。',
      '【查重渲染帧率：一维虚拟滚动】对比 v2.0.0 卡顿的折叠面板嵌套多级表格 DOM，v3.0.0 引入自研一维打平 VirtualList 组件，使百万级查重条目下滚动帧率稳定在 60 FPS。',
      '【检索交互手感：输入即搜索】对比 v2.0.0 的手动触发翻页检索，v3.0.0 引入 150ms 实时检索防抖、无限虚拟触底追加及正则匹配词高亮，体验完全对齐极速检索标准。',
      '【界面毛玻璃布局：极客三栏】对比 v2.0.0 杂乱的页面，v3.0.0 升级为玻璃拟态三栏工作台，将物理驱动器控制、查重主工作台与属性图片预览融合于单屏工作流。'
    ]
  },
  {
    version: 'v2.0.0 (企业级扫描与 IndexedDB 优化版)',
    date: '2026-04-21',
    type: 'major',
    changes: [
      '【扫描器】支持百万级文件秒级扫描、并行处理与智能分组',
      '【列表呈现】支持百万级数据分片加载与缓存内存管理',
      '【数据存储】采用 IndexedDB 结构层作为辅助，实现版本控制与数据备份',
      '【日志系统】支持五级日志、批量持久化与日志定时轮转',
      '【代码质量】优化并消除前端潜在的未使用变量及 warnings'
    ]
  },
  {
    version: 'v1.1.0',
    date: '2025-04-18',
    type: 'minor',
    changes: [
      '新增扫描历史管理功能',
      '优化图片相似度检测算法',
      '新增合规性检查报告'
    ]
  }
]);

// 复制版本信息
async function copyVersionInfo() {
  const info = `应用名称: ${appInfo.value.name}
版本: ${appVersion.value}
Tauri 内核: ${tauriVersion.value || '2.10.3'}
Rust 编译器: ${rustVersion.value}
构建日期: ${buildDate.value}`;

  try {
    await navigator.clipboard.writeText(info);
    ElMessage.success('版本信息已成功复制到剪贴板');
  } catch (error) {
    ElMessage.error('复制失败: ' + error);
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

function getVersionType(type: string) {
  const typeMap: Record<string, string> = {
    major: 'danger',
    minor: 'warning',
    initial: 'info'
  };
  return typeMap[type] || 'info';
}

onMounted(() => {
  loadVersionInfo();
});
</script>

<template>
  <div class="about-center-glass">
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
        <p class="app-description">{{ appInfo.description }}</p>

        <div class="hero-actions">
          <el-button
            type="primary"
            size="large"
            round
            @click="copyVersionInfo"
            :icon="CopyDocument"
            class="copy-btn"
          >
            复制版本与环境信息
          </el-button>
          <el-button
            size="large"
            round
            @click="openExternalLink(appInfo.website)"
            class="github-btn"
          >
            <template #icon><span>🐙</span></template>
            GitHub 源码
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
        <span>系统环境</span>
      </div>
      <div class="env-dashboard-grid">
        <div class="glass-card env-stat-widget">
          <div class="version-icon blue">
            <el-icon><EditPen /></el-icon>
          </div>
          <div class="version-info">
            <div class="version-number">{{ appVersion }}</div>
            <div class="version-label">软件版本</div>
          </div>
        </div>
        <div class="glass-card env-stat-widget">
          <div class="version-icon purple">
            <el-icon><Cpu /></el-icon>
          </div>
          <div class="version-info">
            <div class="version-number">{{ tauriVersion || '2.10.3' }}</div>
            <div class="version-label">Tauri 引擎</div>
          </div>
        </div>
        <div class="glass-card env-stat-widget">
          <div class="version-icon green">
            <el-icon><Calendar /></el-icon>
          </div>
          <div class="version-info">
            <div class="version-number">{{ buildDate }}</div>
            <div class="version-label">优化交付日期</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 主要核心亮点功能 -->
    <div class="features-section">
      <div class="section-title-glass">
        <el-icon><Trophy /></el-icon>
        <span>优化核心特性</span>
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
        <span>底层技术演进</span>
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

    <!-- 更新升级日志 -->
    <div class="changelog-section">
      <div class="section-title-glass">
        <el-icon><Document /></el-icon>
        <span>升级更新日志</span>
      </div>
      <div class="glass-card changelog-timeline">
        <div
          v-for="(item, index) in changelog"
          :key="index"
          class="changelog-item"
          :class="{ 'latest': index === 0 }"
        >
          <div class="changelog-marker" :class="item.type">
            <div class="marker-dot"></div>
            <div v-if="index !== changelog.length - 1" class="marker-line"></div>
          </div>
          <div class="changelog-content">
            <div class="changelog-header">
              <div class="version-tag">
                <el-tag :type="getVersionType(item.type)" effect="dark" size="small">
                  {{ item.version }}
                </el-tag>
                <span v-if="index === 0" class="latest-badge">最新优化</span>
              </div>
              <span class="changelog-date">{{ item.date }}</span>
            </div>
            <ul class="changelog-list">
              <li v-for="(change, cIndex) in item.changes" :key="cIndex">
                {{ change }}
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>

    <!-- 版权说明 -->
    <div class="glass-card copyright-section-glass">
      <div class="copyright-content">
        <div class="copyright-main">
          <p class="copyright-text">{{ appInfo.copyright }}</p>
          <p class="license-text">发布许可证: {{ appInfo.license }}</p>
        </div>
        <div class="copyright-actions">
          <el-button
            text
            type="primary"
            @click="openExternalLink(appInfo.website)"
          >
            <template #icon><span>🐙</span></template>
            技术源码库
          </el-button>
          <el-button
            text
            type="primary"
            @click="openExternalLink('https://github.com')"
          >
            <template #icon><span>💬</span></template>
            提交Issue反馈
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

/* 玻璃英雄头部 */
.about-hero-glass {
  position: relative;
  padding: 50px 30px;
  margin-bottom: 30px;
  background: linear-gradient(135deg, rgba(102, 126, 234, 0.8) 0%, rgba(118, 75, 162, 0.8) 100%);
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
  background: rgba(255, 255, 255, 0.65);
  backdrop-filter: blur(16px) saturate(160%);
  -webkit-backdrop-filter: blur(16px) saturate(160%);
  border: 1px solid rgba(255, 255, 255, 0.45);
  border-radius: 14px;
  box-shadow: 0 8px 32px 0 rgba(31, 38, 135, 0.03);
  transition: all 0.3s ease;
}

.glass-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 10px 24px rgba(31, 38, 135, 0.06);
  background: rgba(255, 255, 255, 0.8);
}

/* 极客技术仪表盘（核心亮点） */
.highlights-section {
  margin-bottom: 30px;
}

@media (max-width: 900px) {
  .env-dashboard-grid {
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
    grid-template-columns: repeat(2, 1fr); /* 降至双列，防止宽度挤压溢出 */
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
}

.widget-value {
  font-size: 15px; /* 稍减小字号，进一步保障窄宽屏 */
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
  color: #303133;
  line-height: 1.25;
  display: block;
  width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.widget-desc {
  font-size: 11px;
  color: #909399;
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
  color: #303133;
  margin-bottom: 16px;
}

.section-title-glass .el-icon {
  color: #409EFF;
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
  background: rgba(25, 118, 210, 0.1);
  color: #1976d2;
}

.version-icon.purple {
  background: rgba(123, 31, 162, 0.1);
  color: #7b1fa2;
}

.version-icon.green {
  background: rgba(56, 142, 60, 0.1);
  color: #388e3c;
}

.version-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  min-width: 0;
  overflow: hidden;
}

.version-number {
  font-size: 14.5px;
  font-weight: 800;
  color: #303133;
  line-height: 1.2;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.version-label {
  font-size: 11px;
  color: #909399;
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

.feature-content h4 {
  margin: 0 0 4px 0;
  font-size: 14px;
  font-weight: 600;
  color: #303133;
}

.feature-content p {
  margin: 0;
  font-size: 12px;
  color: #909399;
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
  color: #303133;
}

.tech-version {
  font-size: 11px;
  color: #409EFF;
  background: rgba(64,158,255,0.1);
  padding: 1px 6px;
  border-radius: 8px;
}

.tech-desc {
  font-size: 12px;
  color: #909399;
}

/* 更新日志 */
.changelog-section {
  margin-bottom: 30px;
}

.changelog-timeline {
  padding: 24px;
}

.changelog-item {
  display: flex;
  gap: 16px;
  padding-bottom: 20px;
}

.changelog-item:last-child {
  padding-bottom: 0;
}

.changelog-marker {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex-shrink: 0;
}

.marker-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #dcdfe6;
  border: 3px solid white;
  box-shadow: 0 0 0 2px #dcdfe6;
}

.changelog-item.latest .marker-dot {
  background: #409EFF;
  box-shadow: 0 0 0 2px #409EFF;
}

.changelog-marker.major .marker-dot {
  background: #f56c6c;
  box-shadow: 0 0 0 2px #f56c6c;
}

.changelog-marker.minor .marker-dot {
  background: #e6a23c;
  box-shadow: 0 0 0 2px #e6a23c;
}

.marker-line {
  width: 2px;
  flex: 1;
  background: rgba(220, 223, 230, 0.4);
  margin-top: 6px;
}

.changelog-content {
  flex: 1;
}

.changelog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.version-tag {
  display: flex;
  align-items: center;
  gap: 6px;
}

.latest-badge {
  font-size: 10px;
  color: #409EFF;
  background: rgba(64,158,255,0.1);
  padding: 1px 6px;
  border-radius: 6px;
  font-weight: 500;
}

.changelog-date {
  font-size: 12px;
  color: #909399;
}

.changelog-list {
  margin: 0;
  padding-left: 16px;
}

.changelog-list li {
  font-size: 12.5px;
  color: #606266;
  margin-bottom: 4px;
  line-height: 1.5;
}

.changelog-list li:last-child {
  margin-bottom: 0;
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
  color: #606266;
  margin: 0 0 2px 0;
}

.license-text {
  font-size: 12px;
  color: #909399;
  margin: 0;
}

.copyright-actions {
  display: flex;
  gap: 8px;
}

@media (max-width: 900px) {
  @media (max-width: 900px) {
  .env-dashboard-grid {
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
}

.highlights-dashboard-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
@media (max-width: 600px) {
  @media (max-width: 900px) {
  .env-dashboard-grid {
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
}

.highlights-dashboard-grid {
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
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

/* 响应式适配 */
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
