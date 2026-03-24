import { ref } from 'vue'

export function useSidebarResize(initialWidth = 280, minWidth = 200, maxWidth = 600) {
  const sidebarWidth = ref(initialWidth)

  function startResize(e: MouseEvent) {
    const startX = e.clientX
    const startWidth = sidebarWidth.value

    const onMouseMove = (event: MouseEvent) => {
      const newWidth = startWidth + (event.clientX - startX)
      if (newWidth >= minWidth && newWidth <= maxWidth) {
        sidebarWidth.value = newWidth
      }
    }

    const onMouseUp = () => {
      document.removeEventListener('mousemove', onMouseMove)
      document.removeEventListener('mouseup', onMouseUp)
    }

    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseup', onMouseUp)
  }

  return { sidebarWidth, startResize }
}
