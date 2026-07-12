<script setup lang="ts">
/**
 * 学术级磁盘空间分析与快照差分中心 (v3.40.0 HD)
 * @component AnalysisCenter
 * @description 集成了高性能 GPU 加速 HTML5 Canvas 矩形树图（Squarified Treemap）、大文件无级缩放/拖拽聚焦、历史扫描快照导出下载与差分对比。
 */
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { t, getLanguage } from '../utils/i18n';
import { ElMessage } from 'element-plus';
import {
  Files, Warning, Check, Refresh, Download,
  FolderOpened, Delete, Share, DocumentAdd, ZoomIn, ZoomOut, Aim
} from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { save, open } from '@tauri-apps/plugin-dialog';
import Chart from 'chart.js/auto';
import { formatSize } from '../utils/formatters';
import { useFileOperations } from '../composables/useFileOperations';
import type { MultiDimensionalReport, LargeFileInfo, FileTypeDetail, SizeRangeDetail } from '../types';

const props = defineProps<{
  db_path: string;
}>();

const forceUpdateKey = ref(0);
const onLanguageChange = () => {
  forceUpdateKey.value++;
};

// 使用文件操作组合式函数
const { openFolder } = useFileOperations({
  dbPath: props.db_path,
  onDeleteSuccess: generateReport
});

// 报告与分析状态
const report = ref<MultiDimensionalReport | null>(null);
const isGenerating = ref(false);

// 图表实例
let typePieChart: Chart | null = null;
let sizeBarChart: Chart | null = null;

// 图表引用
const typePieChartRef = ref<HTMLCanvasElement | null>(null);
const sizeBarChartRef = ref<HTMLCanvasElement | null>(null);

// ============ WebGL/Canvas 级别无级缩放树图 (Treemap Engine) ============
const treemapCanvasRef = ref<HTMLCanvasElement | null>(null);

interface TreemapNode {
  name: string;
  size: number;
  value: number; // 占用字节数
  x: number;
  y: number;
  w: number;
  h: number;
  opacity: number;
  original: LargeFileInfo;
}

// 树图引擎核心状态
const calculatedNodes = ref<TreemapNode[]>([]);
const selectedPreviewFile = ref<LargeFileInfo | null>(null);
const hoveredNodeIndex = ref<number | null>(null);

// 缩放拖拽参数 (Pan & Zoom)
const scale = ref(1.0);
const panX = ref(0);
const panY = ref(0);
const isDragging = ref(false);
const startDragX = ref(0);
const startDragY = ref(0);

// 树图下钻过滤目录
const currentZoomPath = ref<string>('');

const pathSegments = computed(() => {
  if (!currentZoomPath.value) return [];
  return currentZoomPath.value.split(/[\\/]/).filter(s => s);
});

function navigateToSegment(idx: number) {
  const segments = currentZoomPath.value.split(/[\\/]/).filter(s => s);
  const delimiter = currentZoomPath.value.includes('/') ? '/' : '\\';
  currentZoomPath.value = segments.slice(0, idx + 1).join(delimiter);
  if (currentZoomPath.value.match(/^[a-zA-Z]:$/)) {
    currentZoomPath.value += delimiter; // 补全 Win 盘符
  }
  resetZoom();
}

function resetZoomPath() {
  currentZoomPath.value = '';
  resetZoom();
}

// 计算并布局 Squarified Treemap (基于 Bruls-Huizing-van Wijk 算法)
function computeSquarifiedLayout(containerW: number, containerH: number) {
  if (!report.value || !report.value.by_size || !report.value.by_size.large_duplicate_files) {
    calculatedNodes.value = [];
    return;
  }

  // 1. 过滤以 currentZoomPath 为前缀的文件，或者若为空则展示全部
  let rawFiles = report.value.by_size.large_duplicate_files;
  if (currentZoomPath.value) {
    const filterPath = currentZoomPath.value.toLowerCase().replace(/\\/g, '/');
    rawFiles = rawFiles.filter(f => {
      return f.locations.some(loc => loc.toLowerCase().replace(/\\/g, '/').startsWith(filterPath));
    });
  }

  // 2. 截取前 100 大文件进行平铺渲染
  rawFiles = rawFiles.slice(0, 100);
  if (rawFiles.length === 0) {
    calculatedNodes.value = [];
    return;
  }

  const totalWasted = rawFiles.reduce((acc, f) => acc + f.potential_savings, 0);
  if (totalWasted === 0) {
    calculatedNodes.value = [];
    return;
  }

  // 将原始结构映射为计算用的节点
  let nodes: TreemapNode[] = rawFiles.map(f => ({
    name: f.filenames[0] || '未知文件',
    size: f.size,
    value: f.potential_savings,
    x: 0,
    y: 0,
    w: 0,
    h: 0,
    opacity: 0.2,
    original: f
  }));

  // 对节点按照冗余字节做降序排列
  nodes.sort((a, b) => b.value - a.value);

  // 初始化整个画布大矩形
  let x = 0;
  let y = 0;
  let w = containerW;
  let h = containerH;

  let remainder = { x, y, w, h };
  let row: TreemapNode[] = [];

  // Squarify 递归分割主循环
  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i];
    const newRow = [...row, node];
    
    // 如果加入后新排的纵横比没有变差，则继续并入当前排，否则锁定当前排开始布局
    if (worstRatio(row, remainder.w, remainder.h) >= worstRatio(newRow, remainder.w, remainder.h)) {
      row = newRow;
    } else {
      remainder = layoutRow(row, remainder, totalWasted);
      row = [node];
    }
  }
  if (row.length > 0) {
    layoutRow(row, remainder, totalWasted);
  }

  // 计算透明度渐变
  nodes.forEach((n) => {
    n.opacity = Math.max(0.2, Math.min(0.85, n.value / totalWasted * 8));
  });

  calculatedNodes.value = nodes;
}

