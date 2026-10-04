<script setup lang="ts">
import { Comic, commands } from '../../../bindings'
import { useStore } from '../../../store'
import { useIsMobile } from '../../../composables/useIsMobile'
import { useComicExport } from '../../../composables/useComicExport'
const store = useStore()
const isMobile = useIsMobile()
const { exportComic, openPdf, shareCbz } = useComicExport()
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
async function openDirectory() {
  if (props.comic.comicDownloadDir) await commands.showPathInFileManager(props.comic.comicDownloadDir)
}
</script>
<template>
  <article class="library-card" @contextmenu="handleContextMenu(comic)">
    <div class="library-card-main">
      <img
        class="library-cover"
        :src="`https://${store.currentImageCdnDomain}/media/albums/${comic.id}_3x4.jpg`"
        alt="漫画封面"
        :draggable="false"
        referrerpolicy="no-referrer" />
      <div class="library-text">
        <button class="comic-title-button line-clamp-2" @click="pickComic">{{ comic.name }}</button>
        <p>{{ comic.author.join('、') || '未知作者' }}</p>
        <p>JM {{ comic.id }} · 已下载 {{ comic.chapterInfos.filter((c) => c.isDownloaded).length }} 章</p>
      </div>
      <n-checkbox
        class="self-start"
        :aria-label="`选择 ${comic.name}`"
        size="large"
        :checked="checkboxChecked(comic)"
        @update:checked="handleCheckboxClick(comic)" />
    </div>
    <div class="library-card-actions">
      <n-button @click="openPdf(comic)">查看 PDF</n-button>
      <n-button :loading="store.exportingComics.has(`${comic.id}:pdf`)" @click="exportComic(comic, 'pdf', true)">
        导出 PDF
      </n-button>
      <n-button :loading="store.exportingComics.has(`${comic.id}:cbz`)" @click="exportComic(comic, 'cbz', true)">
        导出 CBZ
      </n-button>
      <n-button v-if="store.runtimePlatform === 'ios'" @click="openPdf(comic, false)">分享 PDF</n-button>
      <n-button v-if="store.runtimePlatform === 'ios'" @click="shareCbz(comic)">分享 CBZ</n-button>
      <n-button v-if="!isMobile" @click="openDirectory">下载目录</n-button>
    </div>
  </article>
</template>
