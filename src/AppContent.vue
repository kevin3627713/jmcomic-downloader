<script setup lang="tsx">
import { onMounted, ref, watch } from 'vue'
import { commands } from './bindings.ts'
import { useMessage, useNotification } from 'naive-ui'
import LoginDialog from './dialogs/LoginDialog.vue'
import SearchPane from './panes/SearchPane.vue'
import ChapterPane from './panes/ChapterPane.vue'
import ProgressesPane from './panes/ProgressesPane/ProgressesPane.vue'
import FavoritePane from './panes/FavoritePane.vue'
import AboutDialog from './dialogs/AboutDialog.vue'
import {
  PhInfo,
  PhUser,
  PhClockCounterClockwise,
  PhMagnifyingGlass,
  PhStar,
  PhCalendar,
  PhHardDrive,
  PhBookOpen,
  PhArrowDown,
} from '@phosphor-icons/vue'
import DownloadedPane from './panes/DownloadedPane/DownloadedPane.vue'
import { useStore } from './store.ts'
import LogDialog from './dialogs/LogDialog.vue'
import WeeklyPane from './panes/WeeklyPane.vue'

const store = useStore()

const message = useMessage()
const notification = useNotification()

const loginDialogShowing = ref<boolean>(false)
const aboutDialogShowing = ref<boolean>(false)
const logViewerShowing = ref<boolean>(false)

// --- Responsive layout ---
const isDesktop = ref(window.matchMedia('(min-width: 768px)').matches)

type MobileTab = 'search' | 'favorite' | 'weekly' | 'downloaded' | 'chapter' | 'progresses'
const mobileTab = ref<MobileTab>('search')

const mobileTabs: { value: MobileTab; label: string; icon: any }[] = [
  { value: 'search', label: '搜索', icon: PhMagnifyingGlass },
  { value: 'favorite', label: '收藏夹', icon: PhStar },
  { value: 'weekly', label: '每周必看', icon: PhCalendar },
  { value: 'downloaded', label: '本地库存', icon: PhHardDrive },
  { value: 'chapter', label: '章节详情', icon: PhBookOpen },
  { value: 'progresses', label: '下载', icon: PhArrowDown },
]

watch(
  () => store.currentTabName,
  (tab) => {
    if (!isDesktop.value) {
      mobileTab.value = tab
    }
  },
)

watch(mobileTab, (tab) => {
  if (tab !== 'progresses') {
    store.currentTabName = tab as 'search' | 'favorite' | 'weekly' | 'downloaded' | 'chapter'
  }
})

watch(
  () => store.config,
  async () => {
    if (store.config === undefined) {
      return
    }

    const result = await commands.saveConfig(store.config)
    if (result.status === 'error') {
      console.error(result.error)
      return
    }
    message.success('保存配置成功')
  },
  { deep: true },
)

onMounted(async () => {
  // 屏蔽浏览器右键菜单
  document.oncontextmenu = (event) => {
    event.preventDefault()
  }
  // 获取配置
  store.config = await commands.getConfig()
  // 如果username和password不为空，尝试登录
  if (store.config.username !== '' && store.config.password !== '') {
    const result = await commands.login(store.config.username, store.config.password)
    if (result.status === 'error') {
      console.error(result.error)
      return
    }
    store.userProfile = result.data
    message.success('自动登录成功')
  }
})

onMounted(async () => {
  // 检查日志目录大小
  const result = await commands.getLogsDirSize()
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  if (result.data > 50 * 1024 * 1024) {
    notification.warning({
      title: '日志目录大小超过50MB，请及时清理日志文件',
      description: () => (
        <>
          <div>
            点击右上角的 <span class="bg-gray-2 px-1">日志</span> 按钮
          </div>
          <div>
            里边有 <span class="bg-gray-2 px-1">打开日志目录</span> 按钮
          </div>
          <div>
            你也可以在里边取消勾选 <span class="bg-gray-2 px-1">输出文件日志</span>
          </div>
          <div>这样将不再产生文件日志</div>
        </>
      ),
    })
  }
})

onMounted(() => {
  const mediaQuery = window.matchMedia('(min-width: 768px)')
  isDesktop.value = mediaQuery.matches
  if (!isDesktop.value) {
    mobileTab.value = store.currentTabName
  }
  mediaQuery.addEventListener('change', (e) => {
    isDesktop.value = e.matches
    if (!isDesktop.value) {
      mobileTab.value = store.currentTabName
    }
  })
})
</script>

