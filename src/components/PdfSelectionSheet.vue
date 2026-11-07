<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { PhBookOpen, PhFilePdf, PhX } from '@phosphor-icons/vue'
import { commands, type Comic } from '../bindings'
import { useStore } from '../store'
import { orderedChapters, pdfRequest, selectedChapterIds, type PdfLayout, type PdfScope } from '../utils/pdfSelection'

const props = withDefaults(defineProps<{ show: boolean; comic: Comic; initialAction?: 'read' | 'export' }>(), {
  initialAction: 'read',
})
const emit = defineEmits<{ 'update:show': [value: boolean]; changed: [] }>()
const store = useStore()
const syncedComic = ref<Comic>()
const loading = ref(false)
const working = ref(false)
const error = ref('')
const action = ref<'read' | 'export'>('read')
const scope = ref<PdfScope>('chapters')
const layout = ref<PdfLayout>('merged')
const selected = ref<number[]>([])
const query = ref('')
const resultPaths = ref<string[]>([])
const resultSummary = ref('')
let revision = 0
const chapters = computed(() => orderedChapters(syncedComic.value?.chapterInfos ?? []))
const available = computed(() => chapters.value.filter((chapter) => chapter.isDownloaded === true))
const chosen = computed(() => selectedChapterIds(chapters.value, selected.value))
const fullBook = computed(() => chapters.value.length > 0 && chapters.value.length === available.value.length)
const visibleChapters = computed(() => {
  const term = query.value.trim().toLowerCase()
  return chapters.value.filter(
    (chapter) => !term || `${chapter.order} ${chapter.chapterTitle}`.toLowerCase().includes(term),
  )
})
const key = computed(() => `${props.comic.id}:pdf`)
const busy = computed(() => working.value || store.exportingComics.has(key.value))
const count = computed(() => (scope.value === 'whole' ? chapters.value.length : chosen.value.length))
const valid = computed(
  () => !loading.value && !!syncedComic.value && (scope.value === 'whole' ? fullBook.value : count.value > 0),
)
const outputCount = computed(() => (scope.value === 'whole' || layout.value === 'merged' ? 1 : count.value))
const summary = computed(
  () => `${scope.value === 'whole' ? '整本' : `所选 ${count.value} 章`} · ${outputCount.value} 个 PDF · 按原章节顺序`,
)
const actionLabel = computed(() => (action.value === 'read' ? '生成并阅读' : '导出 PDF'))

function close() {
  if (!working.value) emit('update:show', false)
}
async function loadComic() {
  const request = ++revision
  loading.value = true
  error.value = ''
  syncedComic.value = undefined
  resultPaths.value = []
  resultSummary.value = ''
  selected.value = []
  query.value = ''
  action.value = props.initialAction
  layout.value = 'merged'
  try {
    const result = await commands.getSyncedComic(props.comic)
    if (request !== revision || !props.show) return
    if (result.status === 'error') throw new Error(result.error.err_message)
    syncedComic.value = result.data
    scope.value = fullBook.value ? 'whole' : 'chapters'
  } catch (cause) {
    if (request === revision) error.value = String(cause instanceof Error ? cause.message : cause)
  } finally {
    if (request === revision) loading.value = false
  }
}
watch(
  () => [props.show, props.comic.id] as const,
  ([show]) => {
    if (show) void loadComic()
    else ++revision
  },
  { immediate: true },
)
onBeforeUnmount(() => ++revision)
watch([scope, layout, selected, action], () => {
  error.value = ''
  resultPaths.value = []
  resultSummary.value = ''
})
function toggle(id: number, checked: boolean) {
  selected.value = checked ? [...selected.value, id] : selected.value.filter((item) => item !== id)
}
function toggleAll() {
  selected.value =
    chosen.value.length === available.value.length ? [] : available.value.map((chapter) => chapter.chapterId)
}
async function openResult(preview: boolean) {
  if (!resultPaths.value.length || working.value) return
  error.value = ''
  working.value = true
  try {
    const result = await commands.openExportedFiles(resultPaths.value, preview)
    if (result.status === 'error') throw new Error(result.error.err_message)
  } catch (cause) {
    error.value = `PDF 已生成，但打开失败：${cause instanceof Error ? cause.message : String(cause)}。可再次点击下方按钮。`
  } finally {
    working.value = false
  }
}
async function run() {
  if (!valid.value || busy.value || !syncedComic.value) return
  const request = revision
  const comic = syncedComic.value
  const exportKey = `${comic.id}:pdf`
  const shouldRead = action.value === 'read'
  const currentSummary = summary.value
  error.value = ''
  resultPaths.value = []
  working.value = true
  store.exportingComics.add(exportKey)
  store.progressesPaneTabName = 'export'
  try {
    const selection = pdfRequest(chapters.value, scope.value, selected.value, layout.value)
    const result = await commands.exportPdfSelection(comic, selection.chapterIds, selection.merge)
    if (request !== revision) return
    if (result.status === 'error') throw new Error(result.error.err_message)
    if (!result.data.length) throw new Error('没有生成 PDF 文件，请刷新章节状态后重试。')
    resultPaths.value = result.data
    resultSummary.value = currentSummary
    emit('changed')
    if (shouldRead) {
      const opened = await commands.openExportedFiles(result.data, true)
      if (request !== revision) return
      if (opened.status === 'error')
        throw new Error(`PDF 已生成，但阅读器打开失败：${opened.error.err_message}。请点击下方“阅读”重试。`)
    }
  } catch (cause) {
    if (request === revision) error.value = cause instanceof Error ? cause.message : String(cause)
  } finally {
    store.exportingComics.delete(exportKey)
    working.value = false
  }
}
</script>

