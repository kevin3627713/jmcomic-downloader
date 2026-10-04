<script setup lang="ts">
import { SelectionArea, SelectionEvent } from '@viselect/vue'
import { computed, nextTick, ref, watch, watchEffect } from 'vue'
import { ChapterInfo, commands, DownloadTaskState } from '../bindings.ts'
import { useStore } from '../store.ts'
import { PhFolderOpen } from '@phosphor-icons/vue'
import IconButton from '../components/IconButton.vue'
import { useComicExport } from '../composables/useComicExport'
import { useIsMobile } from '../composables/useIsMobile'
import { useMessage } from 'naive-ui'

const store = useStore()
const isMobile = useIsMobile()
const message = useMessage()

const { exportComic, openPdf } = useComicExport()
const scheduling = ref(false)

const dropdownX = ref<number>(0)
const dropdownY = ref<number>(0)
const showDropdown = ref<boolean>(false)
const dropdownOptions = [
  {
    label: '勾选',
    key: 'check',
    props: {
      onClick: () => {
        // 只有未勾选的才会被勾选
        ;[...selectedIds.value]
          .filter((id) => !checkedIds.value.includes(id))
          .forEach((id) => checkedIds.value.push(id))
        showDropdown.value = false
      },
    },
  },
  {
    label: '取消勾选',
    key: 'uncheck',
    props: {
      onClick: () => {
        checkedIds.value = checkedIds.value.filter((id) => !selectedIds.value.has(id))
        showDropdown.value = false
      },
    },
  },
  {
    label: '全选',
    key: 'check all',
    props: {
      onClick: () => {
        // 只有未锁定的才会被勾选
        store.pickedComic?.chapterInfos
          ?.filter((c) => c.isDownloaded !== true && !checkedIds.value.includes(c.chapterId))
          .forEach((c) => checkedIds.value.push(c.chapterId))
        showDropdown.value = false
      },
    },
  },
  {
    label: '取消全选',
    key: 'uncheck all',
    props: {
      onClick: () => {
        checkedIds.value.length = 0
        showDropdown.value = false
      },
    },
  },
]
const checkedIds = ref<number[]>([])
const selectedIds = ref<Set<number>>(new Set())
const selectionAreaRef = ref<InstanceType<typeof SelectionArea>>()

type State = DownloadTaskState | 'Idle'
const chapterInfos = computed<(ChapterInfo & { state: State })[]>(() => {
  const pickedComic = store.pickedComic

  if (pickedComic === undefined) {
    return []
  }

  return pickedComic.chapterInfos.map((chapterInfo) => {
    const progressData = store.progresses.get(chapterInfo.chapterId)
    if (progressData === undefined) {
      return {
        ...chapterInfo,
        state: 'Idle',
      }
    }
    return {
      ...chapterInfo,
      state: progressData.state,
    }
  })
})

watch(
  () => store.pickedComic?.id,
  () => {
    checkedIds.value = []
    selectedIds.value.clear()
    selectionAreaRef.value?.selection?.clearSelection()
  },
)

watchEffect(() => {
  if (store.pickedComic === undefined) {
    return
  }
  // 只保留未下载的章节
  // TODO: 改用set效率更高
  const notDownloadedChapterIds = chapterInfos.value
    .filter((c) => c.isDownloaded !== true && !isDownloading(c.state))
    .map((c) => c.chapterId)
  checkedIds.value = checkedIds.value.filter((id) => notDownloadedChapterIds.includes(id))
})

function extractIds(elements: Element[]): number[] {
  return elements
    .map((element) => element.getAttribute('data-key'))
    .filter(Boolean)
    .map(Number)
    .filter((id) => {
      const chapterInfo = store.pickedComic?.chapterInfos.find((c) => c.chapterId === id)
      if (chapterInfo === undefined) {
        return false
      }
      return chapterInfo.isDownloaded !== true
    })
}

function unselectAll({ event, selection }: SelectionEvent) {
  if (!event?.ctrlKey && !event?.metaKey) {
    selection.clearSelection()
    selectedIds.value.clear()
  }
}

function updateSelectedIds({
  store: {
    changed: { added, removed },
  },
}: SelectionEvent) {
  extractIds(added).forEach((id) => selectedIds.value.add(id))
  extractIds(removed).forEach((id) => selectedIds.value.delete(id))
}

async function onContextMenu(e: MouseEvent) {
  showDropdown.value = false
  await nextTick()
  showDropdown.value = true
  dropdownX.value = e.clientX
  dropdownY.value = e.clientY
}

const availableIds = computed(() =>
  chapterInfos.value.filter((c) => !c.isDownloaded && !isDownloading(c.state)).map((c) => c.chapterId),
)
const downloadedCount = computed(() => chapterInfos.value.filter((c) => c.isDownloaded).length)
function toggleAll() {
  checkedIds.value = checkedIds.value.length === availableIds.value.length ? [] : [...availableIds.value]
}
async function downloadChapters() {
  const comic = store.pickedComic
  if (!comic || scheduling.value) return
  const ids = checkedIds.value.filter((id) => availableIds.value.includes(id))
  scheduling.value = true
  let queued = 0
  try {
    for (const id of ids) {
      const result = await commands.createDownloadTask(comic, id)
      if (result.status === 'error') message.error(result.error.err_title)
      else queued++
    }
    if (queued) {
      message.success(`已添加 ${queued} 个下载任务`)
      store.progressesPaneTabName = 'uncompleted'
      if (isMobile.value) store.mobileTab = 'progresses'
    }
  } finally {
    scheduling.value = false
  }
}

