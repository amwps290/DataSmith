import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager'

export async function writeClipboardText(text: string) {
  await writeText(text)
}

export async function readClipboardText() {
  return readText()
}
