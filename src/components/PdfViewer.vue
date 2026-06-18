<script setup lang="ts">
import { ref, watch } from 'vue'
import { VuePDF, usePDF } from '@tato30/vue-pdf'
import '@tato30/vue-pdf/style.css'
import { commands } from '../bindings.ts'
import { NModal, NSpin, NEmpty, NButton } from 'naive-ui'

const props = defineProps<{
  pdfPath: string | null
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
}>()

const loading = ref(false)
const error = ref<string | null>(null)
const pdfData = ref<Uint8Array | null>(null)

const { pdf, pages } = usePDF(pdfData)

watch(
  () => props.pdfPath,
  async (newPath) => {
    if (!newPath || !props.show) {
      pdfData.value = null
      return
    }

    loading.value = true
    error.value = null

    try {
      const result = await commands.readPdfFile(newPath)
      if (result.status === 'error') {
        error.value = result.error.err_message
        return
      }
      pdfData.value = new Uint8Array(result.data)
    } catch (e) {
      error.value = e instanceof Error ? e.message : '加载PDF失败'
    } finally {
      loading.value = false
    }
  },
  { immediate: true },
)

function close() {
  emit('update:show', false)
  pdfData.value = null
}
</script>

<template>
  <NModal :show="show" @update:show="close" :mask-closable="true" :close-on-esc="true">
    <div class="fixed inset-0 z-50 flex flex-col bg-white">
      <!-- Header -->
      <div class="flex items-center justify-between px-4 py-2 border-b border-gray-200">
        <span class="font-bold text-lg truncate flex-1">PDF 查看器</span>
        <NButton quaternary size="small" @click="close">
          <template #icon>
            <span class="i-ph-x" />
          </template>
        </NButton>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-auto flex items-center justify-center p-4">
        <NSpin v-if="loading" size="large" />
        <NEmpty v-else-if="error" :description="error" />
        <NEmpty v-else-if="!pdfData" description="无PDF数据" />
        <div v-else class="w-full h-full overflow-auto">
          <div v-if="pdf" class="flex flex-col items-center gap-4">
            <VuePDF
              v-for="pageNumber in pages"
              :key="pageNumber"
              :pdf="pdf"
              :page="pageNumber"
              text-layer
              class="max-w-full shadow-lg" />
          </div>
        </div>
      </div>
    </div>
  </NModal>
</template>
