<script setup lang="ts">
import { PhCircleNotch } from '@phosphor-icons/vue'
import { ProgressData } from './ExportProgresses.vue'
import { commands } from '../../../bindings'
import { useStore } from '../../../store'
import { useComicExport } from '../../../composables/useComicExport'
const props = defineProps<{ p: ProgressData; handleContextMenu: (p: ProgressData) => void }>()
const store = useStore()
const { openFiles } = useComicExport()
async function openResult(preview = false) {
  if (!props.p.chapterExportDir) return
  if (store.runtimePlatform === 'ios') await openFiles([props.p.chapterExportDir], preview)
  else await commands.showPathInFileManager(props.p.chapterExportDir)
}
</script>
<template>
  <article class="library-card mb-2" @contextmenu="handleContextMenu(p)">
    <strong class="block truncate" :title="p.comicTitle">{{ p.comicTitle }}</strong>
    <div v-if="p.state === 'Processing'" class="flex items-center mt-2">
      <PhCircleNotch :size="20" class="animate-spin mr-2 text-orange-5" />
      <n-progress :percentage="p.percentage" processing>{{ p.indicator }}</n-progress>
    </div>
    <p v-else :class="p.state === 'Error' ? 'text-red-5' : 'text-green-6'">{{ p.indicator }}</p>
    <div v-if="p.state === 'End' && p.chapterExportDir" class="action-row mt-2">
      <template v-if="store.runtimePlatform === 'ios'">
        <n-button v-if="p.merged" @click="openResult(true)">查看 PDF</n-button>
        <n-button v-if="p.merged || p.exportType === 'cbz'" type="primary" secondary @click="openResult()">
          分享 / 存到文件
        </n-button>
      </template>
      <n-button v-else @click="openResult()">打开导出位置</n-button>
    </div>
  </article>
</template>