<template>
  <n-modal :show="show" :mask-closable="!working" :close-on-esc="!working" @update:show="close">
    <section class="pdf-sheet" role="dialog" aria-modal="true" aria-label="PDF 阅读与导出">
      <header class="pdf-header">
        <div>
          <h2>PDF 阅读与导出</h2>
          <p>{{ comic.name }}</p>
        </div>
        <button class="pdf-close" aria-label="关闭 PDF 面板" :disabled="working" @click="close">
          <PhX :size="22" />
        </button>
      </header>
      <div class="pdf-scroll">
        <div class="pdf-segment" aria-label="操作方式">
          <button :aria-pressed="action === 'read'" :disabled="busy" @click="action = 'read'">
            <PhBookOpen :size="18" />
            阅读
          </button>
          <button :aria-pressed="action === 'export'" :disabled="busy" @click="action = 'export'">
            <PhFilePdf :size="18" />
            导出
          </button>
        </div>
        <p class="pdf-hint">
          {{
            action === 'read'
              ? '按所选范围生成 PDF，再打开阅读器。'
              : '只生成你需要的 PDF，不再同时导出整本和分章文件。'
          }}
        </p>
        <p v-if="loading" class="pdf-state" role="status">正在检查已下载章节…</p>
        <template v-else-if="syncedComic">
          <div class="pdf-section-title">
            <h3>内容范围</h3>
            <span>已下载 {{ available.length }} / {{ chapters.length }} 章</span>
          </div>
          <div class="pdf-scope">
            <button :aria-pressed="scope === 'whole'" :disabled="busy || !fullBook" @click="scope = 'whole'">
              <strong>整本</strong>
              <small>{{ fullBook ? `全部 ${chapters.length} 章` : '需先下载全部章节' }}</small>
            </button>
            <button :aria-pressed="scope === 'chapters'" :disabled="busy" @click="scope = 'chapters'">
              <strong>选择章节</strong>
              <small>单章或多章</small>
            </button>
          </div>
          <template v-if="scope === 'chapters'">
            <div class="pdf-selection-tools">
              <strong>已选 {{ chosen.length }} 章</strong>
              <button :disabled="busy || !available.length" @click="toggleAll">
                {{ chosen.length === available.length && available.length ? '清空选择' : '全选已下载' }}
              </button>
            </div>
            <input
              v-model="query"
              class="pdf-search"
              :disabled="busy"
              type="search"
              placeholder="搜索章节名称或序号"
              aria-label="搜索章节" />
            <div class="pdf-chapters" aria-label="可选章节">
              <label
                v-for="chapter in visibleChapters"
                :key="chapter.chapterId"
                class="pdf-chapter"
                :class="{ 'is-selected': chosen.includes(chapter.chapterId), 'is-unavailable': !chapter.isDownloaded }">
                <input
                  type="checkbox"
                  :checked="chosen.includes(chapter.chapterId)"
                  :disabled="busy || !chapter.isDownloaded"
                  @change="toggle(chapter.chapterId, ($event.target as HTMLInputElement).checked)" />
                <span>
                  <strong>{{ chapter.chapterTitle }}</strong>
                  <small>第 {{ chapter.order }} 章 · {{ chapter.isDownloaded ? '已下载' : '未下载，暂不可选' }}</small>
                </span>
              </label>
              <p v-if="!visibleChapters.length" class="pdf-state">
                {{ chapters.length ? '没有匹配的章节' : '没有章节记录' }}
              </p>
            </div>
            <p v-if="!available.length" class="pdf-hint">请先返回章节页下载，再打开此面板。</p>
          </template>
          <template v-if="scope === 'chapters' && count > 1">
            <div class="pdf-section-title">
              <h3>{{ action === 'read' ? '阅读方式' : '文件形式' }}</h3>
            </div>
            <div class="pdf-scope">
              <button :aria-pressed="layout === 'merged'" :disabled="busy" @click="layout = 'merged'">
                <strong>合并为一个 PDF</strong>
                <small>{{ action === 'read' ? '连续阅读所选章节' : '按原章节顺序合并' }}</small>
              </button>
              <button :aria-pressed="layout === 'separate'" :disabled="busy" @click="layout = 'separate'">
                <strong>每章一个 PDF</strong>
                <small>{{ action === 'read' ? '在系统阅读器中切换' : `生成 ${count} 个独立文件` }}</small>
              </button>
            </div>
          </template>
        </template>
        <div v-if="error" class="pdf-error" role="alert">{{ error }}</div>
        <button v-if="!syncedComic && !loading" class="pdf-retry" @click="loadComic">重新检查章节</button>
        <div v-if="resultPaths.length" class="pdf-result" role="status">
          <strong>已生成 {{ resultPaths.length }} 个 PDF</strong>
          <p>{{ resultSummary }}</p>
          <div class="pdf-result-actions">
            <button :disabled="working" @click="openResult(true)">阅读</button>
            <button :disabled="working" @click="openResult(false)">
              {{ store.runtimePlatform === 'ios' ? '分享 / 存储到文件' : '打开导出文件' }}
            </button>
          </div>
        </div>
      </div>
      <footer class="pdf-footer">
        <p aria-live="polite">{{ working ? '正在处理，请稍候…' : valid ? summary : '请选择已下载章节' }}</p>
        <n-button type="primary" block :disabled="!valid || busy" :loading="working" @click="run">
          {{ actionLabel }}
        </n-button>
        <small v-if="working">可在进度页查看；完成前请勿关闭此面板。</small>
      </footer>
    </section>
  </n-modal>
