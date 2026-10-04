<script setup lang="ts">
import { useEventSubscriptions } from '../../composables/useEventSubscriptions'
import { onMounted, ref } from 'vue'
import { commands, events, type Comic } from '../../bindings.ts'
import { open } from '@tauri-apps/plugin-dialog'
import { PhFolderOpen, PhGearSix } from '@phosphor-icons/vue'
import { useStore } from '../../store.ts'
import SettingsDialog from '../../dialogs/SettingsDialog.vue'
import UncompletedProgresses from './components/UncompletedProgresses.vue'
import CompletedProgresses from './components/CompletedProgresses.vue'
import { ProgressData } from '../../types.ts'
import ExportProgresses from './components/ExportProgresses.vue'
import { useIsMobile } from '../../composables/useIsMobile'

export type ProgressesPaneTabName = 'uncompleted' | 'completed' | 'export'

const store = useStore()
const subscribe = useEventSubscriptions()
const isMobile = useIsMobile()

const settingsDialogShowing = ref<boolean>(false)

const downloadSpeed = ref<string>('')

onMounted(async () => {
  await subscribe(
    events.downloadSpeedEvent.listen(async ({ payload: { speed } }) => {
      downloadSpeed.value = speed
    }),
  )

  await subscribe(
    events.downloadSleepingEvent.listen(async ({ payload: { id, remainingSec } }) => {
      const progressData = store.progresses.get(id)
      if (progressData !== undefined) {
        progressData.indicator = `将在${remainingSec}秒后继续下载`
      }
    }),
  )

  await subscribe(
    events.downloadTaskEvent.listen(async ({ payload: { event, data } }) => {
      if (event === 'Create') {
        const { chapterInfo, downloadedImgCount, totalImgCount } = data

        store.progresses.set(chapterInfo.chapterId, {
          ...data,
          percentage: totalImgCount ? Math.min(100, (downloadedImgCount / totalImgCount) * 100) : 0,
          indicator: `${data.state === 'Downloading' ? '下载中' : '排队中'} ${downloadedImgCount}/${totalImgCount}`,
        })
      } else if (event === 'Update') {
        const { chapterId, state, downloadedImgCount, totalImgCount } = data

        const progressData = store.progresses.get(chapterId)
        if (progressData === undefined) {
          return
        }

        progressData.state = state
        progressData.downloadedImgCount = downloadedImgCount
        progressData.totalImgCount = totalImgCount

        if (state === 'Completed') {
          progressData.chapterInfo.isDownloaded = true
          progressData.chapterInfo.pageCount = totalImgCount
          // Completed is emitted only after the files and completion marker are saved.
          // Apply it synchronously before any background refresh can yield to a click.
          applyCompletedChapters(progressData.comic)
          if (store.pickedComic) applyCompletedChapters(store.pickedComic)
        }

        progressData.percentage = totalImgCount ? Math.min(100, (downloadedImgCount / totalImgCount) * 100) : 0

        let indicator = ''
        if (state === 'Pending') {
          indicator = `排队中`
        } else if (state === 'Downloading') {
          indicator = `下载中`
        } else if (state === 'Paused') {
          indicator = `已暂停`
        } else if (state === 'Cancelled') {
          indicator = `已取消`
        } else if (state === 'Completed') {
          indicator = `下载完成`
        } else if (state === 'Failed') {
          indicator = `下载失败`
        }
        if (totalImgCount !== 0) {
          indicator += ` ${downloadedImgCount}/${totalImgCount}`
        }

        progressData.indicator = indicator

        if (state === 'Completed') {
          // Slow or failed list refreshes must not delay the completed indicator.
          const refreshes = await Promise.allSettled([
            syncPickedComic(progressData),
            syncComicInSearch(progressData),
            syncComicInFavorite(progressData),
            syncComicInWeekly(progressData),
          ])
          for (const refresh of refreshes) {
            if (refresh.status === 'rejected') console.error('刷新已完成下载的状态失败', refresh.reason)
          }
        }
      }
    }),
  )
})