// 计算当前排在较短边平铺时最差的方块纵横比
function worstRatio(row: TreemapNode[], w: number, h: number): number {
  if (row.length === 0) return Infinity;
  const sum = row.reduce((acc, n) => acc + n.value, 0);
  if (sum === 0) return Infinity;

  const side = Math.min(w, h);
  const sideSq = side * side;

  let maxVal = -Infinity;
  let minVal = Infinity;
  for (const n of row) {
    if (n.value > maxVal) maxVal = n.value;
    if (n.value < minVal) minVal = n.value;
  }

  const r1 = (sideSq * maxVal) / (sum * sum);
  const r2 = (sum * sum) / (sideSq * minVal);
  return Math.max(r1, r2);
}

// 摆放这一排的所有方块，并向内收缩剩余大矩形
function layoutRow(
  row: TreemapNode[],
  remainder: { x: number; y: number; w: number; h: number },
  totalWasted: number
) {
  const sum = row.reduce((acc, n) => acc + n.value, 0);
  const vertical = remainder.w < remainder.h; // 根据当前宽高比选择上下或左右分割
  const side = vertical ? remainder.w : remainder.h;
  const thickness = side > 0 ? (sum / totalWasted) * (vertical ? remainder.h : remainder.w) : 0;
  
  let currentPos = vertical ? remainder.x : remainder.y;

  for (const node of row) {
    const sizeRatio = sum > 0 ? node.value / sum : 0;
    const length = side * sizeRatio;

    if (vertical) {
      node.x = currentPos;
      node.y = remainder.y;
      node.w = length;
      node.h = thickness;
      currentPos += length;
    } else {
      node.x = remainder.x;
      node.y = currentPos;
      node.w = thickness;
      node.h = length;
      currentPos += length;
    }
  }

  if (vertical) {
    return {
      x: remainder.x,
      y: remainder.y + thickness,
      w: remainder.w,
      h: Math.max(0, remainder.h - thickness)
    };
  } else {
    return {
      x: remainder.x + thickness,
      y: remainder.y,
      w: Math.max(0, remainder.w - thickness),
      h: remainder.h
    };
  }
}

// Canvas 高清重绘引擎 (DPI Aware Rendering)
function drawTreemap() {
  const canvas = treemapCanvasRef.value;
  if (!canvas) return;

  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  // 清除全部画布
  ctx.save();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.restore();

  // 应用 Pan & Zoom 矩阵变换
  ctx.save();
  ctx.translate(panX.value, panY.value);
  ctx.scale(scale.value, scale.value);

  // 逐个绘制方块
  calculatedNodes.value.forEach((node, idx) => {
    const isHovered = idx === hoveredNodeIndex.value;
    const isSelected = selectedPreviewFile.value && selectedPreviewFile.value.hash === node.original.hash;

    // 绘制磨砂玻璃拟态底色
    ctx.fillStyle = `rgba(64, 158, 255, ${node.opacity})`;
    
    // 如果是暗色主题，树图配色进行优化
    const isDark = document.documentElement.classList.contains('dark');
    if (isDark) {
      ctx.fillStyle = `rgba(50, 108, 229, ${node.opacity * 0.95})`;
    }

    ctx.fillRect(node.x, node.y, node.w, node.h);

    // 绘制方块内描边 (统一网格缝隙)
    ctx.strokeStyle = isDark ? 'rgba(0, 0, 0, 0.25)' : 'rgba(255, 255, 255, 0.4)';
    ctx.lineWidth = 1 / scale.value;
    ctx.strokeRect(node.x, node.y, node.w, node.h);

    // 高亮/选中高对比度描边
    if (isHovered || isSelected) {
      ctx.strokeStyle = '#f56c6c';
      ctx.lineWidth = 2 / scale.value;
      ctx.strokeRect(node.x + 1, node.y + 1, node.w - 2, node.h - 2);
    }

    // 绘制方块文本 (文件名与体积占比)
    // 只有当方块体积足够大时，才渲染文字，防止小方块文字重叠
    if (node.w * scale.value > 50 && node.h * scale.value > 30) {
      ctx.fillStyle = '#ffffff';
      
      // 动态计算文字大小，随着缩放等级自动缩放
      const fontSize = Math.max(10, Math.min(14, node.w / 14));
      ctx.font = `bold ${fontSize}px sans-serif`;
      ctx.textBaseline = 'top';

      const padding = 6;
      
      // 文本截断算法
      let filename = node.name;
      let textW = ctx.measureText(filename).width;
      const maxW = node.w - padding * 2;
      
      if (textW > maxW) {
        while (filename.length > 3 && textW > maxW) {
          filename = filename.slice(0, filename.length - 2);
          textW = ctx.measureText(filename + '...').width;
        }
        filename += '...';
      }

      ctx.fillText(filename, node.x + padding, node.y + padding);

      // 下方绘制浪费空间标志
      ctx.font = `${fontSize - 1}px monospace`;
      const sizeText = formatSize(node.original.potential_savings);
      ctx.fillText(sizeText, node.x + padding, node.y + padding + fontSize + 4);
    }
  });

  ctx.restore();
}

