<script setup lang="ts">
/**
 * 更新中心组件
 * @component UpdateCenter
 * @description 检查更新、下载更新、版本信息管理
 */

import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Download, Refresh, Check, InfoFilled, Link } from '@element-plus/icons-vue'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

// 当前版本
const CURRENT_VERSION = '3.40.0'

// 更新状态
const updateStatus = ref<'checking' | 'available' | 'latest' | 'error' | 'downloading'>('checking')
const latestVersion = ref('')
const releaseNotes = ref('')
const downloadProgress = ref(0)
const lastCheckTime = ref('')
let activeUpdateInstance: any = null

// 版本历史
const versionHistory = ref([
  {
    version: '3.40.0',
    date: '2026-07-12',
    type: 'minor',
    changes: [
      '物理层去重支持：引入 NTFS 事务型“硬链接（Hardlink）替换去重”机制，避免物理删除对其他依赖程序的损坏',
      '物理卷ID校验：在建立硬链接前执行跨盘卷校验（Windows 卷序列号/Unix 设备ID），安全隔离跨盘非法操作',
      '空间可视化树图：在分析中心集成 CSS 面积占比自适应“空间矩形树图（Treemap）”卡片，支持大文件双击下钻定位',
      '快照差分比对工作台：支持历史扫描快照的 JSON 导出与上传载入，在前端以全文件 Diff 自动计算新旧数据的空间变动趋势'
    ]
  },
  {
    version: '3.30.0',
    date: '2026-07-01',
    type: 'minor',
    changes: [
      '文件检索中心（SearchCenter）学术级完全重写，采用与扫描中心一致的毛玻璃双卡片网格布局',
      '检索组件集成 Levenshtein（莱文斯坦）距离模糊比对打分算法，提供百分比相似度动态 Tag 标签',
      '检索组件支持标准正则表达式（带实时语法错误校验红字提醒）与通配符模式匹配',
      '检索中心增设时延（ms）、吞吐率（文件/秒）与匹配精度分数的学术级实时指标分析板',
      '引入“物理删除前全文件哈希安全审计（Byte-by-Byte Audit）”防误删防灾机制，并在删除时默认启用',
      '修复“大重复文件 TOP10”分析卡片中因 100MB 阈值限制硬编码引发一直显示 No Data 的 Bug'
    ]
  },
  {
    version: '3.20.1',
    date: '2026-06-29',
    type: 'patch',
    changes: [
      '修复“大重复文件 TOP10”因 100MB 硬编码大小过滤阈值导致一直空显 No Data 的算法漏洞',
      '规范了分析数据管道中最大重复项的文件路径排序逻辑'
    ]
  },
  {
    version: '3.20.0',
    date: '2026-06-29',
    type: 'minor',
    changes: [
      '引入“全文件哈希安全审计（Byte-by-Byte Audit）”防灾底线，消除潜在哈希碰撞误删隐患',
      '重复文件页面升级，支持“专业工作台”与“极简向导”双模式自由热切换',
      '文件检索页面独立卡片四边框及阴影重构，与扫描中心风格完全对齐',
      '锁定本地 `@tauri-apps/cli` 构建工具链，彻底修复打包元数据缺失造成的自动更新失效警告',
      '优化 Rust 编译器 Release 构建指令，解决句柄占用导致的内存越界崩溃'
    ]
  },
  {
    version: '3.10.0',
    date: '2026-06-29',
    type: 'minor',
    changes: [
      '全局设计系统级统一重构，彻底解决亮暗色主题冲突',
      '全功能页面顶层包裹容器边距与高宽统一，消除页面切换跳跃',
      '规范化全局毛玻璃卡片（Glassmorphism）、页面标题与交互按钮',
      '软件更新中心卡片玻璃化重构，补充深色模式日志框与边线色值',
      '文件检索页面右侧侧栏属性面板及高级筛选表单深色主题重构'
    ]
  },
  {
    version: '3.0.0',
    date: '2026-06-23',
    type: 'major',
    changes: [
      '物理驱动级 Windows MFT/USN 秒级极速全盘扫描',
      '物理磁盘插拔 GUID 毫秒级热自愈重定位',
      '前 K 字节部分哈希查重 (Partial Match) 算法',
      '百万级文件自研一维打平虚拟滚动列表（支持 60 FPS）',
      '驱动级一键快速文件媒体类型过滤栏'
    ]
  },
  {
    version: '2.0.0',
    date: '2024-01-15',
    type: 'major',
    changes: [
      '全新界面设计，支持暗黑模式',
      '智能重复文件清理助手',
      '文件预览功能（图片、视频、文档）',
      '批量操作增强',
      '空间使用趋势分析',
      '目录健康度评分',
      '操作历史与撤销功能',
      '文件粉碎安全删除'
    ]
  },
  {
    version: '1.1.0',
    date: '2023-12-01',
    type: 'minor',
    changes: [
      '优化扫描性能',
      '修复已知问题',
      '改进用户体验'
    ]
  }
])

