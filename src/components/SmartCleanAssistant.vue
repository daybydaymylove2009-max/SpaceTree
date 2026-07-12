<script setup lang="ts">
/**
 * 专业级智能清理助手组件
 * @component SmartCleanAssistant
 * @description 基于规则引擎的智能重复文件清理系统
 */

import { ref, computed, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { 
  Delete, Warning, 
  Setting, TrendCharts, CircleCheck, CircleClose,
  ArrowRight
} from '@element-plus/icons-vue'
import type { DuplicateGroup } from '../types'
import { formatSize } from '../utils/formatters'
import { useFileOperations } from '../composables/useFileOperations'
import { useCleanRules, type CleanPlan } from '../composables/useCleanRules'

interface Props {
  duplicateGroups: DuplicateGroup[]
  dbPath: string
}

const props = defineProps<Props>()

interface Emits {
  (e: 'cleanComplete'): void
}

const emit = defineEmits<Emits>()

// 使用文件操作和规则引擎
const { deleteFile } = useFileOperations({ dbPath: props.dbPath })
const { 
  rules, 
  enabledRules, 
  generateCleanPlan,
  resetRules
} = useCleanRules()

// 当前步骤
const currentStep = ref<'config' | 'analyzing' | 'preview' | 'executing' | 'complete'>('config')

// 清理方案
const cleanPlans = ref<CleanPlan[]>([])

// 选中的清理项
const selectedPlans = ref<Set<string>>(new Set())

// 是否显示规则配置
const showRuleConfig = ref(false)

// 清理统计
const cleanStats = ref({
  totalGroups: 0,
  safeGroups: 0,
  warningGroups: 0,
  dangerGroups: 0,
  totalSpaceToFree: 0,
  selectedSpaceToFree: 0
})

// 执行进度
const executionProgress = ref({
  current: 0,
  total: 0,
  currentFile: ''
})

// 计算属性
const selectedCount = computed(() => selectedPlans.value.size)
const allSelected = computed(() => 
  cleanPlans.value.length > 0 && selectedPlans.value.size === cleanPlans.value.length
)

/**
 * 分析清理方案
 */
async function analyzeCleanPlan() {
  if (props.duplicateGroups.length === 0) {
    ElMessage.warning('没有重复文件需要清理')
    return
  }

  currentStep.value = 'analyzing'
  
  // 延迟以显示加载状态
  await new Promise(resolve => setTimeout(resolve, 500))
  
  // 生成清理方案
  cleanPlans.value = generateCleanPlan(props.duplicateGroups)
  
  // 默认选中所有安全和高置信度的方案
  selectedPlans.value = new Set(
    cleanPlans.value
      .filter(p => p.safety.level !== 'danger' && p.confidence > 70)
      .map(p => p.group.hash)
  )
  
  // 更新统计
  updateStats()
  
  currentStep.value = 'preview'
}

/**
 * 更新统计信息
 */
function updateStats() {
  const stats = {
    totalGroups: cleanPlans.value.length,
    safeGroups: cleanPlans.value.filter(p => p.safety.level === 'safe').length,
    warningGroups: cleanPlans.value.filter(p => p.safety.level === 'warning').length,
    dangerGroups: cleanPlans.value.filter(p => p.safety.level === 'danger').length,
    totalSpaceToFree: cleanPlans.value.reduce((sum, p) => sum + p.spaceToFree, 0),
    selectedSpaceToFree: cleanPlans.value
      .filter(p => selectedPlans.value.has(p.group.hash))
      .reduce((sum, p) => sum + p.spaceToFree, 0)
  }
  cleanStats.value = stats
}

/**
 * 切换方案选择
 */
function togglePlanSelection(plan: CleanPlan) {
  if (selectedPlans.value.has(plan.group.hash)) {
    selectedPlans.value.delete(plan.group.hash)
  } else {
    selectedPlans.value.add(plan.group.hash)
  }
  updateStats()
}

/**
 * 全选/取消全选
 */
function toggleSelectAll() {
  if (allSelected.value) {
    selectedPlans.value.clear()
  } else {
    selectedPlans.value = new Set(cleanPlans.value.map(p => p.group.hash))
  }
  updateStats()
}

/**
 * 获取安全等级标签类型
 */
function getSafetyTagType(level: CleanPlan['safety']['level']) {
  switch (level) {
    case 'safe': return 'success'
    case 'warning': return 'warning'
    case 'danger': return 'danger'
    default: return 'info'
  }
}

/**
 * 获取置信度颜色
 */
function getConfidenceColor(confidence: number): string {
  if (confidence >= 80) return '#67C23A'
  if (confidence >= 60) return '#E6A23C'
  return '#F56C6C'
}

/**
 * 执行清理
 */
async function executeClean() {
  const selectedPlanList = cleanPlans.value.filter(p => 
    selectedPlans.value.has(p.group.hash)
  )

  if (selectedPlanList.length === 0) {
    ElMessage.warning('请先选择要执行的清理方案')
    return
  }

  // 危险确认
  const hasDanger = selectedPlanList.some(p => p.safety.level === 'danger')
  if (hasDanger) {
    const confirmed = await ElMessageBox.confirm(
      '选中的方案包含危险项，确定要继续吗？',
      '危险操作确认',
      {
        confirmButtonText: '确认清理',
        cancelButtonText: '取消',
        type: 'error'
      }
    ).catch(() => false)
    
    if (!confirmed) return
  }

  currentStep.value = 'executing'
  
  let deletedCount = 0
  let failedCount = 0
  const totalFiles = selectedPlanList.reduce((sum, p) => sum + p.deleteFiles.length, 0)
  
  executionProgress.value.total = totalFiles
  executionProgress.value.current = 0

  for (const plan of selectedPlanList) {
    for (const fileScore of plan.deleteFiles) {
      executionProgress.value.currentFile = fileScore.file.filename
      executionProgress.value.current++
      
      try {
        const success = await deleteFile(fileScore.file)
        if (success) {
          deletedCount++
        } else {
          failedCount++
        }
      } catch {
        failedCount++
      }
      
      // 小延迟以显示进度
      await new Promise(resolve => setTimeout(resolve, 50))
    }
  }

  currentStep.value = 'complete'
  
  ElMessage.success(`清理完成！成功删除 ${deletedCount} 个文件`)
  if (failedCount > 0) {
    ElMessage.warning(`${failedCount} 个文件删除失败`)
  }
  
  emit('cleanComplete')
}

/**
 * 返回配置
 */
function backToConfig() {
  currentStep.value = 'config'
  cleanPlans.value = []
  selectedPlans.value.clear()
}

/**
 * 重新分析
 */
function reanalyze() {
  cleanPlans.value = []
  selectedPlans.value.clear()
  analyzeCleanPlan()
}

// 监听规则变化
watch(rules, () => {
  if (currentStep.value === 'preview') {
    // 如果规则变化，提示重新分析
    ElMessage.info('规则已更改，建议重新分析')
  }
}, { deep: true })
</script>

<template>
  <div class="smart-clean-assistant">
    <!-- 配置阶段 -->
    <template v-if="currentStep === 'config'">
      <el-card shadow="hover">
        <template #header>
          <div class="card-header">
            <span class="title">
              <el-icon size="20"><Setting /></el-icon>
              智能清理助手
            </span>
            <el-tag type="info">{{ enabledRules.length }} 个规则启用</el-tag>
          </div>
        </template>

        <div class="assistant-intro">
          <el-alert
            title="基于规则引擎的智能清理系统"
            description="系统将根据启用的规则自动分析重复文件，评估安全性，并生成最优清理方案。支持多维度评分、安全检查和批量操作。"
            type="info"
            :closable="false"
            show-icon
          />
        </div>

        <!-- 规则配置 -->
        <div class="rules-section">
          <div class="section-header" @click="showRuleConfig = !showRuleConfig">
            <span class="section-title">
              <el-icon><Setting /></el-icon>
              清理规则配置
            </span>
            <el-icon class="expand-icon" :class="{ 'is-expanded': showRuleConfig }">
              <ArrowRight />
            </el-icon>
          </div>
          
          <el-collapse-transition>
            <div v-show="showRuleConfig" class="rules-list">
              <div 
                v-for="rule in rules" 
                :key="rule.id"
                class="rule-item"
                :class="{ 'is-disabled': !rule.enabled }"
              >
                <div class="rule-header">
                  <el-checkbox v-model="rule.enabled" />
                  <span class="rule-name">{{ rule.name }}</span>
                  <el-tag :type="rule.priority === 'high' ? 'danger' : rule.priority === 'medium' ? 'warning' : 'info'" size="small">
                    {{ rule.priority === 'high' ? '高' : rule.priority === 'medium' ? '中' : '低' }}优先级
                  </el-tag>
                </div>
                <p class="rule-desc">{{ rule.description }}</p>
              </div>
              
              <div class="rules-actions">
                <el-button size="small" @click="resetRules">重置默认规则</el-button>
              </div>
            </div>
          </el-collapse-transition>
        </div>

        <!-- 操作按钮 -->
        <div class="action-section">
          <el-button 
            type="primary" 
            size="large" 
            @click="analyzeCleanPlan"
            :disabled="duplicateGroups.length === 0"
          >
            <el-icon><TrendCharts /></el-icon>
            开始智能分析
          </el-button>
          <p class="hint" v-if="duplicateGroups.length === 0">
            暂无重复文件数据，请先进行扫描
          </p>
        </div>
      </el-card>
    </template>

    <!-- 分析中 -->
    <template v-if="currentStep === 'analyzing'">
      <el-card shadow="hover">
        <div class="analyzing-state">
          <el-icon class="is-loading" size="48"><TrendCharts /></el-icon>
          <h3>正在分析清理方案...</h3>
          <p>系统正在评估 {{ duplicateGroups.length }} 个重复文件组</p>
          <p>应用 {{ enabledRules.length }} 条清理规则进行智能评分</p>
        </div>
      </el-card>
    </template>

    <!-- 预览阶段 -->
    <template v-if="currentStep === 'preview'">
      <el-card shadow="hover">
        <template #header>
          <div class="card-header">
            <span class="title">清理方案预览</span>
            <div class="header-actions">
              <el-button size="small" @click="backToConfig">
                <el-icon><ArrowRight /></el-icon>
                返回配置
              </el-button>
              <el-button size="small" type="primary" @click="reanalyze">
                <el-icon><TrendCharts /></el-icon>
                重新分析
              </el-button>
            </div>
          </div>
        </template>

        <!-- 统计概览 -->
        <div class="stats-overview">
          <el-row :gutter="16">
            <el-col :span="6">
              <div class="stat-item">
                <div class="stat-value">{{ cleanStats.totalGroups }}</div>
                <div class="stat-label">可清理组</div>
              </div>
            </el-col>
            <el-col :span="6">
              <div class="stat-item safe">
                <div class="stat-value">{{ cleanStats.safeGroups }}</div>
                <div class="stat-label">安全</div>
              </div>
            </el-col>
            <el-col :span="6">
              <div class="stat-item warning">
                <div class="stat-value">{{ cleanStats.warningGroups }}</div>
                <div class="stat-label">警告</div>
              </div>
            </el-col>
            <el-col :span="6">
              <div class="stat-item danger">
                <div class="stat-value">{{ formatSize(cleanStats.selectedSpaceToFree) }}</div>
                <div class="stat-label">预计释放</div>
              </div>
            </el-col>
          </el-row>
        </div>

        <!-- 选择控制 -->
        <div class="selection-control">
          <el-checkbox :model-value="allSelected" @change="toggleSelectAll">
            全选 (已选 {{ selectedCount }} / {{ cleanPlans.length }} 项)
          </el-checkbox>
        </div>

        <!-- 清理方案列表 -->
        <div class="plans-list">
          <div 
            v-for="plan in cleanPlans" 
            :key="plan.group.hash"
            class="plan-item"
            :class="{ 'is-selected': selectedPlans.has(plan.group.hash) }"
          >
            <div class="plan-header" @click="togglePlanSelection(plan)">
              <el-checkbox 
                :model-value="selectedPlans.has(plan.group.hash)"
                @click.stop
                @change="togglePlanSelection(plan)"
              />
              
              <div class="plan-info">
                <span class="file-name">{{ plan.keepFile.file.filename }}</span>
                <span class="file-count">{{ plan.group.files.length }} 个副本</span>
              </div>
              
              <div class="plan-tags">
                <el-tag :type="getSafetyTagType(plan.safety.level)" size="small">
                  {{ plan.safety.level === 'safe' ? '安全' : plan.safety.level === 'warning' ? '警告' : '危险' }}
                </el-tag>
                <el-tag type="info" size="small">{{ formatSize(plan.spaceToFree) }}</el-tag>
              </div>
              
              <div class="confidence-bar">
                <el-progress 
                  :percentage="plan.confidence" 
                  :color="getConfidenceColor(plan.confidence)"
                  :stroke-width="4"
                  :show-text="false"
                />
                <span class="confidence-text">{{ plan.confidence }}% 置信度</span>
              </div>
            </div>

            <div class="plan-details">
              <!-- 保留文件 -->
              <div class="detail-section keep">
                <div class="section-title">
                  <el-icon><CircleCheck /></el-icon>
                  保留 (评分: {{ plan.keepFile.score }})
                </div>
                <div class="file-info">
                  <span class="path">{{ plan.keepFile.file.path }}</span>
                  <div class="reasons">
                    <el-tag v-for="reason in plan.keepFile.reasons" :key="reason" size="small" type="success">
                      {{ reason }}
                    </el-tag>
                    <el-tag v-for="warning in plan.keepFile.warnings" :key="warning" size="small" type="warning">
                      {{ warning }}
                    </el-tag>
                  </div>
                </div>
              </div>

              <!-- 删除文件 -->
              <div class="detail-section delete">
                <div class="section-title">
                  <el-icon><CircleClose /></el-icon>
                  删除 ({{ plan.deleteFiles.length }} 个文件)
                </div>
                <div class="delete-list">
                  <div v-for="fileScore in plan.deleteFiles" :key="fileScore.file.path" class="delete-item">
                    <span class="path">{{ fileScore.file.path }}</span>
                    <span class="size">{{ formatSize(fileScore.file.size) }}</span>
                  </div>
                </div>
              </div>

              <!-- 安全提示 -->
              <div v-if="plan.safety.details.length > 0" class="detail-section safety">
                <div class="section-title">
                  <el-icon><Warning /></el-icon>
                  安全提示
                </div>
                <ul>
                  <li v-for="detail in plan.safety.details" :key="detail">{{ detail }}</li>
                </ul>
              </div>
            </div>
          </div>
        </div>

        <!-- 执行按钮 -->
        <div class="execute-section">
          <el-button 
            type="danger" 
            size="large" 
            @click="executeClean"
            :disabled="selectedCount === 0"
          >
            <el-icon><Delete /></el-icon>
            执行清理 ({{ selectedCount }} 项，释放 {{ formatSize(cleanStats.selectedSpaceToFree) }})
          </el-button>
        </div>
      </el-card>
    </template>

    <!-- 执行中 -->
    <template v-if="currentStep === 'executing'">
      <el-card shadow="hover">
        <div class="executing-state">
          <el-progress 
            :percentage="Math.round((executionProgress.current / executionProgress.total) * 100)"
            :stroke-width="20"
            status="success"
          />
          <h3>正在执行清理...</h3>
          <p>正在删除: {{ executionProgress.currentFile }}</p>
          <p>{{ executionProgress.current }} / {{ executionProgress.total }} 个文件</p>
        </div>
      </el-card>
    </template>

    <!-- 完成 -->
    <template v-if="currentStep === 'complete'">
      <el-card shadow="hover">
        <div class="complete-state">
          <el-icon size="64" color="#67C23A"><CircleCheck /></el-icon>
          <h3>清理完成</h3>
          <p>已成功释放 {{ formatSize(cleanStats.selectedSpaceToFree) }} 存储空间</p>
          <el-button type="primary" @click="backToConfig">返回</el-button>
        </div>
      </el-card>
    </template>
  </div>
</template>

<style scoped>
.smart-clean-assistant {
  margin-bottom: 24px;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 16px;
}

.assistant-intro {
  margin-bottom: 20px;
}

.rules-section {
  margin-bottom: 20px;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  background: #f5f7fa;
  cursor: pointer;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 500;
}

.expand-icon {
  transition: transform 0.3s;
}

.expand-icon.is-expanded {
  transform: rotate(90deg);
}

.rules-list {
  padding: 16px;
}

.rule-item {
  padding: 12px;
  border-bottom: 1px solid #ebeef5;
}

.rule-item:last-child {
  border-bottom: none;
}

.rule-item.is-disabled {
  opacity: 0.6;
}

.rule-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 4px;
}

