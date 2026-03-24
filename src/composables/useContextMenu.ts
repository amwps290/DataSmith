import { ref, onMounted, onUnmounted } from 'vue'

export function useContextMenu() {
  const contextMenuVisible = ref(false)
  const contextMenuX = ref(0)
  const contextMenuY = ref(0)

  function showContextMenu(e: MouseEvent) {
    e.preventDefault()
    contextMenuX.value = e.clientX
    contextMenuY.value = e.clientY
    contextMenuVisible.value = true
  }

  function hideContextMenu() {
    contextMenuVisible.value = false
  }

  // 点击外部关闭
  function handleClickOutside() {
    contextMenuVisible.value = false
  }

  onMounted(() => document.addEventListener('click', handleClickOutside))
  onUnmounted(() => document.removeEventListener('click', handleClickOutside))

  return {
    contextMenuVisible,
    contextMenuX,
    contextMenuY,
    showContextMenu,
    hideContextMenu,
  }
}
