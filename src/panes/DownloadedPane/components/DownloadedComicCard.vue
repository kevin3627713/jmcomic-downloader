<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { commands, type Comic, type ComicFileStatus, type DeleteKind } from '../../../bindings'
import { useMessage } from 'naive-ui'
import {
  PhDotsThree,
  PhFilePdf,
  PhFileZip,
  PhShareNetwork,
  PhTrash,
  PhDownloadSimple,
  PhFolderOpen,
  PhX,
} from '@phosphor-icons/vue'
import { useStore } from '../../../store'
import { useIsMobile } from '../../../composables/useIsMobile'
import { useComicExport } from '../../../composables/useComicExport'
import { useLibraryRemoval } from '../../../composables/useLibraryRemoval'

const store = useStore()
defineOptions({ inheritAttrs: false })
const message = useMessage()
const isMobile = useIsMobile()
const { exportComic, openPdf, shareCbz } = useComicExport()
const { removeComicState } = useLibraryRemoval()
const props = defineProps<{
  comic: Comic
  managing: boolean
  checkboxChecked: (comic: Comic) => boolean
  handleCheckboxClick: (comic: Comic) => void
  handleContextMenu: (comic: Comic) => void
}>()
const emit = defineEmits<{ changed: [] }>()
const files = ref<ComicFileStatus>({ pdfCount: 0, cbzCount: 0, hasPdf: false })
const loadingFiles = ref(true)
const fileError = ref('')
const menuShowing = ref(false)
const confirmShowing = ref(false)
const deleteKind = ref<DeleteKind>('all')
const deleting = ref(false)
const exporting = computed(() =>
  ['pdf', 'cbz'].some((format) => store.exportingComics.has(`${props.comic.id}:${format}`)),
)
const busy = computed(() => exporting.value || deleting.value)
let fileRequest = 0

async function refreshFiles() {
  const request = ++fileRequest
  loadingFiles.value = true
  try {
    const result = await commands.getComicFileStatus(props.comic)
    if (request !== fileRequest) return
    if (result.status === 'error') fileError.value = result.error.err_message
    else {
      files.value = result.data
      fileError.value = ''
    }
  } catch (error) {
    if (request === fileRequest) fileError.value = String(error)
  } finally {
    if (request === fileRequest) loadingFiles.value = false
  }
}
watch(
  () => [props.comic, exporting.value] as const,
  () => {
    if (!exporting.value) void refreshFiles()
  },
  { immediate: true },
)

function pickComic() {
  store.pickedComic = props.comic
  store.currentTabName = 'chapter'
}
async function openDirectory() {
  menuShowing.value = false
  if (props.comic.comicDownloadDir) await commands.showPathInFileManager(props.comic.comicDownloadDir)
}
async function exportFormat(format: 'pdf' | 'cbz') {
  menuShowing.value = false
  await exportComic(props.comic, format, true)
}
function previewPdf() {
  menuShowing.value = false
  void openPdf(props.comic)
}
function sharePdf() {
  menuShowing.value = false
  void openPdf(props.comic, false)
}
function openCbz() {
  menuShowing.value = false
  void shareCbz(props.comic)
}
function chooseDelete(kind: DeleteKind) {
  deleteKind.value = kind
  menuShowing.value = false
  confirmShowing.value = true
}
const deleteTitle = computed(
  () => ({ all: '删除整本漫画？', pdf: '删除 PDF 文件？', cbz: '删除 CBZ 文件？' })[deleteKind.value],
)
const deleteDescription = computed(
  () =>
    ({
      all: '将删除这本漫画的下载图片、下载记录，以及对应目录中的全部导出文件。之后可以重新下载。',
      pdf: `将删除整本及章节 PDF，共 ${files.value.pdfCount} 个文件。下载图片和 CBZ 保留，可以再次导出 PDF。`,
      cbz: `将删除章节 CBZ，共 ${files.value.cbzCount} 个文件。下载图片和 PDF 保留，可以再次导出 CBZ。`,
    })[deleteKind.value],
)
async function confirmDelete() {
  if (busy.value) return false
  deleting.value = true
  try {
    const result = await commands.deleteComicFiles(props.comic, deleteKind.value)
    if (result.status === 'error') {
      message.error(result.error.err_message)
      emit('changed')
      return false
    }
    if (result.data.removedComic) removeComicState(props.comic.id)
    message.success(
      result.data.removedComic
        ? '已删除漫画及导出文件'
        : result.data.removedFiles
          ? `已删除 ${result.data.removedFiles} 个 ${deleteKind.value.toUpperCase()} 文件`
          : '没有需要删除的文件',
    )
    confirmShowing.value = false
    if (!result.data.removedComic) await refreshFiles()
    emit('changed')
    return true
  } catch (error) {
    message.error(String(error))
    emit('changed')
    return false
  } finally {
    deleting.value = false
  }
}
</script>

