<script setup lang="ts">
import { computed, ref } from 'vue'
import { commands, SearchSort } from '../bindings.ts'
import { useMessage } from 'naive-ui'
import ComicCard from '../components/ComicCard.vue'
import { PhMagnifyingGlass } from '@phosphor-icons/vue'
import { SelectProps } from 'naive-ui'
import { useStore } from '../store.ts'
import { useIsMobile } from '../composables/useIsMobile'

const store = useStore()
const isMobile = useIsMobile()

const message = useMessage()

const sortOptions: SelectProps['options'] = [
  { label: '最新', value: 'Latest' },
  { label: '最多点击', value: 'View' },
  { label: '最多图片', value: 'Picture' },
  { label: '最多爱心', value: 'Like' },
]

const searchInput = ref<string>('')
const searching = ref<boolean>(false)
const sortSelected = ref<SearchSort>('Latest')
const searchPage = ref<number>(1)

const searchPageCount = computed(() => {
  const PAGE_SIZE = 80
  if (store.searchResult === undefined) {
    return 0
  }
  const total = store.searchResult.total
  return Math.ceil(total / PAGE_SIZE)
})

async function search(keyword: string, page: number, sort: SearchSort) {
  if (searching.value) {
    message.warning('有搜索正在进行，请稍后再试')
    return
  }

  searching.value = true
  searchPage.value = page
  try {
    const result = await commands.search(keyword, page, sort)
    if (result.status === 'error') {
      message.error(result.error.err_message)
      return
    }
    const searchResultVariant = result.data
    if ('SearchResult' in searchResultVariant) {
      store.searchResult = searchResultVariant.SearchResult
      if (!store.searchResult.content.length) message.warning('什么都没有搜到，请尝试其他关键词')
    } else if ('Comic' in searchResultVariant) {
      store.pickedComic = searchResultVariant.Comic
      store.currentTabName = 'chapter'
    }
  } catch (error) {
    message.error(String(error))
  } finally {
    searching.value = false
  }
}
</script>

<template>
  <div class="flex-1 min-h-0 flex flex-col gap-2">
    <div class="pane-toolbar px-4 pt-3">
      <div class="flex gap-2 min-w-0">
        <n-input
          v-model:value="searchInput"
          class="flex-1 min-w-0"
          aria-label="漫画关键词或 JM 号"
          placeholder="搜索关键词 / JM 号"
          clearable
          @keydown.enter="search(searchInput.trim(), 1, sortSelected)" />
        <n-button
          :loading="searching"
          type="primary"
          aria-label="搜索漫画"
          @click="search(searchInput.trim(), 1, sortSelected)">
          <template #icon><PhMagnifyingGlass :size="22" /></template>
          搜索
        </n-button>
      </div>
      <div class="flex items-center gap-2 mt-3">
        <span class="text-xs text-gray flex-1">
          {{ store.searchResult ? `共 ${store.searchResult.total} 个结果` : '输入 JM 号可直接查看章节' }}
        </span>
        <n-select
          class="w-30"
          aria-label="排序"
          v-model:value="sortSelected"
          :options="sortOptions"
          :show-checkmark="false"
          @update-value="search(searchInput.trim(), 1, $event)" />
      </div>
    </div>
    <div v-if="!store.searchResult" class="empty-state">
      <n-empty description="发现下一本想读的漫画" />
      <p>支持漫画标题、作者和 JM 号</p>
    </div>
    <div
      v-if="store.searchResult !== undefined"
      class="flex flex-col gap-row-2 overflow-auto box-border px-4 pb-3 flex-1 min-h-0">
      <ComicCard
        v-for="comicInSearch in store.searchResult.content"
        :key="comicInSearch.id"
        :comic-id="comicInSearch.id"
        :comic-title="comicInSearch.name"
        :comic-author="comicInSearch.author"
        :comic-category="comicInSearch.category"
        :comic-category-sub="comicInSearch.categorySub"
        :comic-downloaded="comicInSearch.isDownloaded"
        :comic-download-dir="comicInSearch.comicDownloadDir" />
    </div>

    <n-pagination
      v-if="searchPageCount > 0"
      class="box-border p-2 pt-0 mt-auto"
      :simple="isMobile"
      :page-count="searchPageCount"
      :page="searchPage"
      @update:page="search(searchInput.trim(), $event, sortSelected)" />
  </div>
</template>
