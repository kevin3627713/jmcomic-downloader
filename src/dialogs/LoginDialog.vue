<script setup lang="ts">
import { ref } from 'vue'
import { commands } from '../bindings.ts'
import { useMessage } from 'naive-ui'
import FloatLabelInput from '../components/FloatLabelInput.vue'
import { useStore } from '../store.ts'

const store = useStore()

const message = useMessage()

const showing = defineModel<boolean>('showing', { required: true })

const username = ref<string>(store.config?.username ?? '')
const password = ref<string>(store.config?.password ?? '')
const remember = ref<boolean>(username.value !== '' && password.value !== '')
const loggingIn = ref(false)

async function onLogin() {
  if (store.config === undefined || loggingIn.value) {
    return
  }
  if (username.value === '') {
    message.error('请输入用户名')
    return
  }
  if (password.value === '') {
    message.error('请输入密码')
    return
  }

  loggingIn.value = true
  try {
    const result = await commands.login(username.value, password.value)
    if (result.status === 'error') {
      message.error(result.error.err_message)
      return false
    }
    store.userProfile = result.data
    message.success('登录成功')
    if (remember.value) {
      store.config.username = username.value
      store.config.password = password.value
    }
    showing.value = false
  } catch (error) {
    message.error(String(error))
    return false
  } finally {
    loggingIn.value = false
  }
}

function clearUsernameAndPasswordInConfig() {
  if (store.config === undefined) {
    return
  }

  store.config.username = ''
  store.config.password = ''
}
</script>

<template>
  <n-modal v-model:show="showing">
    <n-dialog
      :showIcon="false"
      title="账号登录"
      positive-text="登录"
      :positive-button-props="{ loading: loggingIn, disabled: loggingIn }"
      @positive-click="onLogin"
      @close="showing = false"
      @keydown.enter="onLogin">
      <div class="flex flex-col gap-2">
        <FloatLabelInput label="用户名" v-model:value="username" />
        <FloatLabelInput label="密码" v-model:value="password" type="password" />
        <div class="flex flex-wrap gap-2 justify-between">
          <n-tooltip>
            用户名和密码将以明文保存在配置文件中
            <template #trigger>
              <n-checkbox v-model:checked="remember">记住我</n-checkbox>
            </template>
          </n-tooltip>
          <n-button type="primary" size="tiny" secondary @click="clearUsernameAndPasswordInConfig">
            清除保存的账号
          </n-button>
        </div>
      </div>
    </n-dialog>
  </n-modal>
</template>
