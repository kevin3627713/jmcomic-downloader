<script setup lang="tsx">
import { Comic, commands } from '../../bindings.ts'
import { computed, nextTick, ref, watch, watchEffect } from 'vue'
import DownloadedComicCard from './components/DownloadedComicCard.vue'
import { open } from '@tauri-apps/plugin-dialog'
import { PhFolderOpen, PhArrowClockwise } from '@phosphor-icons/vue'
import { useStore } from '../../store.ts'
import { DropdownOption, NIcon } from 'naive-ui'
import { SelectionArea, SelectionEvent } from '@viselect/vue'
import { PhChecks, PhCheck, PhX } from '@phosphor-icons/vue'
import UpdateDownloadedComicsButton from './components/UpdateDownloadedComicsButton.vue'
import { useIsMobile } from '../../composables/useIsMobile'
import { useComicExport } from '../../composables/useComicExport'

const store = useStore()
const isMobile = useIsMobile()
const { exportComic } = useComicExport()
const bulkExporting = ref(false)
const managing = ref(false)
const refreshing = ref(false)
const libraryError = ref('')
let refreshRequest = 0

const selectedIds = ref<Set<number>>(new Set())
const checkedIds = ref<Set<number>>(new Set())
const { dropdownX, dropdownY, dropdownShowing, dropdownOptions, showDropdown } = useDropdown()
const selectionAreaRef = ref<InstanceType<typeof SelectionArea>>()

const PAGE_SIZE = 20
// 已下载的漫画
const downloadedComics = ref<Comic[]>([])
// 当前页码
const currentPage = ref<number>(1)
// 总页数
const pageCount = computed<number>(() => {
  if (downloadedComics.value.length === 0) {
    return 1
  }
  return Math.ceil(downloadedComics.value.length / PAGE_SIZE)
})
// 当前页的漫画
const currentPageComics = computed<Comic[]>(() => {
  const start = (currentPage.value - 1) * PAGE_SIZE
  const end = start + PAGE_SIZE
  return downloadedComics.value.slice(start, end)
})
// 确保当前页码不超过总页数
watchEffect(() => {
  if (currentPage.value > pageCount.value) {
    currentPage.value = pageCount.value
  }
})

watch(currentPage, () => {
  selectedIds.value.clear()
  checkedIds.value.clear()
  selectionAreaRef.value?.selection?.clearSelection()
  const area = selectionAreaRef.value
  const element = area instanceof HTMLElement ? area : area?.$el
  element?.scrollTo({ top: 0, behavior: 'instant' })
})

// 监听标签页变化，更新下载的漫画列表
async function refreshLibrary() {
  const request = ++refreshRequest
  refreshing.value = true
  try {
    const result = await commands.getDownloadedComics()
    if (request !== refreshRequest) return
    if (result.status === 'error') {
      libraryError.value = result.error.err_message
      return
    }
    libraryError.value = ''
    const comics = result.data
    downloadedComics.value = comics
    const ids = new Set(comics.map((comic) => comic.id))
    checkedIds.value = new Set([...checkedIds.value].filter((id) => ids.has(id)))
    selectedIds.value = new Set([...selectedIds.value].filter((id) => ids.has(id)))
  } catch (error) {
    if (request === refreshRequest) libraryError.value = String(error)
  } finally {
    if (request === refreshRequest) refreshing.value = false
  }
}
function toggleManaging() {
  managing.value = !managing.value
  if (!managing.value) {
    checkedIds.value.clear()
    selectedIds.value.clear()
  }
}
watch(
  () => store.currentTabName,
  async () => {
    if (store.currentTabName !== 'downloaded') {
      return
    }

    await refreshLibrary()
  },
  { immediate: true },
)

async function selectExportDir() {
  if (store.config === undefined) {
    return
  }

  const selectedDirPath = await open({ directory: true })
  if (selectedDirPath === null) {
    return
  }

  store.config.exportDir = selectedDirPath
}