.rule-name {
  font-weight: 500;
}

.rule-desc {
  margin: 4px 0 0 32px;
  color: #909399;
  font-size: 13px;
}

.rules-actions {
  margin-top: 16px;
  text-align: right;
}

.action-section {
  text-align: center;
  padding: 20px;
}

.hint {
  margin-top: 12px;
  color: #909399;
}

.analyzing-state {
  text-align: center;
  padding: 40px;
}

.analyzing-state h3 {
  margin: 20px 0 12px;
}

.analyzing-state p {
  color: #909399;
  margin: 4px 0;
}

.stats-overview {
  margin-bottom: 20px;
}

.stat-item {
  text-align: center;
  padding: 16px;
  background: #f5f7fa;
  border-radius: 4px;
}

.stat-item.safe {
  background: #f0f9ff;
}

.stat-item.warning {
  background: #fdf6ec;
}

.stat-item.danger {
  background: #fef0f0;
}

.stat-value {
  font-size: 24px;
  font-weight: 700;
  color: #303133;
}

.stat-label {
  margin-top: 4px;
  color: #909399;
  font-size: 13px;
}

.selection-control {
  margin-bottom: 16px;
  padding: 12px;
  background: #f5f7fa;
  border-radius: 4px;
}

.plans-list {
  max-height: 500px;
  overflow-y: auto;
}