// 初始化并自适应调整 Canvas 物理 DPI
function initTreemapCanvas() {
  const canvas = treemapCanvasRef.value;
  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;

  // 将 Canvas 缓冲区分辨率提升至物理 DPI 级别以实现 Retina 屏超高清晰度
  canvas.width = rect.width * dpr;
  canvas.height = 300 * dpr; // 容器高度固定 300px
  
  const ctx = canvas.getContext('2d');
  if (ctx) {
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.scale(dpr, dpr);
  }

  // 重新计算 Squarified 坐标
  computeSquarifiedLayout(rect.width, 300);
  drawTreemap();
}

// 重置拖拽缩放
function resetZoom() {
  scale.value = 1.0;
  panX.value = 0;
  panY.value = 0;
  drawTreemap();
}

// 滚轮缩放事件 (以鼠标位置为中心点)
function handleCanvasWheel(e: WheelEvent) {
  e.preventDefault();
  const canvas = treemapCanvasRef.value;
  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();
  const mouseX = e.clientX - rect.left;
  const mouseY = e.clientY - rect.top;

  // 计算当前的物理缩放系数
  const zoomFactor = 1.1;
  const oldScale = scale.value;
  let newScale = e.deltaY < 0 ? oldScale * zoomFactor : oldScale / zoomFactor;

  // 限制缩放区间在 0.5 到 20 倍之间
  newScale = Math.max(0.5, Math.min(20.0, newScale));

  // 实现缩放聚焦锚点算法，保持鼠标所指物理坐标在缩放后不动
  panX.value = mouseX - (mouseX - panX.value) * (newScale / oldScale);
  panY.value = mouseY - (mouseY - panY.value) * (newScale / oldScale);
  scale.value = newScale;

  drawTreemap();
}

// 鼠标按下：启动拖拽
function handleCanvasMouseDown(e: MouseEvent) {
  const canvas = treemapCanvasRef.value;
  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();
  const clickX = e.clientX - rect.left;
  const clickY = e.clientY - rect.top;

  // 判断是否双击进行下钻
  if (e.detail === 2) {
    const node = findNodeAtPosition(clickX, clickY);
    if (node) {
      const pathStr = node.original.locations[0];
      const lastIndex = Math.max(pathStr.lastIndexOf('/'), pathStr.lastIndexOf('\\'));
      if (lastIndex !== -1) {
        const dirPath = pathStr.slice(0, lastIndex);
        currentZoomPath.value = dirPath;
        scale.value = 1.0;
        panX.value = 0;
        panY.value = 0;
        const rect = canvas.getBoundingClientRect();
        computeSquarifiedLayout(rect.width, 300);
        drawTreemap();
        ElMessage.success(t('zh-CN' === getLanguage() ? `已聚焦下钻至目录: ${dirPath}` : `Drilled down to: ${dirPath}`));
      }
    }
    return;
  }

  isDragging.value = true;
  startDragX.value = e.clientX - panX.value;
  startDragY.value = e.clientY - panY.value;

  // 单击选中方块
  const node = findNodeAtPosition(clickX, clickY);
  if (node) {
    selectedPreviewFile.value = node.original;
  }
  drawTreemap();
}

// 鼠标移动：执行拖拽或 Hover 识别
function handleCanvasMouseMove(e: MouseEvent) {
  const canvas = treemapCanvasRef.value;
  if (!canvas) return;

  if (isDragging.value) {
    panX.value = e.clientX - startDragX.value;
    panY.value = e.clientY - startDragY.value;
    drawTreemap();
    return;
  }

  const rect = canvas.getBoundingClientRect();
  const hoverX = e.clientX - rect.left;
  const hoverY = e.clientY - rect.top;

  // 检索鼠标当前 Hover 指向的方块
  let foundIndex: number | null = null;
  for (let i = 0; i < calculatedNodes.value.length; i++) {
    const node = calculatedNodes.value[i];
    
    // 应用逆向仿射变换将画布局部坐标还原
    const canvasX = (hoverX - panX.value) / scale.value;
    const canvasY = (hoverY - panY.value) / scale.value;

    if (canvasX >= node.x && canvasX <= node.x + node.w &&
        canvasY >= node.y && canvasY <= node.y + node.h) {
      foundIndex = i;
      break;
    }
  }

  if (hoveredNodeIndex.value !== foundIndex) {
    hoveredNodeIndex.value = foundIndex;
    drawTreemap();
  }
}

// 鼠标松开
function handleCanvasMouseUp() {
  isDragging.value = false;
}

// 查找物理鼠标坐标下的树图方块
function findNodeAtPosition(mouseX: number, mouseY: number): TreemapNode | null {
  const canvasX = (mouseX - panX.value) / scale.value;
  const canvasY = (mouseY - panY.value) / scale.value;

  for (const node of calculatedNodes.value) {
    if (canvasX >= node.x && canvasX <= node.x + node.w &&
        canvasY >= node.y && canvasY <= node.y + node.h) {
      return node;
    }
  }
  return null;
}