<template>
  <div v-if="store.config !== undefined" class="h-[100dvh] flex flex-col overflow-hidden">
    <!-- Mobile top bar: action buttons + avatar -->
    <div v-if="!isDesktop" class="flex items-center gap-1 px-2 py-1.5 shrink-0 border-b border-gray-200">
      <n-button type="primary" @click="loginDialogShowing = true" size="small">
        <template #icon>
          <n-icon><PhUser /></n-icon>
        </template>
        登录
      </n-button>
      <n-button @click="logViewerShowing = true" size="small">
        <template #icon>
          <n-icon size="20"><PhClockCounterClockwise /></n-icon>
        </template>
        日志
      </n-button>
      <n-button @click="aboutDialogShowing = true" size="small">
        <template #icon>
          <n-icon size="20"><PhInfo /></n-icon>
        </template>
        关于
      </n-button>
      <div v-if="store.userProfile !== undefined" class="flex items-center ml-auto overflow-hidden">
        <n-avatar
          class="flex-shrink-0"
          round
          :size="32"
          :src="store.userProfile.photo"
          fallback-src="https://cdn-msp.jmapiproxy2.cc/templates/frontend/airav/img/title-png/more-ms-jm.webp?v=2" />
        <span class="whitespace-nowrap text-ellipsis overflow-hidden" :title="store.userProfile.username">
          {{ store.userProfile.username }}
        </span>
      </div>
    </div>

    <!-- Desktop layout: two columns (UNCHANGED from original) -->
    <div v-if="isDesktop" class="flex-1 min-h-0 flex overflow-hidden">
      <n-tabs class="h-full w-1/2" v-model:value="store.currentTabName" type="line" size="small" animated>
        <n-tab-pane class="h-full overflow-auto p-0!" name="search" tab="搜索" display-directive="show">
          <SearchPane />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="favorite" tab="收藏夹" display-directive="show">
          <FavoritePane />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="weekly" tab="每周必看" display-directive="show">
          <WeeklyPane />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="downloaded" tab="本地库存" display-directive="show">
          <DownloadedPane />
        </n-tab-pane>
        <n-tab-pane class="h-full overflow-auto p-0!" name="chapter" tab="章节详情" display-directive="show">
          <ChapterPane />
        </n-tab-pane>
      </n-tabs>
      <div class="w-1/2 overflow-auto flex flex-col">
        <div class="flex px-2 gap-1">
          <n-button type="primary" @click="loginDialogShowing = true">
            <template #icon>
              <n-icon>
                <PhUser />
              </n-icon>
            </template>
            登录
          </n-button>
          <n-button @click="logViewerShowing = true">
            <template #icon>
              <n-icon size="20">
                <PhClockCounterClockwise />
              </n-icon>
            </template>
            日志
          </n-button>
          <n-button @click="aboutDialogShowing = true">
            <template #icon>
              <n-icon size="20">
                <PhInfo />
              </n-icon>
            </template>
            关于
          </n-button>
          <div v-if="store.userProfile !== undefined" class="flex items-center ml-auto overflow-hidden">
            <n-avatar
              class="flex-shrink-0"
              round
              :size="32"
              :src="store.userProfile.photo"
          fallback-src="https://cdn-msp.jmapiproxy2.cc/templates/frontend/airav/img/title-png/more-ms-jm.webp?v=2" />
            <span class="whitespace-nowrap text-ellipsis overflow-hidden" :title="store.userProfile.username">
              {{ store.userProfile.username }}
            </span>
          </div>
        </div>
        <ProgressesPane />
      </div>
    </div>

    <!-- Mobile layout: single column + bottom tab bar -->
    <div v-else class="flex-1 min-h-0 flex flex-col overflow-hidden">
      <div class="flex-1 min-h-0 overflow-auto">
        <div v-show="mobileTab === 'search'" class="h-full flex flex-col">
          <SearchPane />
        </div>
        <div v-show="mobileTab === 'favorite'" class="h-full flex flex-col">
          <FavoritePane />
        </div>
        <div v-show="mobileTab === 'weekly'" class="h-full flex flex-col">
          <WeeklyPane />
        </div>
        <div v-show="mobileTab === 'downloaded'" class="h-full flex flex-col">
          <DownloadedPane />
        </div>
        <div v-show="mobileTab === 'chapter'" class="h-full flex flex-col">
          <ChapterPane />
        </div>
        <div v-show="mobileTab === 'progresses'" class="h-full flex flex-col">
          <ProgressesPane />
        </div>
      </div>
      <div class="flex items-stretch justify-around border-t border-gray-200 bg-white shrink-0">
        <button
          v-for="tab in mobileTabs"
          :key="tab.value"
          class="flex flex-col items-center justify-center flex-1 py-1.5 gap-0.5 text-xs transition-colors duration-200 cursor-pointer border-none bg-transparent"
          :class="mobileTab === tab.value ? 'text-blue-500' : 'text-gray-500'"
          @click="mobileTab = tab.value">
          <n-icon :size="20"><component :is="tab.icon" /></n-icon>
          <span class="leading-tight">{{ tab.label }}</span>
        </button>
      </div>
    </div>

    <!-- Dialogs -->
    <LoginDialog v-model:showing="loginDialogShowing" />
    <AboutDialog v-model:showing="aboutDialogShowing" />
    <LogDialog v-model:showing="logViewerShowing" />
  </div>
</template>

<style scoped>
:global(.n-notification-main__header) {
  @apply break-words;
}

:global(.n-tabs-pane-wrapper) {
  @apply h-full;
}

:deep(.n-tabs-nav) {
  @apply px-2;
}
</style>
