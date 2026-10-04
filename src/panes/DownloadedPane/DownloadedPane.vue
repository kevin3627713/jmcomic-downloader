<script setup lang="tsx">
import { Comic, commands } from '../../bindings.ts'
import { computed, nextTick, ref, watch, watchEffect } from 'vue'
import DownloadedComicCard from './components/DownloadedComicCard.vue'
import { open } from '@tauri-apps/plugin-dialog'
import { PhFolderOpen } from '@phosphor-icons/vue'
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
watch(
  () => store.currentTabName,
  async () => {
    if (store.currentTabName !== 'downloaded') {
      return
    }

    downloadedComics.value = await commands.getDownloadedComics()
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
  <div v-if="store.config !== undefined" class="flex-1 min-h-0 flex flex-col">
    <div class="pane-toolbar flex gap-2 box-border px-4 pt-3">
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
      <update-downloaded-comics-button />
    </div>
    <div class="chapter-tools">
      <span>{{ downloadedComics.length }} 本漫画 · 已选 {{ checkedIds.size }} 本</span>
      <n-button :disabled="!currentPageComics.length" @click="toggleAll">
        {{ checkedIds.size && checkedIds.size === currentPageComics.length ? '取消全选' : '全选本页' }}
      </n-button>
      <n-button
        @click="
          async () => {
            downloadedComics = await commands.getDownloadedComics()
          }
        ">
        刷新
      </n-button>
    </div>
    <p v-if="store.runtimePlatform === 'ios'" class="px-4 m-0 mb-2 text-xs text-gray">
      导出完成后可分享或保存到“文件”。
    </p>
    <div v-if="!downloadedComics.length" class="empty-state">
      <n-empty description="书库还没有漫画，下载完成后会显示在这里" />
    </div>
    <component
      :is="isMobile ? 'div' : SelectionArea"
      class="flex flex-col overflow-auto box-border px-4 selection-container mb-2 flex-1 min-h-0"
      ref="selectionAreaRef"
      :options="{ selectables: '.selectable', features: { deselectOnBlur: true } }"
      @contextmenu="!isMobile && showDropdown($event)"
      @move="updateSelectedIds"
      @start="unselectAll">
      <DownloadedComicCard
        v-for="comic in currentPageComics"
        :key="comic.id"
        :data-key="comic.id"
        :class="['selectable mb-2', selectedIds.has(comic.id) ? 'selected shadow-md' : 'hover:bg-gray-1']"
        :comic="comic"
        :checkbox-checked="checkboxChecked"
        :handle-checkbox-click="handleCheckboxClick"
        :handle-context-menu="handleContextMenu" />
    </component>

    <div v-if="checkedIds.size" class="chapter-actions">
      <n-button class="flex-1" type="primary" :loading="bulkExporting" @click="exportSelected('pdf')">
        导出所选 PDF
      </n-button>
      <n-button class="flex-1" :loading="bulkExporting" @click="exportSelected('cbz')">导出所选 CBZ</n-button>
    </div>
    <n-pagination
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