async function showExportDirInFileManager() {
  if (store.config === undefined) {
    return
  }
  const result = await commands.showPathInFileManager(store.config.exportDir)
  if (result.status === 'error') {
    console.error(result.error)
  }
}

function extractIds(elements: Element[]): number[] {
  return elements
    .map((element) => element.getAttribute('data-key'))
    .filter(Boolean)
    .map(Number)
}

function updateSelectedIds({
  store: {
    changed: { added, removed },
  },
}: SelectionEvent) {
  extractIds(added).forEach((id) => selectedIds.value.add(id))
  extractIds(removed).forEach((id) => selectedIds.value.delete(id))
}

function unselectAll({ event, selection }: SelectionEvent) {
  if (!event?.ctrlKey && !event?.metaKey) {
    selection.clearSelection()
    selectedIds.value.clear()
  }
}

function checkboxChecked(comic: Comic): boolean {
  return checkedIds.value.has(comic.id)
}

function handleCheckboxClick(comic: Comic) {
  if (checkedIds.value.has(comic.id)) {
    checkedIds.value.delete(comic.id)
  } else {
    checkedIds.value.add(comic.id)
  }
}

function handleContextMenu(comic: Comic) {
  if (selectedIds.value.has(comic.id)) {
    return
  }

  selectedIds.value.clear()
  selectedIds.value.add(comic.id)
}

function toggleAll() {
  if (checkedIds.value.size === currentPageComics.value.length) checkedIds.value.clear()
  else checkedIds.value = new Set(currentPageComics.value.map((c) => c.id))
}
async function exportSelected(format: 'pdf' | 'cbz') {
  if (bulkExporting.value) return
  const comics = currentPageComics.value.filter((c) => checkedIds.value.has(c.id))
  bulkExporting.value = true
  try {
    for (const comic of comics) await exportComic(comic, format)
    if (isMobile.value) store.mobileTab = 'progresses'
  } finally {
    bulkExporting.value = false
  }
}

function useDropdown() {
  const dropdownX = ref<number>(0)
  const dropdownY = ref<number>(0)
  const dropdownShowing = ref<boolean>(false)
  const dropdownOptions: DropdownOption[] = [
    {
      label: '勾选',
      key: 'check',
      icon: () => (
        <NIcon size="20">
          <PhCheck />
        </NIcon>
      ),
      props: {
        onClick: () => {
          managing.value = true
          selectedIds.value.forEach((id) => checkedIds.value.add(id))
          dropdownShowing.value = false
        },
      },
    },
    {
      label: '取消勾选',
      key: 'uncheck',
      icon: () => (
        <NIcon size="20">
          <PhX />
        </NIcon>
      ),
      props: {
        onClick: () => {
          selectedIds.value.forEach((id) => checkedIds.value.delete(id))
          dropdownShowing.value = false
        },
      },
    },
    {
      label: '全选',
      key: 'select-all',
      icon: () => (
        <NIcon size="20">
          <PhChecks />
        </NIcon>
      ),
      props: {
        onClick: () => {
          currentPageComics.value.forEach((comic) => selectedIds.value.add(comic.id))
          dropdownShowing.value = false
        },
      },
    },
  ]

  async function showDropdown(e: MouseEvent) {
    dropdownShowing.value = false
    await nextTick()
    dropdownShowing.value = true
    dropdownX.value = e.clientX
    dropdownY.value = e.clientY
  }

  return {
    dropdownX,
    dropdownY,
    dropdownShowing,
    dropdownOptions,
    showDropdown,
  }
}
</script>