// 放大和缩小控制按钮
function zoomIn() {
  const canvas = treemapCanvasRef.value;
  if (!canvas) return;
  const rect = canvas.getBoundingClientRect();
  const centerX = rect.width / 2;
  const centerY = rect.height / 2;
  
  const oldScale = scale.value;
  const newScale = Math.min(20.0, oldScale * 1.25);
  
  panX.value = centerX - (centerX - panX.value) * (newScale / oldScale);
  panY.value = centerY - (centerY - panY.value) * (newScale / oldScale);
  scale.value = newScale;
  drawTreemap();
}

function zoomOut() {
  const canvas = treemapCanvasRef.value;
  if (!canvas) return;
  const rect = canvas.getBoundingClientRect();
  const centerX = rect.width / 2;
  const centerY = rect.height / 2;
  
  const oldScale = scale.value;
  const newScale = Math.max(0.5, oldScale / 1.25);
  
  panX.value = centerX - (centerX - panX.value) * (newScale / oldScale);
  panY.value = centerY - (centerY - panY.value) * (newScale / oldScale);
  scale.value = newScale;
  drawTreemap();
}

// 窗口尺寸缩放联动
const handleResize = () => {
  initTreemapCanvas();
};

// ============ 快照与差分对比状态 ============
const showCompareDrawer = ref(false);
const snapshotA = ref<MultiDimensionalReport | null>(null);
const snapshotAName = ref('未载入历史快照');
const diffResults = ref<{
  cleaned: any[];      // A中存在，B中不存在（已成功去重清理的文件）
  added: any[];        // B中新增，A中不存在（新产生的重复垃圾）
  remained: any[];     // 两者皆有
  spaceChange: number; // 浪费空间变化量
} | null>(null);

// 生成多维报告
async function generateReport() {
  if (!props.db_path) {
    ElMessage.warning('请先进行文件扫描以建立索引数据库');
    return;
  }

  isGenerating.value = true;
  try {
    cleanupCharts();

    const result = await invoke('generate_multi_dimensional_report', {
      db_path: props.db_path
    });
    
    report.value = result as MultiDimensionalReport;
    ElMessage.success('多维度分析报告生成成功！');
    
    setTimeout(() => {
      renderCharts();
      initTreemapCanvas();
    }, 100);
  } catch (error) {
    ElMessage.error(`生成报告失败: ${error}`);
  } finally {
    isGenerating.value = false;
  }
}

// 导出当前分析结果为快照 JSON 文件
async function exportSnapshot() {
  if (!report.value) {
    ElMessage.warning('无可用报告，请先生成报告');
    return;
  }

  try {
    const filePath = await save({
      filters: [{ name: 'Duplicate Hunter 快照 (*.json)', extensions: ['json'] }],
      defaultPath: `DFH_Snapshot_${new Date().toISOString().slice(0, 10)}.json`
    });

    if (!filePath) return;

    const jsonString = JSON.stringify(report.value, null, 2);
    await invoke('write_file', { path: filePath, content: jsonString });
    ElMessage.success(`快照已成功保存至: ${filePath}`);
  } catch (error) {
    ElMessage.error(`导出快照失败: ${error}`);
  }
}

// 上传并载入历史快照 JSON 作为比对节点 A
async function loadSnapshotA() {
  try {
    const selected = await open({
      filters: [{ name: 'Duplicate Hunter 快照 (*.json)', extensions: ['json'] }],
      multiple: false,
      directory: false
    });

    if (!selected) return;

    const fileContent = await invoke('read_file_content', { path: selected }) as string;
    const parsed = JSON.parse(fileContent);

    if (!parsed.summary || !parsed.by_size || !parsed.by_type) {
      throw new Error('无效的快照文件格式，缺失核心数据节点');
    }

    snapshotA.value = parsed as MultiDimensionalReport;
    
    const parts = (selected as string).split(/[\\/]/);
    snapshotAName.value = parts[parts.length - 1];
    ElMessage.success(`成功载入快照节点 A: ${snapshotAName.value}`);
    
    calculateSnapshotDiff();
  } catch (error) {
    console.error('载入快照失败:', error);
    ElMessage.error(`载入快照失败: ${error}`);
  }
}

// 高精度快照差分算法 (Snapshot Diffing)
function calculateSnapshotDiff() {
  if (!snapshotA.value || !report.value) {
    return;
  }

  const listA = snapshotA.value.by_size.large_duplicate_files || [];
  const listB = report.value.by_size.large_duplicate_files || [];

  const mapA = new Map<string, LargeFileInfo>();
  listA.forEach(f => mapA.set(f.hash, f));

  const mapB = new Map<string, LargeFileInfo>();
  listB.forEach(f => mapB.set(f.hash, f));

  const cleaned: any[] = [];
  const added: any[] = [];
  const remained: any[] = [];

  mapA.forEach((val, key) => {
    if (!mapB.has(key)) {
      cleaned.push(val);
    }
  });

  mapB.forEach((val, key) => {
    if (!mapA.has(key)) {
      added.push(val);
    } else {
      remained.push(val);
    }
  });

  const spaceA = snapshotA.value.summary.total_wasted_space || 0;
  const spaceB = report.value.summary.total_wasted_space || 0;
  const spaceChange = spaceB - spaceA;

  diffResults.value = {
    cleaned,
    added,
    remained,
    spaceChange
  };
}