.plan-item {
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  margin-bottom: 12px;
  overflow: hidden;
}

.plan-item.is-selected {
  border-color: #409EFF;
  box-shadow: 0 0 0 2px rgba(64, 158, 255, 0.2);
}

.plan-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: #f5f7fa;
  cursor: pointer;
}

.plan-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.file-name {
  font-weight: 500;
}

.file-count {
  font-size: 12px;
  color: #909399;
}

.plan-tags {
  display: flex;
  gap: 8px;
}

.confidence-bar {
  width: 100px;
  text-align: center;
}

.confidence-text {
  font-size: 12px;
  color: #909399;
}

.plan-details {
  padding: 16px;
  border-top: 1px solid #ebeef5;
}

.detail-section {
  margin-bottom: 16px;
}

.detail-section:last-child {
  margin-bottom: 0;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 500;
  margin-bottom: 8px;
}

.detail-section.keep .section-title {
  color: #67C23A;
}

.detail-section.delete .section-title {
  color: #F56C6C;
}

.detail-section.safety .section-title {
  color: #E6A23C;
}

.file-info {
  padding: 12px;
  background: #f0f9ff;
  border-radius: 4px;
}

.path {
  display: block;
  margin-bottom: 8px;
  word-break: break-all;
}

.reasons {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.delete-list {
  max-height: 150px;
  overflow-y: auto;
}

.delete-item {
  display: flex;
  justify-content: space-between;
  padding: 8px 12px;
  background: #fef0f0;
  border-radius: 4px;
  margin-bottom: 8px;
}

.delete-item:last-child {
  margin-bottom: 0;
}

.delete-item .size {
  color: #909399;
  font-size: 13px;
}

.execute-section {
  margin-top: 20px;
  text-align: center;
}

.executing-state {
  text-align: center;
  padding: 40px;
}

.executing-state h3 {
  margin: 20px 0 12px;
}

.complete-state {
  text-align: center;
  padding: 40px;
}

.complete-state h3 {
  margin: 20px 0 12px;
}

.complete-state p {
  color: #909399;
  margin-bottom: 20px;
}
</style>