<template>
  <div v-if="store.config !== undefined" class="library-pane flex-1 min-h-0 flex flex-col">
    <div
      v-if="store.runtimePlatform !== 'ios'"
      class="library-directory-bar pane-toolbar flex gap-2 box-border px-4 pt-3">
      <n-input-group v-if="store.runtimePlatform !== 'ios'" class="min-w-0 flex-1">
        <n-input-group-label size="small">导出目录</n-input-group-label>
        <n-input v-model:value="store.config.exportDir" size="small" readonly @click="selectExportDir" />
        <n-button v-if="!isMobile" class="w-10" size="small" @click="showExportDirInFileManager">
          <template #icon>
            <n-icon size="20">
              <PhFolderOpen />
            </n-icon>
          </template>
        </n-button>
      </n-input-group>
    </div>
    <div class="library-header">
      <span class="library-count">
        <strong>{{ downloadedComics.length }}</strong>
        本漫画
      </span>
      <update-downloaded-comics-button />
      <button
        class="library-toolbar-button"
        :disabled="!downloadedComics.length || bulkExporting"
        @click="toggleManaging">
        {{ managing ? '完成' : '管理' }}
      </button>
      <button class="library-more" aria-label="刷新书库" :disabled="refreshing" @click="refreshLibrary">
        <PhArrowClockwise :size="19" :class="{ 'library-refreshing': refreshing }" />
      </button>
    </div>
    <div v-if="managing" class="library-selection-toolbar">
      <span>已选 {{ checkedIds.size }} 本</span>
      <button class="library-toolbar-button" :disabled="!currentPageComics.length" @click="toggleAll">
        {{ checkedIds.size && checkedIds.size === currentPageComics.length ? '取消全选' : '全选本页' }}
      </button>
    </div>
    <div v-if="libraryError" class="library-storage-error" role="alert">
      <strong>书库读取失败</strong>
      <p>{{ libraryError }}</p>
      <p>请在设置的“存储位置”确认目录，并检查 LiveContainer 当前选中的数据容器。</p>
      <button class="library-toolbar-button" :disabled="refreshing" @click="refreshLibrary">重新读取书库</button>
    </div>
    <div v-if="!downloadedComics.length && !libraryError" class="empty-state">
      <n-empty description="书库还没有漫画，下载完成后会显示在这里" />
    </div>
    <component
      v-else-if="downloadedComics.length"
      :is="isMobile ? 'div' : SelectionArea"
      class="library-list selection-container flex-1 min-h-0"
      ref="selectionAreaRef"
      :options="{ selectables: '.selectable', features: { deselectOnBlur: true } }"
      @contextmenu="!isMobile && showDropdown($event)"
      @move="updateSelectedIds"
      @start="unselectAll">
      <DownloadedComicCard
        v-for="comic in currentPageComics"
        :key="comic.id"
        :data-key="comic.id"
        :class="['selectable', selectedIds.has(comic.id) ? 'selected' : '']"
        :comic="comic"
        :managing="managing"
        :checkbox-checked="checkboxChecked"
        :handle-checkbox-click="handleCheckboxClick"
        :handle-context-menu="handleContextMenu"
        @changed="refreshLibrary" />
    </component>

    <div v-if="checkedIds.size" class="library-bulk-actions">
      <button class="library-action library-action-primary" :disabled="bulkExporting" @click="exportSelected('pdf')">
        导出所选 PDF
      </button>
      <button class="library-action" :disabled="bulkExporting" @click="exportSelected('cbz')">导出所选 CBZ</button>
    </div>
    <n-pagination
      v-if="pageCount > 1"
      class="box-border p-2 pt-0 mt-auto"
      :simple="isMobile"
      :page-count="pageCount"
      :page="currentPage"
      @update:page="currentPage = $event" />

    <n-dropdown
      placement="bottom-start"
      trigger="manual"
      :x="dropdownX"
      :y="dropdownY"
      :options="dropdownOptions"
      :show="dropdownShowing"
      :on-clickoutside="() => (dropdownShowing = false)" />
  </div>
</template>

<style scoped>
.selection-container {
  @apply select-none overflow-auto;
}

.selection-container .selected {
  @apply bg-[rgb(204,232,255)];
}
</style>