async function refreshChapters() {
  if (store.pickedComic === undefined) {
    return
  }
  const result = await commands.getComic(store.pickedComic.id)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  store.pickedComic = result.data
  // TODO: 如果pickedComic已下载，则更新元数据
}

async function showComicDownloadDirInFileManager() {
  if (store.pickedComic === undefined) {
    return
  }

  const comicDownloadDir = store.pickedComic.comicDownloadDir
  if (comicDownloadDir === undefined || comicDownloadDir === null) {
    console.error('comicDownloadDir的值为undefined或null')
    return
  }

  const result = await commands.showPathInFileManager(comicDownloadDir)
  if (result.status === 'error') {
    console.error(result.error)
  }
}

function isDownloading(state: State) {
  return state === 'Pending' || state === 'Downloading' || state === 'Paused'
}
</script>

<template>
  <div class="flex-1 min-h-0 flex flex-col box-border">
    <div v-if="!store.pickedComic" class="empty-state">
      <n-empty description="搜索漫画或从书库中选择一本，查看章节" />
    </div>
    <template v-else>
      <div class="comic-summary">
        <img
          :src="`https://${store.currentImageCdnDomain}/media/albums/${store.pickedComic.id}_3x4.jpg`"
          alt="漫画封面"
          referrerpolicy="no-referrer" />
        <div class="comic-summary-text">
          <h2 class="line-clamp-2">{{ store.pickedComic.name }}</h2>
          <p>{{ store.pickedComic.author.join('、') || '未知作者' }} · JM {{ store.pickedComic.id }}</p>
          <p class="line-clamp-2">{{ store.pickedComic.tags.join(' · ') }}</p>
          <p>共 {{ chapterInfos.length }} 章 · 已下载 {{ downloadedCount }} 章</p>
        </div>
      </div>
      <div class="chapter-tools">
        <span>已选 {{ checkedIds.length }} 章</span>
        <n-button :disabled="!availableIds.length" @click="toggleAll">
          {{ checkedIds.length && checkedIds.length === availableIds.length ? '取消全选' : '全选' }}
        </n-button>
        <n-button @click="refreshChapters">刷新</n-button>
      </div>
      <span v-if="!isMobile" class="px-4 text-gray text-xs">左键拖动框选，右键打开菜单</span>
      <component
        :is="isMobile ? 'div' : SelectionArea"
        ref="selectionAreaRef"
        class="selection-container pane-scroll px-4 pb-4"
        :options="{ selectables: '.selectable', features: { deselectOnBlur: true } }"
        @contextmenu="!isMobile && onContextMenu($event)"
        @move="updateSelectedIds"
        @start="unselectAll">
        <n-checkbox-group v-model:value="checkedIds" class="chapter-grid">
          <n-checkbox
            v-for="{ chapterId, chapterTitle, isDownloaded, state } in chapterInfos"
            :key="chapterId"
            :data-key="chapterId"
            class="selectable"
            :value="chapterId"
            :disabled="isDownloaded === true || isDownloading(state)"
            :class="{
              selected: selectedIds.has(chapterId),
              downloaded: isDownloaded,
              downloading: !isDownloaded && isDownloading(state),
            }">
            <span>{{ chapterTitle }}</span>
            <small v-if="isDownloaded" class="block text-green-6">已下载</small>
            <small v-else-if="isDownloading(state)" class="block text-orange-6">
              {{ state === 'Paused' ? '已暂停' : '下载中' }}
            </small>
          </n-checkbox>
        </n-checkbox-group>
      </component>
      <div class="chapter-actions">
        <n-button
          class="download-action"
          type="primary"
          :disabled="!checkedIds.length"
          :loading="scheduling"
          @click="downloadChapters">
          下载所选 {{ checkedIds.length ? `(${checkedIds.length})` : '' }}
        </n-button>
        <n-button v-if="downloadedCount" @click="openPdf(store.pickedComic)">查看 PDF</n-button>
        <div v-if="downloadedCount" class="action-row w-full">
          <n-button
            :loading="store.exportingComics.has(`${store.pickedComic.id}:pdf`)"
            @click="exportComic(store.pickedComic, 'pdf', true)">
            导出 PDF
          </n-button>
          <n-button
            :loading="store.exportingComics.has(`${store.pickedComic.id}:cbz`)"
            @click="exportComic(store.pickedComic, 'cbz', true)">
            导出 CBZ
          </n-button>
          <IconButton v-if="!isMobile" title="打开下载目录" @click="showComicDownloadDirInFileManager">
            <PhFolderOpen :size="24" />
          </IconButton>
        </div>
      </div>
    </template>
    <n-dropdown
      v-if="!isMobile"
      placement="bottom-start"
      trigger="manual"
      :x="dropdownX"
      :y="dropdownY"
      :options="dropdownOptions"
      :show="showDropdown"
      :on-clickoutside="() => (showDropdown = false)" />
  </div>
</template>

<style scoped>
.selection-container {
  @apply select-none overflow-auto;
}

.selection-container .selected {
  @apply bg-[rgb(204,232,255)];
}

.selection-container .downloaded {
  @apply bg-[rgba(24,160,88,0.16)];
}

:deep(.downloaded .n-checkbox__label) {
  color: #507064 !important;
}

.selection-container .downloading {
  @apply bg-[rgba(114,46,209,0.16)];
}

:deep(.n-checkbox__label) {
  overflow-wrap: anywhere;
}

:global(.selection-area) {
  @apply bg-[rgba(46,115,252,0.5)];
}
</style>
