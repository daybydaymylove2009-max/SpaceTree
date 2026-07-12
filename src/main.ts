import { createApp } from 'vue'
import { createPinia } from 'pinia'
import './style.css'
import './styles/theme.css'
import App from './App.vue'

// 创建 Pinia 实例
const pinia = createPinia()

// Element Plus 中文语言包
// @ts-ignore
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'

// 按需导入 Element Plus 组件
import {
  ElButton,
  ElCard,
  ElCollapse,
  ElCollapseItem,
  ElDescriptions,
  ElDescriptionsItem,
  ElDialog,
  ElDrawer,
  ElEmpty,
  ElForm,
  ElFormItem,
  ElIcon,
  ElInput,
  ElInputNumber,
  ElMenu,
  ElMenuItem,
  ElMessage,
  ElMessageBox,
  ElProgress,
  ElRadio,
  ElRadioGroup,
  ElRadioButton,
  ElCollapseTransition,
  ElButtonGroup,
  ElSelect,
  ElOption,
  ElSlider,
  ElStatistic,
  ElSwitch,
  ElTable,
  ElTableColumn,
  ElTabs,
  ElTabPane,
  ElTag,
  ElTooltip,
  ElBadge,
  ElDivider,
  ElAlert,
  ElCheckbox,
  ElCheckboxGroup,
  ElUpload,
  ElDatePicker,
  ElTimePicker,
  ElPopover,
  ElDropdown,
  ElDropdownItem,
  ElDropdownMenu,
  ElBreadcrumb,
  ElBreadcrumbItem,
  ElPagination,
  ElSteps,
  ElStep,
  ElTimeline,
  ElTimelineItem,
  ElTree,
  ElColorPicker,
  ElTransfer,
  ElCarousel,
  ElCarouselItem,
  ElCalendar,
  ElImage,
  ElAvatar,
  ElSkeleton,
  ElSkeletonItem,
  ElResult,
  ElPageHeader,
  ElBacktop,
  ElInfiniteScroll,
  ElLoading,
  ElNotification,
} from 'element-plus'

import 'element-plus/dist/index.css'

// 按需导入图标
import {
  CircleCheck,
  Folder,
  Search,
  Document,
  DataAnalysis,
  Tools,
  Setting,
  Fold,
  Expand,
  Plus,
  Delete,
  Refresh,
  Loading,
  VideoPause,
  VideoPlay,
  Clock,
  Check,
  Warning,
  FolderOpened,
  Close,
  FullScreen,
  Filter,
  InfoFilled,
  Brush,
  Download,
  Bell,
  ArrowRight,
  ArrowLeft,
  ArrowUp,
  ArrowDown,
  More,
  MoreFilled,
  Menu,
  HomeFilled,
} from '@element-plus/icons-vue'

const app = createApp(App)

// 注册 Element Plus 组件
const components = [
  ElButton,
  ElCard,
  ElCollapse,
  ElCollapseItem,
  ElDescriptions,
  ElDescriptionsItem,
  ElDialog,
  ElDrawer,
  ElEmpty,
  ElForm,
  ElFormItem,
  ElIcon,
  ElInput,
  ElInputNumber,
  ElMenu,
  ElMenuItem,
  ElProgress,
  ElRadio,
  ElRadioGroup,
  ElRadioButton,
  ElCollapseTransition,
  ElButtonGroup,
  ElSelect,
  ElOption,
  ElSlider,
  ElStatistic,
  ElSwitch,
  ElTable,
  ElTableColumn,
  ElTabs,
  ElTabPane,
  ElTag,
  ElTooltip,
  ElBadge,
  ElDivider,
  ElAlert,
  ElCheckbox,
  ElCheckboxGroup,
  ElUpload,
  ElDatePicker,
  ElTimePicker,
  ElPopover,
  ElDropdown,
  ElDropdownItem,
  ElDropdownMenu,
  ElBreadcrumb,
  ElBreadcrumbItem,
  ElPagination,
  ElSteps,
  ElStep,
  ElTimeline,
  ElTimelineItem,
  ElTree,
  ElColorPicker,
  ElTransfer,
  ElCarousel,
  ElCarouselItem,
  ElCalendar,
  ElImage,
  ElAvatar,
  ElSkeleton,
  ElSkeletonItem,
  ElResult,
  ElPageHeader,
  ElBacktop,
]

components.forEach(component => {
  if (component.name) {
    app.component(component.name, component)
  }
})

// 注册指令
app.directive('loading', ElLoading.directive)
app.directive('infinite-scroll', ElInfiniteScroll)

// 注册图标组件
const icons = {
  CircleCheck,
  Folder,
  Search,
  Document,
  DataAnalysis,
  Tools,
  Setting,
  Fold,
  Expand,
  Plus,
  Delete,
  Refresh,
  Loading,
  VideoPause,
  VideoPlay,
  Clock,
  Check,
  Warning,
  FolderOpened,
  Close,
  FullScreen,
  Filter,
  InfoFilled,
  Brush,
  Download,
  Bell,
  ArrowRight,
  ArrowLeft,
  ArrowUp,
  ArrowDown,
  More,
  MoreFilled,
  Menu,
  HomeFilled,
}

Object.entries(icons).forEach(([key, component]) => {
  app.component(key, component)
})

// 挂载属性
app.config.globalProperties.$message = ElMessage
app.config.globalProperties.$messageBox = ElMessageBox
app.config.globalProperties.$notification = ElNotification
app.config.globalProperties.$loading = ElLoading.service

// 配置 Element Plus 中文语言
app.config.globalProperties.$ELEMENT = { locale: zhCn }

// 使用 Pinia
app.use(pinia)

app.mount('#app')
