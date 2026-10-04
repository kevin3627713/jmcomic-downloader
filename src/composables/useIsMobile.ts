import { ref, onScopeDispose } from 'vue'

export function useIsMobile() {
  const mediaQuery = window.matchMedia('(max-width: 767px)')
  const ios =
    /iPad|iPhone|iPod/.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1)
  const isMobile = ref(mediaQuery.matches || ios)
  const handler = () => {
    isMobile.value = mediaQuery.matches || ios
  }
  mediaQuery.addEventListener('change', handler)
  onScopeDispose(() => mediaQuery.removeEventListener('change', handler))

  return isMobile
}