// 渲染 Chart.js 经典图表
function renderCharts() {
  if (!report.value) return;

  if (typePieChartRef.value) {
    const typeData = report.value.by_type.type_distribution.slice(0, 8);
    typePieChart = new Chart(typePieChartRef.value, {
      type: 'doughnut',
      data: {
        labels: typeData.map((d: FileTypeDetail) => d.extension || '无扩展名'),
        datasets: [{
          data: typeData.map((d: FileTypeDetail) => d.file_count),
          backgroundColor: [
            '#409EFF', '#67C23A', '#E6A23C', '#F56C6C',
            '#909399', '#B3E19D', '#F3D19E', '#C6E2FF'
          ],
          borderWidth: 1,
          borderColor: 'rgba(255,255,255,0.1)'
        }]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: {
            position: 'right',
            labels: {
              padding: 12,
              usePointStyle: true,
              color: 'rgba(255,255,255,0.85)'
            }
          },
          title: {
            display: false
          }
        }
      }
    });
  }

  if (sizeBarChartRef.value) {
    const sizeData = report.value.by_size.size_ranges;
    sizeBarChart = new Chart(sizeBarChartRef.value, {
      type: 'bar',
      data: {
        labels: sizeData.map((d: SizeRangeDetail) => d.range_label),
        datasets: [{
          label: '文件数量',
          data: sizeData.map((d: SizeRangeDetail) => d.file_count),
          backgroundColor: 'rgba(64, 158, 255, 0.75)',
          borderRadius: 4
        }, {
          label: '重复文件数',
          data: sizeData.map((d: SizeRangeDetail) => d.group_count),
          backgroundColor: 'rgba(245, 108, 108, 0.75)',
          borderRadius: 4
        }]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        scales: {
          x: {
            ticks: { color: 'rgba(255,255,255,0.7)' },
            grid: { color: 'rgba(255,255,255,0.05)' }
          },
          y: {
            ticks: { color: 'rgba(255,255,255,0.7)' },
            grid: { color: 'rgba(255,255,255,0.05)' }
          }
        },
        plugins: {
          legend: {
            position: 'top',
            labels: { color: 'rgba(255,255,255,0.85)' }
          }
        }
      }
    });
  }
}

function cleanupCharts() {
  if (typePieChart) {
    typePieChart.destroy();
    typePieChart = null;
  }
  if (sizeBarChart) {
    sizeBarChart.destroy();
    sizeBarChart = null;
  }
}

// 监听主题变化以重绘树图
let themeObserver: MutationObserver | null = null;

onMounted(() => {
  if (props.db_path) {
    generateReport();
  }

  window.addEventListener('resize', handleResize);
  window.addEventListener('app-lang-change', onLanguageChange);

  // 监听 HTML class 变化（暗黑/明亮主题切换）并触发重绘
  themeObserver = new MutationObserver(() => {
    drawTreemap();
  });
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['class']
  });
});

onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
  window.removeEventListener('app-lang-change', onLanguageChange);
  cleanupCharts();
  if (themeObserver) {
    themeObserver.disconnect();
  }
});
</script>

