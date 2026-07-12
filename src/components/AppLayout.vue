<script setup lang="ts">
import { ref } from 'vue';
import { 
  Folder, Search, Document, DataAnalysis, Tools, Setting, 
  Fold, Expand, CircleCheck, InfoFilled, Files, Picture, Download
} from '@element-plus/icons-vue';

interface NavItem {
  key: string;
  label: string;
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

// 导航菜单配置
const navItems: NavItem[] = [
  { key: 'scan', label: '扫描中心', icon: Folder },
  { key: 'search', label: '文件搜索', icon: Search },
  { key: 'directory', label: '目录浏览', icon: Files },
  { key: 'duplicates', label: '重复文件', icon: Document },
  { key: 'image_archive', label: '图片打包', icon: Picture },
  { key: 'analysis', label: '分析中心', icon: DataAnalysis },
  { key: 'tools', label: '系统工具', icon: Tools },
  { key: 'settings', label: '全局设置', icon: Setting },
  { key: 'update', label: '检查更新', icon: Download },
  { key: 'about', label: '关于软件', icon: InfoFilled },
];

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

// 监听窗口大小变化
window.addEventListener('resize', () => {
  isMobile.value = window.innerWidth < 768;
  if (isMobile.value) {
    isCollapsed.value = true;
  } else {
    isCollapsed.value = false;
  }
});

// 初始化
if (isMobile.value) {
  isCollapsed.value = true;
}
</script>

<template>
  <div class="app-layout" :class="{ 'is-collapsed': isCollapsed, 'is-mobile': isMobile }">
    <!-- 侧边栏 -->
    <aside class="sidebar">
      <!-- Logo区域 -->
      <div class="sidebar-header">
        <div class="brand">
          <div class="brand-logo">
            <el-icon :size="28" color="#409EFF"><CircleCheck /></el-icon>
          </div>
          <div class="brand-text" v-show="!isCollapsed">
            <h1 class="brand-title">重复文件猎手</h1>
            <span class="brand-subtitle">v3.0.0</span>
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
          <span class="nav-label" v-show="!isCollapsed">{{ item.label }}</span>
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
          <span class="footer-text">已激活</span>
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
  background: radial-gradient(circle at 10% 20%, #f4f6fa 0%, #eef2f7 90%);
  position: relative;
}

.app-layout::before {
  content: '';
  position: absolute;
  width: 400px;
  height: 400px;
  background: radial-gradient(circle, rgba(64, 158, 255, 0.12) 0%, transparent 70%);
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
  background: radial-gradient(circle, rgba(103, 194, 58, 0.08) 0%, transparent 70%);
  bottom: -150px;
  right: -150px;
  pointer-events: none;
  z-index: 1;
}

/* 侧边栏 */
.sidebar {
  width: 240px;
  height: 100%;
  background: rgba(255, 255, 255, 0.55);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-right: 1px solid rgba(255, 255, 255, 0.45);
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
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.1);
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
  border-bottom: 1px solid #e4e7ed;
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
  background: linear-gradient(135deg, #e3f2fd 0%, #bbdefb 100%);
  border-radius: 10px;
}

.brand-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  white-space: nowrap;
}

.brand-title {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}

.brand-subtitle {
  font-size: 12px;
  color: #909399;
}

.collapse-btn {
  flex-shrink: 0;
  color: #909399;
}

.collapse-btn:hover {
  color: #409EFF;
  background: #ecf5ff;
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
  color: #606266;
  position: relative;
}

.nav-item:hover {
  background: rgba(64, 158, 255, 0.1);
  color: #409EFF;
}

.nav-item.is-active {
  background: linear-gradient(135deg, #409EFF 0%, #66b1ff 100%);
  color: #ffffff;
  box-shadow: 0 4px 16px rgba(64, 158, 255, 0.25);
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
  border-top: 1px solid #e4e7ed;
}

.footer-info {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #f5f7fa;
  border-radius: 6px;
}

.footer-text {
  font-size: 12px;
  color: #67C23A;
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

/* 确保 slot 内容填满宽度 */
.main-content > * {
  width: 100% !important;
  max-width: 100% !important;
  min-width: 100% !important;
}

.is-mobile .main-content {
  margin-left: 0;
}

/* 滚动条样式 */
.sidebar-nav::-webkit-scrollbar {
  width: 4px;
}

.sidebar-nav::-webkit-scrollbar-track {
  background: transparent;
}

.sidebar-nav::-webkit-scrollbar-thumb {
  background: #dcdfe6;
  border-radius: 2px;
}

.sidebar-nav::-webkit-scrollbar-thumb:hover {
  background: #c0c4cc;
}
</style>