<template>
  <article
    v-bind="$attrs"
    class="library-card"
    :class="{ 'library-card-checked': checkboxChecked(comic) }"
    @contextmenu="handleContextMenu(comic)">
    <div class="library-card-main">
      <img
        class="library-cover"
        :src="`https://${store.currentImageCdnDomain}/media/albums/${comic.id}_3x4.jpg`"
        alt="漫画封面"
        :draggable="false"
        referrerpolicy="no-referrer" />
      <div class="library-text">
        <button class="comic-title-button line-clamp-2" @click="pickComic">{{ comic.name }}</button>
        <p class="library-author">{{ comic.author.join('、') || '未知作者' }}</p>
        <p class="library-details">
          {{ comic.chapterInfos.filter((chapter) => chapter.isDownloaded).length }} 章
          <span>·</span>
          JM {{ comic.id }}
        </p>
        <div class="library-file-badges" aria-label="导出文件状态">
          <span v-if="loadingFiles" class="file-badge">读取文件状态…</span>
          <span v-else-if="fileError" class="file-badge">文件状态待刷新</span>
          <template v-else>
            <span v-if="files.pdfCount" class="file-badge">
              <PhFilePdf :size="12" />
              {{ files.hasPdf ? 'PDF' : '章节 PDF' }}
            </span>
            <span v-if="files.cbzCount" class="file-badge">
              <PhFileZip :size="12" />
              CBZ
            </span>
            <span v-if="!files.pdfCount && !files.cbzCount" class="library-unexported">尚未导出</span>
          </template>
        </div>
      </div>
      <n-checkbox
        v-if="managing"
        class="library-selection"
        :aria-label="`选择 ${comic.name}`"
        :checked="checkboxChecked(comic)"
        @update:checked="handleCheckboxClick(comic)" />
    </div>
    <div class="library-card-actions">
      <button
        class="library-action library-action-primary"
        :disabled="busy || loadingFiles"
        @click="files.hasPdf ? openPdf(comic) : exportFormat('pdf')">
        <PhFilePdf :size="17" />
        {{ files.hasPdf ? '查看 PDF' : '导出 PDF' }}
      </button>
      <button
        class="library-action"
        :disabled="busy || loadingFiles"
        @click="files.cbzCount ? shareCbz(comic) : exportFormat('cbz')">
        <PhFileZip :size="17" />
        {{ files.cbzCount ? (store.runtimePlatform === 'ios' ? '分享 CBZ' : '打开 CBZ') : '导出 CBZ' }}
      </button>
      <button class="library-more" :aria-label="`更多操作 · ${comic.name}`" @click="menuShowing = true">
        <PhDotsThree :size="24" />
      </button>
    </div>
    <p v-if="exporting" class="library-busy-note">正在导出…</p>
  </article>

  <n-modal v-model:show="menuShowing">
    <section class="library-sheet" role="dialog" aria-modal="true" :aria-label="`${comic.name}的文件操作`">
      <header class="library-sheet-header">
        <div>
          <h3>文件与操作</h3>
          <p class="line-clamp-2">{{ comic.name }}</p>
        </div>
        <button class="library-more" aria-label="关闭文件操作" @click="menuShowing = false"><PhX :size="20" /></button>
      </header>
      <div class="library-sheet-body">
        <button v-if="files.hasPdf" class="library-menu-item" @click="previewPdf">
          <PhFilePdf :size="20" />
          <span>查看 PDF</span>
        </button>
        <button v-if="store.runtimePlatform === 'ios' && files.hasPdf" class="library-menu-item" @click="sharePdf">
          <PhShareNetwork :size="20" />
          <span>分享 PDF</span>
        </button>
        <button v-if="store.runtimePlatform === 'ios' && files.cbzCount" class="library-menu-item" @click="openCbz">
          <PhShareNetwork :size="20" />
          <span>分享 CBZ</span>
        </button>
        <button class="library-menu-item" :disabled="busy" @click="exportFormat('pdf')">
          <PhDownloadSimple :size="20" />
          <span>{{ files.hasPdf ? '重新导出 PDF' : '导出 PDF' }}</span>
        </button>
        <button class="library-menu-item" :disabled="busy" @click="exportFormat('cbz')">
          <PhDownloadSimple :size="20" />
          <span>{{ files.cbzCount ? '重新导出 CBZ' : '导出 CBZ' }}</span>
        </button>
        <button v-if="!isMobile" class="library-menu-item" @click="openDirectory">
          <PhFolderOpen :size="20" />
          <span>下载目录</span>
        </button>
        <div class="library-menu-divider"></div>
        <button
          class="library-menu-item library-menu-danger"
          :disabled="busy || loadingFiles || !files.pdfCount"
          @click="chooseDelete('pdf')">
          <PhTrash :size="20" />
          <span>删除 PDF</span>
          <small>{{ files.pdfCount }} 个文件</small>
        </button>
        <button
          class="library-menu-item library-menu-danger"
          :disabled="busy || loadingFiles || !files.cbzCount"
          @click="chooseDelete('cbz')">
          <PhTrash :size="20" />
          <span>删除 CBZ</span>
          <small>{{ files.cbzCount }} 个文件</small>
        </button>
        <button class="library-menu-item library-menu-danger" :disabled="busy" @click="chooseDelete('all')">
          <PhTrash :size="20" />
          <span>删除整本漫画</span>
        </button>
      </div>
    </section>
  </n-modal>
  <n-modal
    v-model:show="confirmShowing"
    preset="dialog"
    type="warning"
    :title="deleteTitle"
    positive-text="确认删除"
    negative-text="取消"
    :mask-closable="!deleting"
    :close-on-esc="!deleting"
    :closable="!deleting"
    :positive-button-props="{ type: 'error', loading: deleting }"
    :negative-button-props="{ disabled: deleting }"
    @positive-click="confirmDelete">
    <p class="library-delete-name">{{ comic.name }}</p>
    <p class="library-delete-description">{{ deleteDescription }}</p>
    <p class="library-delete-warning">删除后无法撤销。</p>
  </n-modal>
</template>