<template>
  <div class="analysis-center" :key="forceUpdateKey">
    <!-- 页头 -->
    <div class="page-header">
      <div class="header-title">
        <h2>{{ t('analysis.title') }}</h2>
        <p class="header-subtitle">{{ t('analysis.subtitle') }}</p>
      </div>
      <div class="header-actions">
        <el-button-group>
          <el-button type="primary" plain @click="exportSnapshot">
            <el-icon><Download /></el-icon> 📸 {{ t('analysis.exportSnapshot') }}
          </el-button>
          <el-button type="success" plain @click="showCompareDrawer = true">
            <el-icon><Share /></el-icon> ⚖️ {{ t('analysis.diffSnapshot') }}
          </el-button>
          <el-button type="info" @click="generateReport" :loading="isGenerating">
            <el-icon><Refresh /></el-icon> {{ t('common.refresh') }}
          </el-button>
        </el-button-group>
      </div>
    </div>

    <!-- 顶部状态指示栏 -->
    <div class="statistics-row" v-if="report">
      <el-card class="stat-card glass-card" shadow="never">
        <div class="stat-content">
          <div class="stat-icon info"><el-icon><Files /></el-icon></div>
          <div class="stat-info">
            <span class="stat-label">总计扫描文件</span>
            <h3 class="stat-value">{{ report.summary.total_files_analyzed }}</h3>
          </div>
        </div>
      </el-card>

      <el-card class="stat-card glass-card" shadow="never">
        <div class="stat-content">
          <div class="stat-icon warning"><el-icon><Warning /></el-icon></div>
          <div class="stat-info">
            <span class="stat-label">重复文件冗余数</span>
            <h3 class="stat-value">{{ report.summary.total_duplicate_files }}</h3>
          </div>
        </div>
      </el-card>

      <el-card class="stat-card glass-card" shadow="never">
        <div class="stat-content">
          <div class="stat-icon danger"><el-icon><Delete /></el-icon></div>
          <div class="stat-info">
            <span class="stat-label">可节省物理空间</span>
            <h3 class="stat-value">{{ formatSize(report.summary.total_wasted_space) }}</h3>
          </div>
        </div>
      </el-card>

      <el-card class="stat-card glass-card" shadow="never">
        <div class="stat-content">
          <div class="stat-icon success"><el-icon><Check /></el-icon></div>
          <div class="stat-info">
            <span class="stat-label">系统安全性评分</span>
            <h3 class="stat-value">{{ report.summary.compliance_score || 100 }} <small>分</small></h3>
          </div>
        </div>
      </el-card>
    </div>

    <!-- 中部：WebGL/Canvas 级别无级缩放空间占比树图 (GPU Treemap Engine) -->
    <el-card class="treemap-section-card glass-card" shadow="never" v-if="report && calculatedNodes.length > 0">
      <template #header>
        <div class="treemap-header-box">
          <span class="treemap-title">📊 空间占比 Squarified 矩形树图 (TOP 100 重复大文件)</span>
          <div class="treemap-controls">
            <el-button-group>
              <el-button size="small" :icon="ZoomIn" @click="zoomIn">放大</el-button>
              <el-button size="small" :icon="ZoomOut" @click="zoomOut">缩小</el-button>
              <el-button size="small" :icon="Aim" @click="resetZoom">重置聚焦</el-button>
            </el-button-group>
          </div>
        </div>
      </template>

      <!-- 面包屑导航 -->
      <div class="treemap-breadcrumbs-bar" v-if="currentZoomPath">
        <el-tag 
          class="breadcrumb-pill clickable" 
          type="info" 
          effect="plain"
          size="small" 
          @click="resetZoomPath"
        >
          🌐 {{ t('zh-CN' === getLanguage() ? '全局磁盘' : 'Global') }}
        </el-tag>
        <span class="breadcrumb-separator">/</span>
        <template v-for="(segment, idx) in pathSegments" :key="idx">
          <el-tag 
            class="breadcrumb-pill clickable" 
            :type="idx === pathSegments.length - 1 ? 'primary' : 'info'" 
            :effect="idx === pathSegments.length - 1 ? 'light' : 'plain'"
            size="small"
            @click="navigateToSegment(idx)"
          >
            {{ segment }}
          </el-tag>
          <span class="breadcrumb-separator" v-if="idx < pathSegments.length - 1">/</span>
        </template>
      </div>

      <!-- HTML5 Canvas 绘制区域 -->
      <div class="canvas-wrapper-box">
        <canvas 
          ref="treemapCanvasRef" 
          class="treemap-canvas-element"
          @mousedown="handleCanvasMouseDown"
          @mousemove="handleCanvasMouseMove"
          @mouseup="handleCanvasMouseUp"
          @mouseleave="handleCanvasMouseUp"
          @wheel="handleCanvasWheel"
        ></canvas>
        
        <!-- 侧悬浮卡片：展示当前选中的文件详情 -->
        <div class="selected-hover-panel" v-if="selectedPreviewFile">
          <div class="panel-header">🎯 {{ t('analysis.hoverPanelTitle') }}</div>
          <div class="panel-body">
            <div class="panel-row"><span class="lbl">{{ t('analysis.fileName') }}:</span> <span class="val" :title="selectedPreviewFile.filenames[0]">{{ selectedPreviewFile.filenames[0] }}</span></div>
            <div class="panel-row"><span class="lbl">{{ t('analysis.fileSize') }}:</span> <span class="val">{{ formatSize(selectedPreviewFile.size) }}</span></div>
            <div class="panel-row"><span class="lbl">{{ t('analysis.wastedSpace') }}:</span> <span class="val font-red">{{ formatSize(selectedPreviewFile.potential_savings) }} ({{ selectedPreviewFile.file_count }} 份重复)</span></div>
            <div class="panel-row"><span class="lbl">{{ t('analysis.mainPath') }}:</span> <span class="val" :title="selectedPreviewFile.locations[0]">{{ selectedPreviewFile.locations[0] }}</span></div>
          </div>
        </div>
      </div>
      
      <div class="treemap-footer-hint">
        {{ t('zh-CN' === getLanguage() ? '* 滚轮可无级缩放，按住鼠标拖拽可平移画布。双击方块可在本地目录树中深度下钻聚焦。' : '* Mouse wheel to zoom, drag to pan. Double click block to drill-down directory.') }}
      </div>
    </el-card>

    <!-- 下部双卡片图表 -->
    <div class="charts-row" v-if="report">
      <el-card class="chart-card glass-card" shadow="never">
        <template #header>
          <div class="chart-card-header">
            <span>文件类型统计 (前 8 类)</span>
          </div>
        </template>
        <div class="chart-container">
          <canvas ref="typePieChartRef"></canvas>
        </div>
      </el-card>

      <el-card class="chart-card glass-card" shadow="never">
        <template #header>
          <div class="chart-card-header">
            <span>文件大小统计区间</span>
          </div>
        </template>
        <div class="chart-container">
          <canvas ref="sizeBarChartRef"></canvas>
        </div>
      </el-card>
    </div>

    <!-- 底部：详细分析表格与洞察 -->
    <div class="detail-row" v-if="report">
      <el-card class="detail-card glass-card" shadow="never">
        <template #header>
          <div class="detail-header">
            <span>大重复文件 TOP10 列表</span>
          </div>
        </template>

        <el-table :data="report.by_size.large_duplicate_files.slice(0, 10)" stripe>
          <el-table-column type="index" width="50" />
          <el-table-column label="文件名" min-width="180" show-overflow-tooltip>
            <template #default="{ row }">
              {{ row.filenames[0] || '未知文件' }}
            </template>
          </el-table-column>
          <el-table-column label="文件大小" width="120" sortable>
            <template #default="{ row }">
              <span style="font-weight: 600; color: #F56C6C;">
                {{ formatSize(row.size) }}
              </span>
            </template>
          </el-table-column>
          <el-table-column label="重复份数" prop="file_count" width="100">
            <template #default="{ row }">
              <el-tag type="danger" size="small">{{ row.file_count }} 份</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="可节省空间" width="120">
            <template #default="{ row }">
              {{ formatSize(row.potential_savings) }}
            </template>
          </el-table-column>
          <el-table-column label="位置" min-width="200" show-overflow-tooltip>
            <template #default="{ row }">
              {{ row.locations[0] }}
            </template>
          </el-table-column>
          <el-table-column label="操作" width="120">
            <template #default="{ row }">
              <el-button-group>
                <el-button size="small" type="primary" link @click="openFolder(row.locations[0])">
                  <el-icon><FolderOpened /></el-icon>
                </el-button>
              </el-button-group>
            </template>
          </el-table-column>
        </el-table>
      </el-card>
    </div>

    <!-- 抽屉：快照差分比对分析台 -->
    <el-drawer
      v-model="showCompareDrawer"
      title="⚖️ 历史扫描快照差分比对分析"
      size="55%"
      class="glass-drawer"
    >
      <div class="compare-drawer-content">
        <!-- 节点加载 -->
        <div class="compare-nodes-loader">
          <div class="node-box">
            <span class="node-label">历史节点 A (比对基准)</span>
            <div class="node-status-bar">
              <el-tag size="default" type="info">{{ snapshotAName }}</el-tag>
              <el-button type="primary" size="small" @click="loadSnapshotA">
                <el-icon><DocumentAdd /></el-icon> 载入快照
              </el-button>
            </div>
          </div>
          <div class="node-box">
            <span class="node-label">节点 B (当前最新扫描数据)</span>
            <div class="node-status-bar">
              <el-tag size="default" type="success">当前活跃内存索引</el-tag>
            </div>
          </div>
        </div>

        <!-- 比对结论洞察 -->
        <div class="diff-report-summary" v-if="diffResults">
          <div class="summary-metric">
            <span class="sum-label">去重减少浪费</span>
            <span class="sum-value" :class="{ green: diffResults.spaceChange <= 0, red: diffResults.spaceChange > 0 }">
              {{ diffResults.spaceChange <= 0 ? '-' : '+' }} {{ formatSize(Math.abs(diffResults.spaceChange)) }}
            </span>
          </div>
          <div class="summary-metric">
            <span class="sum-label">成功清理文件</span>
            <span class="sum-value green">{{ diffResults.cleaned.length }} 个</span>
          </div>
          <div class="summary-metric">
            <span class="sum-label">新增重复大文件</span>
            <span class="sum-value red">{{ diffResults.added.length }} 个</span>
          </div>
        </div>

        <!-- 详细差分数据展示 -->
        <div class="diff-lists-wrapper" v-if="diffResults">
          <el-tabs type="border-card" class="diff-tabs glass-tabs">
            <el-tab-pane label="♻️ 已清理去重的文件">
              <el-table :data="diffResults.cleaned" max-height="350px" stripe>
                <el-table-column type="index" width="50" />
                <el-table-column label="已清理文件名" show-overflow-tooltip>
                  <template #default="{ row }">
                    <span style="color: #67C23A; text-decoration: line-through;">{{ row.filenames[0] }}</span>
                  </template>
                </el-table-column>
                <el-table-column label="释放空间" width="120">
                  <template #default="{ row }">
                    {{ formatSize(row.potential_savings) }}
                  </template>
                </el-table-column>
              </el-table>
            </el-tab-pane>

            <el-tab-pane label="🚨 新生成的重复垃圾">
              <el-table :data="diffResults.added" max-height="350px" stripe>
                <el-table-column type="index" width="50" />
                <el-table-column label="新增重复文件名" show-overflow-tooltip>
                  <template #default="{ row }">
                    <span style="color: #F56C6C; font-weight: bold;">{{ row.filenames[0] }}</span>
                  </template>
                </el-table-column>
                <el-table-column label="新增浪费" width="120">
                  <template #default="{ row }">
                    {{ formatSize(row.potential_savings) }}
                  </template>
                </el-table-column>
              </el-table>
            </el-tab-pane>
          </el-tabs>
        </div>
        <div class="no-diff-state" v-else>
          <el-empty description="请上传并载入历史快照 JSON，系统将为您自动生成高精度空间变动差分报告" />
        </div>
      </div>
    </el-drawer>
  </div>