// 检查更新
async function checkForUpdates() {
  updateStatus.value = 'checking'
  lastCheckTime.value = new Date().toLocaleString('zh-CN')
  
  try {
    const update = await check()
    if (update) {
      activeUpdateInstance = update
      updateStatus.value = 'available'
      latestVersion.value = update.version
      releaseNotes.value = update.body || '性能优化与体验改进'
    } else {
      updateStatus.value = 'latest'
    }
  } catch (error) {
    console.error('检查更新出错:', error)
    updateStatus.value = 'error'
    ElMessage.error('检查更新失败: ' + error)
  }
}

// 下载更新
async function downloadUpdate() {
  if (!activeUpdateInstance) {
    ElMessage.warning('没有可下载的更新实例')
    return
  }

  updateStatus.value = 'downloading'
  downloadProgress.value = 0
  
  try {
    let downloaded = 0
    let contentLength = 0
    
    await activeUpdateInstance.downloadAndInstall((event: any) => {
      switch (event.event) {
        case 'Started':
          contentLength = event.data.contentLength || 0
          break;
        case 'Progress':
          downloaded += event.data.chunkLength
          if (contentLength > 0) {
            downloadProgress.value = Math.round((downloaded / contentLength) * 100)
          }
          break;
        case 'Finished':
          downloadProgress.value = 100
          break;
      }
    })
    
    await ElMessageBox.alert('更新下载并安装完成，请点击立即重启以应用新版本！', '更新就绪', {
      confirmButtonText: '立即重启',
      type: 'success'
    })
    
    await relaunch()
  } catch (error) {
    console.error('下载安装更新出错:', error)
    updateStatus.value = 'error'
    ElMessage.error('更新下载失败: ' + error)
  }
}

// 打开官网
function openWebsite() {
  // 实际应调用后端打开浏览器
  // invoke('open_url', { url: 'https://your-website.com' })
  ElMessage.info('官网链接功能预留')
}

// 打开反馈页面
function openFeedback() {
  ElMessage.info('反馈功能预留')
}

onMounted(() => {
  checkForUpdates()
})
</script>

