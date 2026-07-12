<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { 
  Folder, Search, Document, DataAnalysis, Tools, Setting, 
  Fold, Expand, CircleCheck, InfoFilled, Files, Picture, Download
} from '@element-plus/icons-vue';
import { t, getLanguage } from '../utils/i18n';

interface NavItem {
  key: string;
  icon: any;
  badge?: number;
}

const props = defineProps<{
  activeMenu: string;
}>()

const emit = defineEmits<{
  (e: 'menuChange', key: string): void
}>()

const isCollapsed = ref(false);
const isMobile = ref(window.innerWidth < 768);
const forceUpdateKey = ref(0);

const onLanguageChange = () => {
  forceUpdateKey.value++;
};

// 导航菜单配置
const navItems: NavItem[] = [
  { key: 'scan', icon: Folder },
  { key: 'search', icon: Search },
  { key: 'directory', icon: Files },
  { key: 'duplicates', icon: Document },
  { key: 'image_archive', icon: Picture },
  { key: 'analysis', icon: DataAnalysis },
  { key: 'tools', icon: Tools },
  { key: 'settings', icon: Setting },
  { key: 'update', icon: Download },
  { key: 'about', icon: InfoFilled },
];

// 动态解析国际化菜单标签
function getMenuLabel(key: string): string {
  switch (key) {
    case 'scan': return t('menu.scan');
    case 'search': return t('search.title');
    case 'directory': return t('zh-CN' === getLanguage() ? '目录浏览' : 'Directory View');
    case 'duplicates': return t('zh-CN' === getLanguage() ? '重复文件' : 'Duplicates List');
    case 'image_archive': return t('zh-CN' === getLanguage() ? '图片打包' : 'Image Packager');
    case 'analysis': return t('analysis.title');
    case 'tools': return t('menu.tools');
    case 'settings': return t('menu.settings');
    case 'update': return t('zh-CN' === getLanguage() ? '检查更新' : 'Check Updates');
    case 'about': return t('menu.about');
    default: return '';
  }
}

// 处理菜单点击
function handleMenuClick(key: string) {
  emit('menuChange', key);
  if (isMobile.value) {
    isCollapsed.value = true;
  }
}

// 切换侧边栏
function toggleSidebar() {
  isCollapsed.value = !isCollapsed.value;
}

const handleResize = () => {
  isMobile.value = window.innerWidth < 768;
  if (isMobile.value) {
    isCollapsed.value = true;
  } else {
    isCollapsed.value = false;
  }
};

onMounted(() => {
  window.addEventListener('resize', handleResize);
  window.addEventListener('app-lang-change', onLanguageChange);
  if (isMobile.value) {
    isCollapsed.value = true;
  }
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  window.removeEventListener('app-lang-change', onLanguageChange);
});
</script>

<template>
  <div class="app-layout" :class="{ 'is-collapsed': isCollapsed, 'is-mobile': isMobile }" :key="forceUpdateKey">
    <!-- 侧边栏 -->
    <aside class="sidebar">
      <!-- Logo区域 -->
      <div class="sidebar-header">
        <div class="brand">
          <div class="brand-logo">
            <el-icon :size="28" color="#409EFF"><CircleCheck /></el-icon>
          </div>
          <div class="brand-text" v-show="!isCollapsed">
            <h1 class="brand-title">{{ t('common.appName') }}</h1>
            <span class="brand-subtitle">v3.50.0</span>
          </div>
        </div>
        <el-button 
          class="collapse-btn"
          text
          circle
          @click="toggleSidebar"
          v-if="!isMobile"
        >
          <el-icon :size="18">
            <Fold v-if="!isCollapsed" />
            <Expand v-else />
          </el-icon>
        </el-button>
      </div>

      <!-- 导航菜单 -->
      <nav class="sidebar-nav">
        <div 
          v-for="item in navItems" 
          :key="item.key"
          class="nav-item"
          :class="{ 'is-active': activeMenu === item.key }"
          @click="handleMenuClick(item.key)"
        >
          <el-icon :size="20" class="nav-icon">
            <component :is="item.icon" />
          </el-icon>
          <span class="nav-label" v-show="!isCollapsed">{{ getMenuLabel(item.key) }}</span>
          <el-badge 
            v-if="item.badge && !isCollapsed" 
            :value="item.badge" 
            class="nav-badge"
          />
        </div>
      </nav>

      <!-- 底部信息 -->
      <div class="sidebar-footer" v-show="!isCollapsed">
        <div class="footer-info">
          <el-tag size="small" type="info" effect="plain">Pro</el-tag>
          <span class="footer-text">{{ t('zh-CN' === getLanguage() ? '已激活' : 'Activated') }}</span>
        </div>
      </div>
    </aside>

    <!-- 遮罩层(移动端) -->
    <div 
      class="sidebar-overlay" 
      v-if="isMobile && !isCollapsed"
      @click="isCollapsed = true"
    ></div>

    <!-- 主内容区 -->
    <main class="main-content">
      <slot />
    </main>
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background: radial-gradient(circle at 10% 20%, var(--el-bg-color-page) 0%, var(--el-bg-color) 90%);
  position: relative;
}

