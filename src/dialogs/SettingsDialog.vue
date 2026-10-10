<script setup lang="ts">
import { commands, type Config, type StorageInfo } from '../bindings'
import { ref, watch } from 'vue'
import { useStore } from '../store'
import { useMessage } from 'naive-ui'

const store = useStore()
const message = useMessage()
const showing = defineModel<boolean>('showing', { required: true })
const dirFmt = ref(store.config?.dirFmt ?? '')
const proxyHost = ref(store.config?.proxyHost ?? '')
const customApiDomain = ref(store.config?.customApiDomain ?? '')
const storageInfo = ref<StorageInfo>()
const storageError = ref('')
let storageRequest = 0

type NumericKey =
  | 'chapterConcurrency'
  | 'imgConcurrency'
  | 'chapterDownloadIntervalSec'
  | 'imgDownloadIntervalSec'
  | 'downloadAllFavoritesIntervalSec'
  | 'updateDownloadedComicsIntervalSec'
  | 'proxyPort'
const speedSettings: { key: NumericKey; label: string; hint: string; min: number; restart?: boolean }[] = [
  { key: 'chapterConcurrency', label: '章节并发数', hint: '同时下载的章节数，重启后生效', min: 1, restart: true },
  { key: 'imgConcurrency', label: '图片并发数', hint: '同时下载的图片数，重启后生效', min: 1, restart: true },
  { key: 'chapterDownloadIntervalSec', label: '章节下载间隔（秒）', hint: '每个章节下载完成后的休息时间', min: 0 },
  { key: 'imgDownloadIntervalSec', label: '图片下载间隔（秒）', hint: '每张图片下载完成后的休息时间', min: 0 },
  {
    key: 'downloadAllFavoritesIntervalSec',
    label: '收藏夹下载间隔（秒）',
    hint: '批量下载时，每本漫画处理后的休息时间',
    min: 0,
  },
  {
    key: 'updateDownloadedComicsIntervalSec',
    label: '书库更新间隔（秒）',
    hint: '更新书库时，每本漫画处理后的休息时间',
    min: 0,
  },
]
const formatOptions = [
  { label: 'JPG · 体积小，编码快', value: 'Jpeg' },
  { label: 'PNG · 无损，支持长图', value: 'Png' },
  { label: 'WebP · 无损', value: 'Webp' },
]
const domainOptions = [1, 2, 3, 4, 5]
  .map((n) => ({ label: `线路 ${n}`, value: `Domain${n}` }))
  .concat([{ label: '自定义域名', value: 'Custom' }])
const proxyOptions = [
  { label: '系统代理', value: 'System' },
  { label: '直接连接', value: 'NoProxy' },
  { label: '自定义 HTTP 代理', value: 'Custom' },
]
const formatHints: Record<Config['downloadFormat'], string> = {
  Jpeg: '有损压缩，宽高上限为 65534 像素。',
  Png: '文件较大，编码较慢，适合超过 JPG / WebP 尺寸上限的条漫。',
  Webp: '宽高上限为 16383 像素，超长条漫请使用 PNG。',
}

function updateNumber(key: NumericKey, value: number | null, restart = false) {
  if (!store.config || value === null || !Number.isFinite(value)) return
  if (store.config[key] === value) return
  store.config[key] = value
  if (restart) message.warning('并发数的修改将在重启后生效')
}

watch(showing, async (show) => {
  const request = ++storageRequest
  if (!show) return
  dirFmt.value = store.config?.dirFmt ?? ''
  proxyHost.value = store.config?.proxyHost ?? ''
  customApiDomain.value = store.config?.customApiDomain ?? ''
  storageInfo.value = undefined
  storageError.value = ''
  try {
    const result = await commands.getStorageInfo()
    if (request !== storageRequest) return
    if (result.status === 'ok') storageInfo.value = result.data
    else storageError.value = result.error.err_message
  } catch (error) {
    if (request === storageRequest) storageError.value = String(error)
  }
})
watch([() => store.config?.apiDomainMode, () => store.config?.customApiDomain], () => {
  message.warning('切换线路后可能需要重新登录')
})

function saveDirectoryFormat() {
  if (!store.config) return
  const parts = dirFmt.value.split('/').filter((part) => part.trim())
  if (parts.length < 2) {
    message.warning('下载目录格式至少需要漫画和章节两个层级')
    dirFmt.value = store.config.dirFmt
    return
  }
  store.config.dirFmt = dirFmt.value
}

async function showConfigInFileManager() {
  if (!storageInfo.value) return
  const result = await commands.showPathInFileManager(storageInfo.value.configPath)
  if (result.status === 'error') message.error(result.error.err_message)
}
</script>