<template>
  <div class="update-center">
    <!-- 页面标题 -->
    <div class="page-header">
      <h2>软件更新</h2>
      <p class="header-subtitle">检查更新、查看版本历史</p>
    </div>

    <!-- 当前版本卡片 -->
    <el-card class="glass-card version-card" shadow="hover">
      <div class="current-version">
        <div class="version-info">
          <div class="version-number">{{ CURRENT_VERSION }}</div>
          <div class="version-label">当前版本</div>
        </div>
        <div class="version-status">
          <el-tag v-if="updateStatus === 'checking'" type="info">
            <el-icon class="is-loading"><Refresh /></el-icon>
            检查中...
          </el-tag>
          <el-tag v-else-if="updateStatus === 'latest'" type="success">
            <el-icon><Check /></el-icon>
            已是最新版本
          </el-tag>
          <el-tag v-else-if="updateStatus === 'available'" type="warning">
            <el-icon><InfoFilled /></el-icon>
            有新版本可用
          </el-tag>
          <el-tag v-else-if="updateStatus === 'error'" type="danger">
            检查失败
          </el-tag>
        </div>
      </div>

      <!-- 更新操作 -->
      <div class="update-actions" v-if="updateStatus === 'available'">
        <div class="update-info">
          <p>最新版本: <strong>{{ latestVersion }}</strong></p>
          <div class="release-notes">
            <h4>更新内容:</h4>
            <p>{{ releaseNotes }}</p>
          </div>
        </div>
        <el-button type="primary" size="large" @click="downloadUpdate">
          <el-icon><Download /></el-icon>
          立即更新
        </el-button>
      </div>

      <!-- 下载进度 -->
      <div v-if="updateStatus === 'downloading'" class="download-progress">
        <el-progress :percentage="downloadProgress" :stroke-width="20" status="success" />
        <p>正在下载更新...</p>
      </div>

      <!-- 检查按钮 -->
      <div class="check-actions">
        <el-button @click="checkForUpdates" :loading="updateStatus === 'checking'">
          <el-icon><Refresh /></el-icon>
          重新检查
        </el-button>
        <span v-if="lastCheckTime" class="last-check-time">
          上次检查: {{ lastCheckTime }}
        </span>
      </div>
    </el-card>

    <!-- 版本历史 -->
    <el-card class="glass-card history-card" shadow="never">
      <template #header>
        <div class="card-header">
          <span>版本历史</span>
        </div>
      </template>

      <el-timeline>
        <el-timeline-item
          v-for="(version, index) in versionHistory"
          :key="version.version"
          :type="version.type === 'major' ? 'primary' : 'info'"
          :timestamp="version.date"
        >
          <div class="version-item">
            <h4>
              v{{ version.version }}
              <el-tag v-if="index === 0" size="small" type="success">当前</el-tag>
              <el-tag v-if="version.type === 'major'" size="small" type="danger">重大更新</el-tag>
            </h4>
            <ul>
              <li v-for="change in version.changes" :key="change">{{ change }}</li>
            </ul>
          </div>
        </el-timeline-item>
      </el-timeline>
    </el-card>

    <!-- 其他链接 -->
    <el-card class="glass-card links-card" shadow="never">
      <template #header>
        <div class="card-header">
          <span>相关链接</span>
        </div>
      </template>

      <div class="links-list">
        <el-button text @click="openWebsite">
          <el-icon><Link /></el-icon>
          访问官网
        </el-button>
        <el-button text @click="openFeedback">
          <el-icon><InfoFilled /></el-icon>
          问题反馈
        </el-button>
      </div>
    </el-card>
  </div>
</template>

<style scoped>
.update-center {
  width: 100%;
  height: 100%;
}

.page-header {
  margin-bottom: 24px;
}

.page-header h2 {
  margin: 0 0 8px 0;
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}

.header-subtitle {
  margin: 0;
  color: #909399;
  font-size: 14px;
}

.version-card {
  margin-bottom: 24px;
}

.current-version {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 0;
  border-bottom: 1px solid #ebeef5;
}

.version-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.version-number {
  font-size: 36px;
  font-weight: 700;
  color: #409EFF;
  line-height: 1;
}

.version-label {
  font-size: 14px;
  color: #909399;
}

.version-status {
  font-size: 16px;
}

.update-actions {
  padding: 20px 0;
  border-bottom: 1px solid #ebeef5;
}

.update-info {
  margin-bottom: 16px;
}

.release-notes {
  margin-top: 12px;
  padding: 12px;
  background: #f5f7fa;
  border-radius: 4px;
}

.release-notes h4 {
  margin: 0 0 8px 0;
  font-size: 14px;
  color: #303133;
}

.download-progress {
  padding: 20px 0;
  text-align: center;
}

.check-actions {
  display: flex;
  align-items: center;
  gap: 16px;
  padding-top: 20px;
}

.last-check-time {
  color: #909399;
  font-size: 13px;
}

.history-card {
  margin-bottom: 24px;
}

.card-header {
  font-weight: 600;
  color: #303133;
}

.version-item h4 {
  margin: 0 0 8px 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.version-item ul {
  margin: 0;
  padding-left: 20px;
  color: #606266;
}

.version-item li {
  margin-bottom: 4px;
}

.links-list {
  display: flex;
  gap: 16px;
}

/* 暗色模式适配 */
.dark .current-version,
.dark .update-actions,
[data-theme="dark"] .current-version,
[data-theme="dark"] .update-actions {
  border-color: rgba(255, 255, 255, 0.08);
}

.dark .release-notes,
[data-theme="dark"] .release-notes {
  background: rgba(255, 255, 255, 0.04);
}

.dark .release-notes h4,
[data-theme="dark"] .release-notes h4 {
  color: var(--text-primary);
}

.dark .version-item h4,
[data-theme="dark"] .version-item h4 {
  color: var(--text-primary);
}

.dark .version-item ul,
[data-theme="dark"] .version-item ul {
  color: var(--text-secondary);
}

.dark .last-check-time,
.dark .version-label,
[data-theme="dark"] .last-check-time,
[data-theme="dark"] .version-label {
  color: var(--text-tertiary);
}
</style>
