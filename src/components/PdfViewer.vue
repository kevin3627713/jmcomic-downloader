<script setup lang="ts">
import { computed } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { NModal, NButton } from 'naive-ui'

const props = defineProps<{
  pdfPath: string | null
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'update:show', value: boolean): void
}>()

const pdfUrl = computed(() => {
  if (!props.pdfPath) return null
  return convertFileSrc(props.pdfPath)
})

function close() {
  emit('update:show', false)
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
      <div class="flex-1">
        <iframe
          v-if="pdfUrl"
          :src="pdfUrl"
          class="w-full h-full border-0"
          title="PDF Viewer" />
      </div>
    </div>
  </NModal>
</template>
