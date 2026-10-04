<script setup lang="tsx">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { commands } from './bindings'
import { useMessage, useNotification } from 'naive-ui'
import LoginDialog from './dialogs/LoginDialog.vue'
import AboutDialog from './dialogs/AboutDialog.vue'
import LogDialog from './dialogs/LogDialog.vue'
import SettingsDialog from './dialogs/SettingsDialog.vue'
import SearchPane from './panes/SearchPane.vue'
import ChapterPane from './panes/ChapterPane.vue'
import FavoritePane from './panes/FavoritePane.vue'
import WeeklyPane from './panes/WeeklyPane.vue'
import DownloadedPane from './panes/DownloadedPane/DownloadedPane.vue'
import ProgressesPane from './panes/ProgressesPane/ProgressesPane.vue'
import {
  PhInfo,
  PhUser,
  PhClockCounterClockwise,
  PhMagnifyingGlass,
  PhStar,
  PhHardDrive,
  PhArrowDown,
  PhGearSix,
  PhCaretLeft,
} from '@phosphor-icons/vue'
import { useStore } from './store'
import { useIsMobile } from './composables/useIsMobile'
import { useAppViewport } from './composables/useAppViewport'
import type { CurrentTabName } from './types'

const store = useStore()
const message = useMessage()
const notification = useNotification()
const isMobile = useIsMobile()
useAppViewport()
const isDesktop = computed(() => !isMobile.value)
watch(isMobile, (mobile) => document.documentElement.classList.toggle('mobile-device', mobile), { immediate: true })
const loginDialogShowing = ref(false)
const aboutDialogShowing = ref(false)
const logViewerShowing = ref(false)
const settingsShowing = ref(false)
const startupError = ref('')
const lastBrowseTab = ref<CurrentTabName>('search')
const activeDownloads = computed(
  () => [...store.progresses.values()].filter((p) => ['Pending', 'Downloading', 'Paused'].includes(p.state)).length,
)
const mobileTabs = [
  { value: 'search', label: '发现', icon: PhMagnifyingGlass },
  { value: 'favorite', label: '收藏', icon: PhStar },
  { value: 'downloaded', label: '书库', icon: PhHardDrive },
  { value: 'progresses', label: '任务', icon: PhArrowDown },
] as const
const pageTitle = computed(
  () =>
    ({
      search: '发现漫画',
      weekly: '每周必看',
      favorite: '我的收藏',
      downloaded: '本地书库',
      chapter: '章节详情',
      progresses: '下载与导出',
    })[store.mobileTab],
)
const activeMobileTab = computed(() =>
  store.mobileTab === 'weekly' ? 'search' : store.mobileTab === 'chapter' ? lastBrowseTab.value : store.mobileTab,
)

watch(
  () => store.currentTabName,
  (tab, previous) => {
    if (tab === 'chapter' && previous !== 'chapter') lastBrowseTab.value = previous
    store.mobileTab = tab
  },
)
watch(
  () => store.pickedComic?.id,
  () => {
    if (store.currentTabName === 'chapter') store.mobileTab = 'chapter'
  },
)
function navigate(tab: typeof store.mobileTab) {
  store.mobileTab = tab
  if (tab !== 'progresses') store.currentTabName = tab
}

let saveTimer: ReturnType<typeof setTimeout> | undefined
watch(
  () => store.config,
  (config, previous) => {
    if (!config || !previous) return
    clearTimeout(saveTimer)
    saveTimer = setTimeout(async () => {
      try {
        const result = await commands.saveConfig(config)
        if (result.status === 'error') message.error(result.error.err_message)
      } catch (error) {
        message.error(String(error))
      }
    }, 400)
  },
  { deep: true },
)
onUnmounted(() => clearTimeout(saveTimer))

