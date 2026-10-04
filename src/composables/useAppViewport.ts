import { onMounted, onUnmounted } from 'vue'

function isEditingText(element: Element | null): boolean {
  if (element instanceof HTMLTextAreaElement) return !element.disabled && !element.readOnly
  if (element instanceof HTMLInputElement) {
    return (
      !element.disabled &&
      !element.readOnly &&
      ['text', 'search', 'email', 'url', 'tel', 'password', 'number'].includes(element.type)
    )
  }
  return element instanceof HTMLElement && element.isContentEditable
}

export function useAppViewport() {
  const root = document.documentElement
  const viewport = window.visualViewport
  let frame: number | undefined
  let observer: ResizeObserver | undefined

  function update() {
    const layoutHeight = root.clientHeight || window.innerHeight
    const layoutWidth = root.clientWidth || window.innerWidth
    const keyboardOpen =
      isEditingText(document.activeElement) &&
      viewport != null &&
      viewport.height > 0 &&
      Math.abs(viewport.scale - 1) < 0.01 &&
      Math.abs(viewport.width - layoutWidth) < 2 &&
      layoutHeight - viewport.height > 120

    // WKWebView can report a visual viewport with safe areas already subtracted
    // on first launch. Normal layout fills the WebView through CSS, without
    // freezing that smaller value into the app's height.
    root.style.setProperty('--app-height', `${keyboardOpen ? viewport!.height : layoutHeight}px`)
    root.style.setProperty('--app-top', `${keyboardOpen ? viewport!.offsetTop : 0}px`)
    root.classList.toggle('keyboard-open', keyboardOpen)
    if (keyboardOpen) root.style.setProperty('--keyboard-height', `${viewport!.height}px`)
    else root.style.removeProperty('--keyboard-height')
  }

  function scheduleUpdate() {
    if (frame !== undefined) cancelAnimationFrame(frame)
    frame = requestAnimationFrame(() => {
      frame = undefined
      update()
    })
  }

  onMounted(() => {
    update()
    scheduleUpdate()
    observer = new ResizeObserver(scheduleUpdate)
    observer.observe(root)
    viewport?.addEventListener('resize', scheduleUpdate)
    viewport?.addEventListener('scroll', scheduleUpdate)
    window.addEventListener('resize', scheduleUpdate)
    window.addEventListener('orientationchange', scheduleUpdate)
    window.addEventListener('pageshow', scheduleUpdate)
    window.addEventListener('focus', scheduleUpdate)
    document.addEventListener('focusin', scheduleUpdate)
    document.addEventListener('focusout', scheduleUpdate)
    document.addEventListener('visibilitychange', scheduleUpdate)
  })

  onUnmounted(() => {
    if (frame !== undefined) cancelAnimationFrame(frame)
    observer?.disconnect()
    viewport?.removeEventListener('resize', scheduleUpdate)
    viewport?.removeEventListener('scroll', scheduleUpdate)
    window.removeEventListener('resize', scheduleUpdate)
    window.removeEventListener('orientationchange', scheduleUpdate)
    window.removeEventListener('pageshow', scheduleUpdate)
    window.removeEventListener('focus', scheduleUpdate)
    document.removeEventListener('focusin', scheduleUpdate)
    document.removeEventListener('focusout', scheduleUpdate)
    document.removeEventListener('visibilitychange', scheduleUpdate)
    root.style.removeProperty('--app-height')
    root.style.removeProperty('--app-top')
    root.style.removeProperty('--keyboard-height')
    root.classList.remove('keyboard-open')
  })
}
