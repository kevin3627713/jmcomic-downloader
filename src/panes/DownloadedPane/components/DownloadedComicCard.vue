<script setup lang="ts">
import { ref } from 'vue'
import { Comic, commands } from '../../../bindings.ts'
import { useStore } from '../../../store.ts'
import { PhFilePdf, PhFileZip, PhFolderOpen } from '@phosphor-icons/vue'
import IconButton from '../../../components/IconButton.vue'
import PdfViewer from '../../../components/PdfViewer.vue'
import { useIsMobile } from '../../../composables/useIsMobile'
import { useMessage } from 'naive-ui'

const store = useStore()
const isMobile = useIsMobile()
const message = useMessage()

const showPdfViewer = ref(false)
const pdfPath = ref<string | null>(null)

const props = defineProps<{
  comic: Comic
  checkboxChecked: (comic: Comic) => boolean
  handleCheckboxClick: (comic: Comic) => void
  handleContextMenu: (comic: Comic) => void
}>()

function pickComic() {
  store.pickedComic = props.comic
  store.currentTabName = 'chapter'
}

async function exportCbz() {
  store.progressesPaneTabName = 'export'
  const result = await commands.exportCbz(props.comic)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
}

async function exportPdf() {
  store.progressesPaneTabName = 'export'
  const result = await commands.exportPdf(props.comic)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
}

async function showComicDownloadDirInFileManager() {
  if (store.config === undefined) {
    return
  }

  const comicDownloadDir = props.comic.comicDownloadDir

  if (comicDownloadDir === undefined || comicDownloadDir === null) {
    console.error('comicDownloadDir的值为undefined或null')
    return
  }

  const result = await commands.showPathInFileManager(comicDownloadDir)
  if (result.status === 'error') {
    console.error(result.error)
  }
}

async function openComicPdf() {
  const result = await commands.getComicPdfPath(props.comic)
  if (result.status === 'error') {
    console.error(result.error)
    message.error('获取PDF路径失败')
    return
  }

  if (result.data === null) {
    message.warning('请先导出PDF')
    return
  }

  pdfPath.value = result.data
  showPdfViewer.value = true
}
</script>

<template>
  <div class="flex relative border border-solid rounded-md border-gray-2 p-1" @contextmenu="handleContextMenu(comic)">
    <n-checkbox
      size="large"
      class="absolute top-3 left-3 z-1"
      :checked="checkboxChecked(comic)"
      @click="handleCheckboxClick(comic)" />
    <img
      class="w-24 object-cover mr-4"
      :src="`https://${store.currentImageCdnDomain}/media/albums/${comic.id}_3x4.jpg`"
      alt=""
      :draggable="false"
      referrerpolicy="no-referrer" />
    <div class="flex flex-col w-full">
      <span
        class="font-bold text-lg line-clamp-2 cursor-pointer transition-colors duration-200 hover:text-blue-5"
        @click="pickComic">
        {{ comic.name }}
      </span>
      <span class="text-red">作者：{{ comic.author }}</span>
      <div class="flex mt-auto gap-2">
        <IconButton v-if="!isMobile" title="打开下载目录" @click="showComicDownloadDirInFileManager">
          <PhFolderOpen :size="24" />
        </IconButton>

        <IconButton v-if="isMobile" title="查看PDF" @click="openComicPdf">
          <PhFilePdf :size="24" />
        </IconButton>

        <IconButton class="ml-auto" title="导出cbz" @click="exportCbz">
          <PhFileZip :size="24" />
        </IconButton>

        <IconButton title="导出pdf" @click="exportPdf">
          <PhFilePdf :size="24" />
        </IconButton>
      </div>
    </div>
  </div>

  <PdfViewer v-model:show="showPdfViewer" :pdf-path="pdfPath" />
</template>