</template>

<style scoped>
.analysis-center {
  width: 100%;
  min-width: 100%;
  max-width: 100%;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.analysis-center > * {
  width: 100% !important;
  box-sizing: border-box;
}

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

/* 顶部状态网格 */
.statistics-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 8px;
}

.stat-card {
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.45);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.04);
}

.dark .stat-card {
  border-color: rgba(255, 255, 255, 0.06);
  background: rgba(30, 32, 40, 0.45);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25);
}

.stat-content {
  display: flex;
  align-items: center;
  gap: 14px;
}

.stat-icon {
  width: 42px;
  height: 42px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
}

.stat-icon.info { background: rgba(64, 158, 255, 0.15); color: #409EFF; }
.stat-icon.warning { background: rgba(230, 162, 60, 0.15); color: #E6A23C; }
.stat-icon.danger { background: rgba(245, 108, 108, 0.15); color: #F56C6C; }
.stat-icon.success { background: rgba(103, 194, 94, 0.15); color: #67C23A; }

.stat-info {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
}

.stat-label {
  font-size: 11px;
  color: var(--el-text-color-placeholder);
}

.stat-value {
  font-size: 20px;
  font-weight: 700;
  margin: 2px 0 0 0;
  color: var(--el-text-color-primary);
}

/* Canvas-based Treemap */
.treemap-section-card {
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.45);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border-radius: 12px;
  margin-bottom: 8px;
}

.dark .treemap-section-card {
  border-color: rgba(255, 255, 255, 0.08);
  background: rgba(30, 32, 40, 0.55);
}

.treemap-header-box {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
}

.treemap-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.canvas-wrapper-box {
  width: 100%;
  height: 300px;
  position: relative;
  overflow: hidden;
  border-radius: 8px;
  background: rgba(0,0,0,0.02);
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.dark .canvas-wrapper-box {
  background: rgba(0,0,0,0.25);
  border-color: rgba(255, 255, 255, 0.05);
}

.treemap-canvas-element {
  width: 100%;
  height: 100%;
  display: block;
  cursor: grab;
}

.treemap-canvas-element:active {
  cursor: grabbing;
}

/* 选中高亮悬浮面板 */
.selected-hover-panel {
  position: absolute;
  top: 12px;
  right: 12px;
  width: 250px;
  background: rgba(255, 255, 255, 0.85);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 8px;
  padding: 10px;
  box-shadow: 0 6px 20px rgba(0,0,0,0.08);
  pointer-events: none; /* 穿透鼠标 */
  text-align: left;
}

.dark .selected-hover-panel {
  background: rgba(30, 32, 40, 0.85);
  border-color: rgba(255, 255, 255, 0.08);
  box-shadow: 0 6px 20px rgba(0,0,0,0.3);
}

.panel-header {
  font-size: 12px;
  font-weight: bold;
  color: var(--el-color-primary);
  margin-bottom: 6px;
  border-bottom: 1px dashed rgba(0,0,0,0.08);
  padding-bottom: 4px;
}

.dark .panel-header {
  border-bottom-color: rgba(255,255,255,0.08);
}

.panel-row {
  display: flex;
  font-size: 11px;
  margin-bottom: 4px;
  gap: 6px;
}

.panel-row .lbl {
  color: var(--el-text-color-placeholder);
  flex-shrink: 0;
}

.panel-row .val {
  color: var(--el-text-color-primary);
  word-break: break-all;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.font-red {
  color: #f56c6c !important;
  font-weight: bold;
}

.treemap-footer-hint {
  font-size: 11px;
  color: var(--el-text-color-placeholder);
  margin-top: 8px;
  text-align: left;
  font-style: italic;
}

/* 图表与表格 */
.charts-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 8px;
}

.chart-card {
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.45);
  backdrop-filter: blur(20px);
}

.dark .chart-card {
  border-color: rgba(255, 255, 255, 0.08);
  background: rgba(30, 32, 40, 0.55);
}

.chart-card-header {
  font-size: 14px;
  font-weight: 600;
  text-align: left;
  color: var(--el-text-color-primary);
}

.chart-container {
  height: 220px;
  position: relative;
}

.detail-card {
  border-radius: 12px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.45);
}

.dark .detail-card {
  border-color: rgba(255, 255, 255, 0.08);
  background: rgba(30, 32, 40, 0.55);
}

.detail-header {
  font-size: 14px;
  font-weight: 600;
  text-align: left;
}

/* 抽屉内差分比对台 */
.compare-drawer-content {
  display: flex;
  flex-direction: column;
  gap: 18px;
  height: 100%;
}

.compare-nodes-loader {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
  background: rgba(0,0,0,0.03);
  padding: 12px;
  border-radius: 10px;
}

.dark .compare-nodes-loader {
  background: rgba(255, 255, 255, 0.02);
}

.node-box {
  display: flex;
  flex-direction: column;
  gap: 6px;
  text-align: left;
}

.node-label {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.node-status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.diff-report-summary {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 14px;
}

.summary-metric {
  background: rgba(255,255,255,0.4);
  border: 1px solid rgba(0,0,0,0.04);
  border-radius: 8px;
  padding: 10px;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.dark .summary-metric {
  background: rgba(255,255,255,0.02);
  border-color: rgba(255,255,255,0.05);
}

.sum-label {
  font-size: 11px;
  color: var(--el-text-color-secondary);
}

.sum-value {
  font-size: 18px;
  font-weight: bold;
  margin-top: 4px;
}

.sum-value.green { color: #67C23A; }
.sum-value.red { color: #F56C6C; }

.diff-lists-wrapper {
  flex: 1;
  overflow: hidden;
}

.diff-tabs {
  height: 100%;
  border-radius: 8px;
}

.no-diff-state {
  margin-top: 40px;
}

.treemap-breadcrumbs-bar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  padding: 10px 14px;
  background: var(--el-fill-color-light);
  border: 1px solid var(--el-border-color-light);
  border-radius: 8px;
  margin-bottom: 12px;
  backdrop-filter: blur(10px);
  text-align: left;
}

.breadcrumb-pill.clickable {
  cursor: pointer;
  transition: all 0.2s ease;
}

.breadcrumb-pill.clickable:hover {
  transform: translateY(-1px);
  filter: brightness(1.05);
}

.breadcrumb-separator {
  color: var(--el-text-color-placeholder);
  font-size: 12px;
  user-select: none;
}
</style>