.app-layout::before {
  content: '';
  position: absolute;
  width: 400px;
  height: 400px;
  background: radial-gradient(circle, rgba(64, 158, 255, 0.08) 0%, transparent 70%);
  top: -100px;
  left: -100px;
  pointer-events: none;
  z-index: 1;
}

.app-layout::after {
  content: '';
  position: absolute;
  width: 500px;
  height: 500px;
  background: radial-gradient(circle, rgba(103, 194, 58, 0.05) 0%, transparent 70%);
  bottom: -150px;
  right: -150px;
  pointer-events: none;
  z-index: 1;
}

/* 侧边栏 */
.sidebar {
  width: 240px;
  height: 100%;
  background: var(--el-bg-color-overlay);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-right: 1px solid var(--el-border-color-light);
  display: flex;
  flex-direction: column;
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  z-index: 100;
}

.is-collapsed .sidebar {
  width: 64px;
}

.is-mobile .sidebar {
  position: fixed;
  left: 0;
  top: 0;
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.15);
}

.is-mobile.is-collapsed .sidebar {
  transform: translateX(-100%);
}

/* 侧边栏头部 */
.sidebar-header {
  padding: 20px 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--el-border-color-light);
}

.brand {
  display: flex;
  align-items: center;
  gap: 12px;
  overflow: hidden;
}

.brand-logo {
  flex-shrink: 0;
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--el-color-primary-light-9) 0%, var(--el-color-primary-light-7) 100%);
  border-radius: 10px;
}

.brand-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  white-space: nowrap;
  text-align: left;
}

.brand-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  margin: 0;
}

.brand-subtitle {
  font-size: 12px;
  color: var(--el-text-color-placeholder);
}

.collapse-btn {
  flex-shrink: 0;
  color: var(--el-text-color-placeholder);
}

.collapse-btn:hover {
  color: var(--el-color-primary);
  background: var(--el-fill-color-light);
}

/* 导航菜单 */
.sidebar-nav {
  flex: 1;
  padding: 12px 8px;
  overflow-y: auto;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  margin-bottom: 4px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
  color: var(--el-text-color-regular);
  position: relative;
}

.nav-item:hover {
  background: var(--el-fill-color-light);
  color: var(--el-color-primary);
}

.nav-item.is-active {
  background: linear-gradient(135deg, var(--el-color-primary) 0%, var(--el-color-primary-light-3) 100%);
  color: #ffffff;
  box-shadow: 0 4px 16px rgba(64, 158, 255, 0.2);
}

.nav-icon {
  flex-shrink: 0;
}

.nav-label {
  flex: 1;
  font-size: 14px;
  font-weight: 500;
  white-space: nowrap;
}

.nav-badge {
  flex-shrink: 0;
}

/* 侧边栏底部 */
.sidebar-footer {
  padding: 16px;
  border-top: 1px solid var(--el-border-color-light);
}

.footer-info {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--el-fill-color-light);
  border-radius: 6px;
}

.footer-text {
  font-size: 12px;
  color: var(--el-color-success);
  font-weight: 500;
}

/* 遮罩层 */
.sidebar-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: 99;
}

/* 主内容区 */
.main-content {
  flex: 1;
  min-width: 0;
  width: 100%;
  height: 100%;
  overflow: auto;
  padding: 24px;
  transition: margin-left 0.3s;
  box-sizing: border-box;
  z-index: 2;
}

.main-content > * {
  width: 100% !important;
  max-width: 100% !important;
  min-width: 100% !important;
}

.is-mobile .main-content {
  margin-left: 0;
}

.sidebar-nav::-webkit-scrollbar {
  width: 4px;
}

.sidebar-nav::-webkit-scrollbar-track {
  background: transparent;
}

.sidebar-nav::-webkit-scrollbar-thumb {
  background: var(--el-border-color);
  border-radius: 2px;
}

.sidebar-nav::-webkit-scrollbar-thumb:hover {
  background: var(--el-border-color-hover);
}
</style>
