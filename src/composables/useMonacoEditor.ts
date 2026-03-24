import { onBeforeUnmount, watch, type Ref, shallowRef, nextTick } from 'vue'
import * as monaco from 'monaco-editor'
import { useAppStore } from '@/stores/app'

export interface MonacoEditorOptions {
  language?: string
  readOnly?: boolean
  theme?: string
  minimap?: boolean
  lineNumbers?: 'on' | 'off'
  value?: string
  fontSize?: number
  scrollBeyondLastLine?: boolean
}

export function useMonacoEditor(
  containerRef: Ref<HTMLElement | undefined | null>,
  options: MonacoEditorOptions = {}
) {
  const editor = shallowRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const appStore = useAppStore()

  const getTheme = () => {
    if (options.theme) return options.theme
    return appStore.theme === 'dark' ? 'vs-dark' : 'vs'
  }

  const createEditor = async () => {
    if (editor.value) return
    await nextTick()
    if (!containerRef.value) return

    editor.value = monaco.editor.create(containerRef.value, {
      language: options.language || 'sql',
      theme: getTheme(),
      readOnly: options.readOnly ?? false,
      minimap: { enabled: options.minimap ?? false },
      lineNumbers: options.lineNumbers || 'on',
      automaticLayout: true,
      value: options.value || '',
      fontSize: options.fontSize ?? 13,
      scrollBeyondLastLine: options.scrollBeyondLastLine ?? false,
    })
  }

  const getValue = (): string => editor.value?.getValue() || ''

  const setValue = (value: string) => {
    if (editor.value) {
      editor.value.setValue(value)
    }
  }

  const dispose = () => {
    if (editor.value) {
      editor.value.dispose()
      editor.value = null
    }
  }

  // 监听主题变化
  watch(() => appStore.theme, () => {
    if (editor.value) {
      monaco.editor.setTheme(getTheme())
    }
  })

  onBeforeUnmount(() => dispose())

  return { editor, getValue, setValue, createEditor, dispose }
}
