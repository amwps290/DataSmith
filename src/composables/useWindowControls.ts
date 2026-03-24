import { ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'

export function useWindowControls() {
  const appWindow = getCurrentWindow()
  const isMaximized = ref(false)

  async function minimizeWindow() {
    try { await appWindow.minimize() } catch (e) { console.error(e) }
  }

  async function toggleMaximize() {
    try {
      await appWindow.toggleMaximize()
      isMaximized.value = await appWindow.isMaximized()
    } catch (e) { console.error(e) }
  }

  async function closeWindow() {
    try { await appWindow.close() } catch (e) { console.error(e) }
  }

  function setupMaximizeListener() {
    appWindow.isMaximized().then(v => { isMaximized.value = v }).catch(console.error)
    appWindow.onResized(async () => {
      try { isMaximized.value = await appWindow.isMaximized() } catch (e) { console.error(e) }
    })
  }

  return { isMaximized, minimizeWindow, toggleMaximize, closeWindow, setupMaximizeListener }
}