function applyCompletedChapters(comic: Comic): Comic {
  for (const progress of store.progresses.values()) {
    if (progress.state !== 'Completed' || progress.comic.id !== comic.id) continue
    const chapter = comic.chapterInfos.find((chapter) => chapter.chapterId === progress.chapterInfo.chapterId)
    if (!chapter) continue
    Object.assign(chapter, progress.chapterInfo, { isDownloaded: true, pageCount: progress.totalImgCount })
    comic.isDownloaded = true
    if (progress.comic.comicDownloadDir) comic.comicDownloadDir = progress.comic.comicDownloadDir
  }
  return comic
}

async function syncPickedComic(progressData: ProgressData) {
  const comic = store.pickedComic
  if (comic === undefined || comic.id !== progressData.comic.id) {
    return
  }
  const result = await commands.getSyncedComic(comic)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  // The user may have opened another comic while this request was in flight.
  if (store.pickedComic === comic) store.pickedComic = applyCompletedChapters(result.data)
}

async function syncComicInSearch(progressData: ProgressData) {
  if (store.searchResult === undefined) {
    return
  }
  const comic = store.searchResult.content.find((comic) => comic.id === progressData.comic.id)
  if (comic === undefined) {
    return
  }
  const result = await commands.getSyncedComicInSearch(comic)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  Object.assign(comic, { ...result.data })
}

async function syncComicInFavorite(progressData: ProgressData) {
  if (store.getFavoriteResult === undefined) {
    return
  }
  const comic = store.getFavoriteResult.list.find((comic) => comic.id === progressData.comic.id)
  if (comic === undefined) {
    return
  }
  const result = await commands.getSyncedComicInFavorite(comic)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  Object.assign(comic, { ...result.data })
}

async function syncComicInWeekly(progressData: ProgressData) {
  if (store.getWeeklyResult === undefined) {
    return
  }
  const comic = store.getWeeklyResult.list.find((comic) => comic.id === progressData.comic.id)
  if (comic === undefined) {
    return
  }
  const result = await commands.getSyncedComicInWeekly(comic)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  Object.assign(comic, { ...result.data })
}

async function showDownloadDirInFileManager() {
  if (store.config === undefined) {
    return
  }
  const result = await commands.showPathInFileManager(store.config.downloadDir)
  if (result.status === 'error') {
    console.error(result.error)
  }
}

async function selectDownloadDir() {
  if (store.config === undefined) {
    return
  }

  const selectedDirPath = await open({ directory: true })
  if (selectedDirPath === null) {
    return
  }

  store.config.downloadDir = selectedDirPath
}
</script>

<template>
  <div v-if="store.config !== undefined" class="flex flex-col flex-1 min-h-0 overflow-hidden">
    <div class="pane-toolbar flex gap-2 box-border px-4 pt-3">
      <n-input-group v-if="store.runtimePlatform !== 'ios'" class="min-w-0 flex-1">
        <n-input-group-label size="small">下载目录</n-input-group-label>
        <n-input v-model:value="store.config.downloadDir" size="small" readonly @click="selectDownloadDir" />
        <n-button v-if="!isMobile" class="w-10" size="small" @click="showDownloadDirInFileManager">
          <template #icon>
            <n-icon size="20">
              <PhFolderOpen />
            </n-icon>
          </template>
        </n-button>
      </n-input-group>
      <n-button size="small" @click="settingsDialogShowing = true">
        <template #icon>
          <n-icon size="20">
            <PhGearSix />
          </n-icon>
        </template>
        配置
      </n-button>
    </div>
    <n-tabs class="flex-1 min-h-0 overflow-hidden" v-model:value="store.progressesPaneTabName" type="line" size="small">
      <n-tab-pane class="h-full p-0! overflow-auto" name="uncompleted" tab="未完成">
        <UncompletedProgresses />
      </n-tab-pane>
      <n-tab-pane class="h-full p-0! overflow-auto" name="completed" tab="已完成">
        <CompletedProgresses />
      </n-tab-pane>
      <n-tab-pane class="h-full p-0! overflow-auto" name="export" tab="导出进度" display-directive="show">
        <ExportProgresses />
      </n-tab-pane>

      <template #suffix>
        <span class="whitespace-nowrap text-ellipsis overflow-hidden shrink-0">{{ downloadSpeed }}</span>
      </template>
    </n-tabs>
    <SettingsDialog v-model:showing="settingsDialogShowing" />
  </div>
</template>