<template>
  <n-modal v-if="store.config" v-model:show="showing">
    <n-dialog class="settings-dialog" :show-icon="false" title="设置" @close="showing = false">
      <div class="settings-form">
        <p class="settings-note">修改后自动保存</p>
        <section>
          <h3>存储位置</h3>
          <p v-if="store.runtimePlatform === 'ios'" class="settings-note">
            漫画和导出文件保存在当前应用的 Documents 中。在 LiveContainer 的当前数据容器内打开 Documents
            即可查找。配置和日志保存在 Library 内，兼容旧版。可长按下方路径复制。
          </p>
          <p v-if="storageError" class="storage-error">{{ storageError }}</p>
          <template v-if="storageInfo">
            <p v-for="warning in storageInfo.migrationWarnings" :key="warning" class="storage-error">{{ warning }}</p>
            <label
              v-for="location in [
                { label: '漫画下载目录', path: storageInfo.downloadDir },
                { label: 'PDF / CBZ 导出目录', path: storageInfo.exportDir },
                { label: '配置文件位置', path: storageInfo.configPath },
              ]"
              :key="location.label"
              class="settings-field">
              <span>{{ location.label }}</span>
              <n-input
                :value="location.path"
                type="textarea"
                readonly
                :autosize="{ minRows: 2, maxRows: 5 }"
                :input-props="{ 'aria-label': location.label }" />
            </label>
          </template>
        </section>
        <section>
          <h3>下载速度</h3>
          <div class="settings-grid">
            <label v-for="setting in speedSettings" :key="setting.key" class="settings-field">
              <span>{{ setting.label }}</span>
              <n-input-number
                :value="store.config[setting.key]"
                :min="setting.min"
                :precision="0"
                :show-button="false"
                :input-props="{ 'aria-label': setting.label, inputmode: 'numeric' }"
                @update:value="updateNumber(setting.key, $event, setting.restart)" />
              <small>{{ setting.hint }}</small>
            </label>
          </div>
        </section>
        <section>
          <h3>文件与下载</h3>
          <label class="settings-field">
            <span>图片保存格式</span>
            <n-select v-model:value="store.config.downloadFormat" :options="formatOptions" aria-label="图片保存格式" />
            <small>{{ formatHints[store.config.downloadFormat] }}</small>
          </label>
          <label class="settings-field">
            <span>下载目录格式</span>
            <n-input
              v-model:value="dirFmt"
              aria-label="下载目录格式"
              @blur="saveDirectoryFormat"
              @keydown.enter="saveDirectoryFormat" />
            <small>用 / 分隔漫画和章节目录，至少两个层级。</small>
          </label>
          <details class="directory-format-help">
            <summary>查看目录格式可用字段</summary>
            <p>
              漫画：
              <code>{comic_id}</code>
              、
              <code>{comic_title}</code>
              、
              <code>{author}</code>
            </p>
            <p>
              章节：
              <code>{chapter_id}</code>
              、
              <code>{chapter_title}</code>
              、
              <code>{order}</code>
            </p>
            <p>
              示例：
              <code>{comic_title}/{order} - {chapter_title}</code>
            </p>
          </details>
          <n-checkbox class="settings-checkbox" v-model:checked="store.config.shouldDownloadCover">下载封面</n-checkbox>
        </section>
        <section>
          <h3>网络</h3>
          <label class="settings-field">
            <span>API 线路</span>
            <n-select v-model:value="store.config.apiDomainMode" :options="domainOptions" aria-label="API 线路" />
          </label>
          <label v-if="store.config.apiDomainMode === 'Custom'" class="settings-field">
            <span>自定义 API 域名</span>
            <n-input
              v-model:value="customApiDomain"
              aria-label="自定义 API 域名"
              placeholder="example.com"
              :input-props="{ autocapitalize: 'none', autocorrect: 'off' }"
              @blur="store.config.customApiDomain = customApiDomain.trim()"
              @keydown.enter="store.config.customApiDomain = customApiDomain.trim()" />
          </label>
          <label class="settings-field">
            <span>代理类型</span>
            <n-select v-model:value="store.config.proxyMode" :options="proxyOptions" aria-label="代理类型" />
          </label>
          <div v-if="store.config.proxyMode === 'Custom'" class="settings-grid">
            <label class="settings-field">
              <span>HTTP 代理地址</span>
              <n-input
                v-model:value="proxyHost"
                aria-label="HTTP 代理地址"
                placeholder="127.0.0.1"
                :input-props="{ autocapitalize: 'none', autocorrect: 'off' }"
                @blur="store.config.proxyHost = proxyHost.trim()"
                @keydown.enter="store.config.proxyHost = proxyHost.trim()" />
            </label>
            <label class="settings-field">
              <span>代理端口</span>
              <n-input-number
                :value="store.config.proxyPort"
                :min="1"
                :max="65535"
                :precision="0"
                :show-button="false"
                :input-props="{ 'aria-label': '代理端口', inputmode: 'numeric' }"
                @update:value="updateNumber('proxyPort', $event)" />
            </label>
          </div>
        </section>
      </div>
      <template #action>
        <div class="action-row justify-end">
          <n-button v-if="store.runtimePlatform !== 'ios'" @click="showConfigInFileManager">打开配置目录</n-button>
          <n-button type="primary" @click="showing = false">完成</n-button>
        </div>
      </template>
    </n-dialog>
  </n-modal>
</template>