</template>

<style scoped>
.pdf-sheet {
  width: min(600px, calc(100vw - 24px));
  max-height: min(90dvh, 850px);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: #fff;
  border-radius: 20px;
  color: #282b32;
}
.pdf-header {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 20px 20px 14px;
  border-bottom: 1px solid #eceef1;
}
.pdf-header > div {
  flex: 1;
  min-width: 0;
}
.pdf-header h2 {
  font-size: 19px;
  margin: 0 0 5px;
}
.pdf-header p {
  margin: 0;
  font-size: 13px;
  color: #747b87;
  overflow-wrap: anywhere;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.pdf-close {
  border: 0;
  width: 44px;
  height: 44px;
  border-radius: 12px;
  background: #f4f5f7;
  display: grid;
  place-items: center;
  flex: none;
}
.pdf-scroll {
  overflow-y: auto;
  overscroll-behavior: contain;
  min-height: 0;
  padding: 16px 20px 20px;
}
.pdf-segment {
  display: flex;
  padding: 4px;
  background: #f2f3f6;
  border-radius: 12px;
  gap: 4px;
}
.pdf-segment button {
  flex: 1;
  display: flex;
  gap: 8px;
  justify-content: center;
  align-items: center;
  min-height: 44px;
  border: 0;
  border-radius: 9px;
  background: transparent;
  color: #666d77;
}
.pdf-segment button[aria-pressed='true'] {
  background: #fff;
  color: #b65000;
  font-weight: 700;
  box-shadow: 0 1px 5px #0000000d;
}
.pdf-hint {
  color: #747b87;
  font-size: 12px;
  line-height: 1.65;
  margin: 10px 0 20px;
}
.pdf-section-title {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  margin: 16px 0 10px;
}
.pdf-section-title h3 {
  font-size: 14px;
  margin: 0;
}
.pdf-section-title span {
  color: #747b87;
  font-size: 12px;
}
.pdf-scope {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
.pdf-scope button {
  text-align: left;
  padding: 12px;
  min-height: 70px;
  border: 1px solid #e4e7ec;
  border-radius: 12px;
  background: #fff;
  color: inherit;
}
.pdf-scope button[aria-pressed='true'] {
  border-color: #ef8a3e;
  background: #fff7ee;
}
.pdf-scope strong {
  display: block;
  font-size: 13px;
}
.pdf-scope small {
  display: block;
  font-size: 11px;
  line-height: 1.5;
  margin-top: 5px;
  color: #747b87;
}
button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.pdf-selection-tools {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 12px;
  font-size: 13px;
}
.pdf-selection-tools button {
  min-height: 44px;
  padding: 0 8px;
  border: 0;
  background: transparent;
  color: #b65000;
}
.pdf-search {
  width: 100%;
  min-height: 44px;
  border: 1px solid #e4e7ec;
  border-radius: 10px;
  padding: 10px 12px;
  font: inherit;
  font-size: 13px;
  margin-bottom: 8px;
}
.pdf-search:focus {
  outline: 2px solid #ef8a3e;
  outline-offset: 1px;
}
.pdf-chapters {
  max-height: 32dvh;
  min-height: 55px;
  overflow-y: auto;
  overscroll-behavior: contain;
  border: 1px solid #e4e7ec;
  border-radius: 12px;
}
.pdf-chapter {
  display: flex;
  align-items: center;
  min-height: 58px;
  padding: 10px 12px;
  gap: 12px;
  border-bottom: 1px solid #edf0f3;
  cursor: pointer;
}
.pdf-chapter:last-child {
  border-bottom: 0;
}
.pdf-chapter input {
  width: 20px;
  height: 20px;
  flex: none;
  accent-color: #e87415;
}
.pdf-chapter span {
  min-width: 0;
}
.pdf-chapter strong {
  display: block;
  font-size: 13px;
  font-weight: 500;
  overflow-wrap: anywhere;
}
.pdf-chapter small {
  color: #7b828d;
  font-size: 11px;
}
.pdf-chapter.is-selected {
  background: #fff7ee;
}
.pdf-chapter.is-unavailable {
  background: #f7f8fa;
  color: #8a9099;
  cursor: default;
}
.pdf-state {
  padding: 20px 8px;
  text-align: center;
  color: #747b87;
  font-size: 13px;
}
.pdf-error {
  background: #fff0ee;
  color: #a83527;
  padding: 12px;
  border-radius: 10px;
  font-size: 13px;
  line-height: 1.6;
  margin-top: 16px;
  overflow-wrap: anywhere;
}
.pdf-retry {
  border: 1px solid #e4e7ec;
  background: white;
  border-radius: 10px;
  min-height: 44px;
  padding: 0 16px;
  margin-top: 12px;
}
.pdf-result {
  margin-top: 16px;
  padding: 14px;
  background: #f0f8f2;
  border-radius: 12px;
  font-size: 13px;
}
.pdf-result p {
  margin: 5px 0 12px;
  color: #65756b;
  font-size: 12px;
}
.pdf-result-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.pdf-result-actions button {
  min-height: 44px;
  padding: 8px 12px;
  border: 1px solid #c8dccd;
  background: #fff;
  border-radius: 9px;
  color: #315d3e;
}
.pdf-footer {
  padding: 12px 20px max(16px, env(safe-area-inset-bottom));
  border-top: 1px solid #eceef1;
  flex: none;
  background: #fff;
}
.pdf-footer p {
  margin: 0 0 10px;
  font-size: 12px;
  color: #666d77;
}
.pdf-footer :deep(.n-button) {
  min-height: 46px;
  border-radius: 12px;
}
.pdf-footer small {
  display: block;
  margin-top: 8px;
  color: #747b87;
  font-size: 11px;
}
@media (max-width: 600px) {
  .pdf-sheet {
    max-height: calc(100dvh - env(safe-area-inset-top) - 24px);
    width: calc(100vw - 16px);
  }
  .pdf-header {
    padding: 16px 16px 12px;
  }
  .pdf-scroll {
    padding: 14px 16px 18px;
  }
  .pdf-footer {
    padding-left: 16px;
    padding-right: 16px;
  }
  .pdf-scope {
    gap: 8px;
  }
}
</style>
