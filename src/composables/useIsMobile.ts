import { ref, onMounted, onUnmounted } from 'vue'

const isMobile = ref(false)

export function useIsMobile() {
  onMounted(() => {
    const mediaQuery = window.matchMedia('(min-width: 768px)')
    isMobile.value = !mediaQuery.matches

    const handler = (e: MediaQueryListEvent) => {
      isMobile.value = !e.matches
    }
    mediaQuery.addEventListener('change', handler)

    onUnmounted(() => {
      mediaQuery.removeEventListener('change', handler)
    })
  })

  return isMobile
}
