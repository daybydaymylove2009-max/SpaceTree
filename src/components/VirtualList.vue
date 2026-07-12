<script setup lang="ts" generic="T">
/**
 * 高性能虚拟滚动列表组件
 * @description 专为海量数据渲染设计，避免大量 DOM 节点导致页面卡顿，内存占用低且滚动顺滑
 */
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';

const props = withDefaults(defineProps<{
  items: T[];
  itemHeight: number;
  height?: string;
  bufferSize?: number; // 视口上下预渲染数量
}>(), {
  height: '100%',
  bufferSize: 10
});

const emit = defineEmits<{
  (e: 'scroll', event: Event): void;
}>();

const containerRef = ref<HTMLDivElement | null>(null);
const scrollTop = ref(0);
const containerHeight = ref(400); // 默认视口高度

// 监听容器大小变化以自适应高度
let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
  if (containerRef.value) {
    containerHeight.value = containerRef.value.clientHeight;
    scrollTop.value = containerRef.value.scrollTop;

    if (window.ResizeObserver) {
      resizeObserver = new ResizeObserver((entries) => {
        for (let entry of entries) {
          containerHeight.value = entry.contentRect.height;
        }
      });
      resizeObserver.observe(containerRef.value);
    }
  }
});

onUnmounted(() => {
  if (resizeObserver && containerRef.value) {
    resizeObserver.unobserve(containerRef.value);
  }
});

// 重置滚动位置（如在数据源变化时）
watch(() => props.items, () => {
  if (containerRef.value) {
    scrollTop.value = containerRef.value.scrollTop;
  }
}, { deep: false });

// 滚动事件处理
function handleScroll(e: Event) {
  if (containerRef.value) {
    scrollTop.value = containerRef.value.scrollTop;
  }
  emit('scroll', e);
}

// 撑开容器的虚拟总高度
const totalHeight = computed(() => props.items.length * props.itemHeight);

// 计算可见范围的起始和结束索引
const range = computed(() => {
  const start = Math.floor(scrollTop.value / props.itemHeight);
  const visibleCount = Math.ceil(containerHeight.value / props.itemHeight);
  
  const startIdx = Math.max(0, start - props.bufferSize);
  const endIdx = Math.min(props.items.length, start + visibleCount + props.bufferSize);
  
  return {
    start: startIdx,
    end: endIdx
  };
});

// 提取当前需要渲染的数据切片，并加上绝对定位所需的偏移量
const visibleItems = computed(() => {
  const { start, end } = range.value;
  return props.items.slice(start, end).map((item, idx) => ({
    data: item,
    index: start + idx,
    top: (start + idx) * props.itemHeight
  }));
});

// 提供外部直接定位滚动高度的方法
function scrollTo(offset: number) {
  if (containerRef.value) {
    containerRef.value.scrollTop = offset;
    scrollTop.value = offset;
  }
}

// 提供外部定位到指定索引的方法
function scrollToIndex(index: number) {
  scrollTo(index * props.itemHeight);
}

defineExpose({
  scrollTo,
  scrollToIndex,
  scrollTop,
  visibleItems
});
</script>

<template>
  <div 
    ref="containerRef" 
    class="virtual-scroll-container" 
    :style="{ height: height }"
    @scroll="handleScroll"
  >
    <!-- 总高度撑开器，使原生滚动条高度正确 -->
    <div class="virtual-scroll-phantom" :style="{ height: totalHeight + 'px' }"></div>
    
    <!-- 仅渲染可见区域的列表内容容器 -->
    <div class="virtual-scroll-list">
      <div 
        v-for="item in visibleItems" 
        :key="item.index"
        class="virtual-scroll-item" 
        :style="{ 
          height: itemHeight + 'px',
          transform: `translate3d(0, ${item.top}px, 0)`
        }"
      >
        <slot :item="item.data" :index="item.index"></slot>
      </div>
    </div>
    
    <!-- 空状态槽 -->
    <div v-if="items.length === 0" class="virtual-scroll-empty">
      <slot name="empty">
        <div class="default-empty">暂无数据</div>
      </slot>
    </div>
  </div>
</template>

<style scoped>
.virtual-scroll-container {
  overflow-y: auto;
  overflow-x: hidden;
  position: relative;
  width: 100%;
}

.virtual-scroll-phantom {
  position: absolute;
  left: 0;
  top: 0;
  right: 0;
  z-index: -1;
  pointer-events: none;
}

.virtual-scroll-list {
  left: 0;
  right: 0;
  top: 0;
  position: absolute;
  width: 100%;
}

.virtual-scroll-item {
  position: absolute;
  left: 0;
  right: 0;
  width: 100%;
  box-sizing: border-box;
  will-change: transform;
}

.virtual-scroll-empty {
  display: flex;
  justify-content: center;
  align-items: center;
  width: 100%;
  height: 100%;
}

.default-empty {
  color: #909399;
  font-size: 14px;
}
</style>
