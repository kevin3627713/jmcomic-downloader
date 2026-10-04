<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { commands, FavoriteSort } from '../bindings.ts'
import { useMessage } from 'naive-ui'
import ComicCard from '../components/ComicCard.vue'
import { SelectProps } from 'naive-ui'
import { useStore } from '../store.ts'
import { useIsMobile } from '../composables/useIsMobile'
import DownloadAllFavoriteButton from '../components/DownloadAllFavoriteButton.vue'

const store = useStore()
const isMobile = useIsMobile()

const message = useMessage()

const sortOptions: SelectProps['options'] = [
  { label: '收藏时间', value: 'FavoriteTime' },
  { label: '更新时间', value: 'UpdateTime' },
]

const sortSelected = ref<FavoriteSort>('FavoriteTime')
const pageSelected = ref<number>(1)
const folderIdSelected = ref<number>(0)

const favoritePageCount = computed(() => {
  const PAGE_SIZE = 20
  if (store.getFavoriteResult === undefined) {
    return 0
  }
  const total = store.getFavoriteResult.total
  return Math.ceil(total / PAGE_SIZE)
})
const folderOptions = computed<SelectProps['options']>(() => [
  { label: '全部', value: 0 },
  ...(store.getFavoriteResult?.folderList || []).map((folder) => ({
    label: folder.name,
    value: parseInt(folder.FID),
  })),
])

watch(
  () => store.userProfile,
  async () => {
    if (store.userProfile === undefined) {
      store.getFavoriteResult = undefined
      return
    }
    await getFavourite(0, 1, 'FavoriteTime')
  },
  { immediate: true },
)

async function getFavourite(folderId: number, page: number, sort: FavoriteSort) {
  folderIdSelected.value = folderId
  sortSelected.value = sort
  pageSelected.value = page
  const result = await commands.getFavoriteFolder(folderId, page, sort)
  if (result.status === 'error') {
    message.error(result.error.err_message)
    return
  }
  store.getFavoriteResult = result.data
}

async function syncFavoriteFolder() {
  const result = await commands.syncFavoriteFolder()
  if (result.status === 'error') {
    message.error(result.error.err_message)
    return
  }
  await getFavourite(0, 1, 'FavoriteTime')
  message.success('收藏夹已同步')
}
</script>

<template>
  <div class="flex-1 min-h-0 flex flex-col gap-2">
    <div v-if="store.getFavoriteResult !== undefined" class="pane-toolbar flex flex-wrap gap-2 box-border px-4 pt-3">
      <n-select
        class="flex-1 min-w-0"
        v-model:value="folderIdSelected"
        :options="folderOptions"
        :show-checkmark="false"
        size="small"
        @update-value="getFavourite($event, 1, sortSelected)" />
      <n-select
        class="flex-1 min-w-0"
        v-model:value="sortSelected"
        :options="sortOptions"
        :show-checkmark="false"
        size="small"
        @update-value="getFavourite(folderIdSelected, 1, $event)" />
      <download-all-favorite-button />
    </div>

    <div v-if="store.getFavoriteResult !== undefined" class="flex box-border px-2 gap-2">
      <n-tooltip v-if="!isMobile" placement="top" trigger="hover">
        <span>已弃用，请改用</span>
        <span class="bg-gray-2/30 px-1 rounded">本地库存</span>
        <span>中的</span>
        <span class="bg-gray-2/30 px-1 rounded">更新库存</span>
        <span>按钮</span>
        <template #trigger>
          <n-button disabled class="ml-auto" size="small">更新漫画</n-button>
        </template>
      </n-tooltip>
      <n-button size="small" type="primary" secondary @click="syncFavoriteFolder">收藏不对点我</n-button>
    </div>

    <div
      v-if="store.getFavoriteResult !== undefined"
      class="flex flex-col gap-row-2 overflow-auto box-border px-4 pb-3 flex-1 min-h-0">
      <ComicCard
        v-for="comicInFavorite in store.getFavoriteResult?.list"
        :key="comicInFavorite.id"
        :comic-id="comicInFavorite.id"
        :comic-title="comicInFavorite.name"
        :comic-author="comicInFavorite.author"
        :comic-category="comicInFavorite.category"
        :comic-category-sub="comicInFavorite.categorySub"
        :comic-downloaded="comicInFavorite.isDownloaded"
        :comic-download-dir="comicInFavorite.comicDownloadDir" />
    </div>

    <div v-if="!store.userProfile" class="empty-state">
      <n-empty description="登录账号后可查看收藏夹" />
      <p>点击右上角的账号按钮登录</p>
    </div>
    <n-pagination
      v-if="favoritePageCount > 0"
      class="box-border p-2 pt-0 mt-auto"
      :simple="isMobile"
      :page-count="favoritePageCount"
      :page="pageSelected"
      @update:page="getFavourite(folderIdSelected, $event, sortSelected)" />
  </div>
</template>
