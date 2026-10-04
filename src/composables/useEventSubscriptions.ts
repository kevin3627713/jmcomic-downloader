import { onScopeDispose } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'

export function useEventSubscriptions() {
  const listeners: UnlistenFn[] = []
  let disposed = false
  onScopeDispose(() => {
    disposed = true
    listeners.splice(0).forEach((unlisten) => unlisten())
  })
  return async (subscription: Promise<UnlistenFn>) => {
    const unlisten = await subscription
    if (disposed) unlisten()
    else listeners.push(unlisten)
  }
}