async function initialize() {
  startupError.value = ''
  try {
    const [config, platform] = await Promise.all([commands.getConfig(), commands.getRuntimePlatform()])
    store.runtimePlatform = platform
    store.config = config
    if (config.username && config.password) {
      const result = await commands.login(config.username, config.password)
      if (result.status === 'ok') {
        store.userProfile = result.data
        message.success('自动登录成功')
      }
    }
    const logs = await commands.getLogsDirSize()
    if (logs.status === 'ok' && logs.data > 50 * 1024 * 1024)
      notification.warning({ title: '日志超过 50 MB，可在日志设置中清理或关闭文件日志', duration: 6000 })
  } catch (error) {
    startupError.value = String(error)
  }
}
onMounted(initialize)
</script>
<template>
  <div v-if="store.config !== undefined" class="app-shell" :class="{ 'mobile-layout': isMobile }">
    <header v-if="isMobile" class="mobile-header">
      <button
        v-if="store.mobileTab === 'chapter'"
        class="header-icon"
        aria-label="返回列表"
        @click="navigate(lastBrowseTab)">
        <PhCaretLeft :size="24" />
      </button>
      <span v-else class="brand-mark">JM</span>
      <div class="header-heading">
        <span class="header-eyebrow">阅读 · 收藏 · 下载</span>
        <strong>{{ pageTitle }}</strong>
      </div>
      <button class="header-icon" aria-label="账号登录" @click="loginDialogShowing = true">
        <PhUser :size="23" />
      </button>
      <n-dropdown
        trigger="click"
        :options="[
          { label: '设置', key: 'settings' },
          { label: '运行日志', key: 'logs' },
          { label: '关于', key: 'about' },
        ]"
        @select="(key: string) => { if (key === 'settings') settingsShowing = true; else if (key === 'logs') logViewerShowing = true; else aboutDialogShowing = true }">
        <button class="header-icon" aria-label="更多选项"><PhGearSix :size="23" /></button>
      </n-dropdown>
    </header>
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

    <div v-else class="mobile-main">
      <div
        v-if="store.mobileTab === 'search' || store.mobileTab === 'weekly'"
        class="discover-switch"
        aria-label="发现分类">
        <button :class="{ active: store.mobileTab === 'search' }" @click="navigate('search')">搜索</button>
        <button :class="{ active: store.mobileTab === 'weekly' }" @click="navigate('weekly')">每周必看</button>
      </div>
      <div v-show="store.mobileTab === 'search'" class="mobile-pane"><SearchPane /></div>
      <div v-show="store.mobileTab === 'weekly'" class="mobile-pane"><WeeklyPane /></div>
      <div v-show="store.mobileTab === 'favorite'" class="mobile-pane"><FavoritePane /></div>
      <div v-show="store.mobileTab === 'downloaded'" class="mobile-pane"><DownloadedPane /></div>
      <div v-show="store.mobileTab === 'chapter'" class="mobile-pane"><ChapterPane /></div>
      <div v-show="store.mobileTab === 'progresses'" class="mobile-pane"><ProgressesPane /></div>
    </div>
    <nav v-if="isMobile" class="mobile-nav" aria-label="主导航">
      <button
        v-for="tab in mobileTabs"
        :key="tab.value"
        :class="{ active: activeMobileTab === tab.value }"
        :aria-current="activeMobileTab === tab.value ? 'page' : undefined"
        @click="navigate(tab.value)">
        <span class="nav-icon">
          <component :is="tab.icon" :size="23" />
          <span v-if="tab.value === 'progresses' && activeDownloads" class="nav-badge">{{ activeDownloads }}</span>
        </span>
        <span>{{ tab.label }}</span>
      </button>
    </nav>
    <LoginDialog v-model:showing="loginDialogShowing" />
    <AboutDialog v-model:showing="aboutDialogShowing" />
    <LogDialog v-model:showing="logViewerShowing" />
    <SettingsDialog v-model:showing="settingsShowing" />
  </div>
  <div v-else class="startup-state">
    <template v-if="startupError">
      <p>启动失败：{{ startupError }}</p>
      <n-button @click="initialize">重试</n-button>
    </template>
    <template v-else>
      <n-spin />
      <p>正在加载…</p>
    </template>
  </div>
</template>
<style scoped>
:global(.n-notification-main__header) {
  overflow-wrap: anywhere;
}
:deep(.n-tabs-nav) {
  padding: 0 8px;
}
</style>
