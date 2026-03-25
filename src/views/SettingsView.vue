<template>
  <a-layout class="settings-layout" :class="{ 'dark-mode': appStore.theme === 'dark' }">
    <AppHeader
      :show-search="false"
      @new-connection="goHome"
      @open-settings="noop"
      @open-search="noop"
    />

    <a-layout-content class="settings-content">
      <div class="settings-shell">
        <aside class="settings-sidebar">
          <div class="settings-sidebar-header">
            <h2>{{ $t('settings_page.title') }}</h2>
            <a-button type="link" @click="goHome">{{ $t('settings_page.back_to_workspace') }}</a-button>
          </div>

          <a-menu v-model:selectedKeys="selectedKeys" mode="inline" class="settings-menu">
            <a-menu-item key="interface">
              <template #icon><AppstoreOutlined /></template>
              {{ $t('settings_page.interface_title') }}
            </a-menu-item>
            <a-menu-item key="editor">
              <template #icon><CodeOutlined /></template>
              {{ $t('settings_page.editor_title') }}
            </a-menu-item>
            <a-menu-item key="database">
              <template #icon><DatabaseOutlined /></template>
              {{ $t('settings_page.database_title') }}
            </a-menu-item>
          </a-menu>
        </aside>

        <main class="settings-main">
          <div class="settings-hero">
            <div>
              <p class="hero-eyebrow">{{ $t('settings_page.preferences') }}</p>
              <h1>{{ currentSectionTitle }}</h1>
              <p class="hero-description">{{ currentSectionDescription }}</p>
            </div>
            <div class="hero-status">
              <a-tag color="blue">{{ $t('settings_page.theme_mode') }}: {{ currentThemeModeLabel }}</a-tag>
              <a-tag color="geekblue">{{ $t('common.language') }}: {{ languageModel === 'zh-CN' ? '中文' : 'English' }}</a-tag>
            </div>
          </div>

          <template v-if="currentSection === 'interface'">
            <a-card :title="$t('settings_page.appearance_group')" class="settings-card">
              <div class="setting-row">
                <div class="setting-meta">
                  <div class="setting-label">{{ $t('common.theme') }}</div>
                  <div class="setting-help">{{ $t('settings_page.theme_help') }}</div>
                </div>
                <a-radio-group v-model:value="themeModeModel" button-style="solid">
                  <a-radio-button value="light">{{ $t('settings_page.theme_light') }}</a-radio-button>
                  <a-radio-button value="dark">{{ $t('settings_page.theme_dark') }}</a-radio-button>
                  <a-radio-button value="system">{{ $t('settings_page.theme_system') }}</a-radio-button>
                </a-radio-group>
              </div>

              <div class="setting-row">
                <div class="setting-meta">
                  <div class="setting-label">{{ $t('settings_page.current_theme') }}</div>
                  <div class="setting-help">{{ $t('settings_page.current_theme_help') }}</div>
                </div>
                <a-tag :color="appStore.theme === 'dark' ? 'purple' : 'gold'">
                  {{ appStore.theme === 'dark' ? $t('settings_page.theme_dark') : $t('settings_page.theme_light') }}
                </a-tag>
              </div>

              <div class="setting-row">
                <div class="setting-meta">
                  <div class="setting-label">{{ $t('settings_page.interface_font') }}</div>
                  <div class="setting-help">{{ $t('settings_page.interface_font_help') }}</div>
                </div>
                <a-select v-model:value="interfaceFontModel" style="width: 280px" :options="interfaceFontOptions" />
              </div>

              <div class="setting-row">
                <div class="setting-meta">
                  <div class="setting-label">{{ $t('common.language') }}</div>
                  <div class="setting-help">{{ $t('settings_page.language_help') }}</div>
                </div>
                <a-radio-group v-model:value="languageModel" button-style="solid">
                  <a-radio-button value="zh-CN">中文</a-radio-button>
                  <a-radio-button value="en-US">English</a-radio-button>
                </a-radio-group>
              </div>
            </a-card>

            <a-card :title="$t('settings_page.diagnostics_group')" class="settings-card secondary-card">
              <div class="setting-row">
                <div class="setting-meta">
                  <div class="setting-label">{{ $t('settings_page.log_level') }}</div>
                  <div class="setting-help">{{ $t('settings_page.log_level_help') }}</div>
                </div>
                <a-select v-model:value="logLevelModel" style="width: 220px" :options="logLevelOptions" />
              </div>
            </a-card>

            <a-card :title="$t('settings_page.preview_group')" class="settings-card secondary-card">
              <div class="preview-surface">
                <div class="preview-toolbar">
                  <span>{{ $t('settings_page.preview_title') }}</span>
                  <a-tag>{{ interfaceFontLabel }}</a-tag>
                </div>
                <div class="preview-body">
                  <div class="preview-panel">
                    <strong>{{ $t('connection.manager') }}</strong>
                    <p>{{ $t('settings_page.preview_sidebar') }}</p>
                  </div>
                  <div class="preview-panel">
                    <strong>{{ $t('editor.result') }}</strong>
                    <p>{{ $t('settings_page.preview_workspace') }}</p>
                  </div>
                </div>
              </div>
            </a-card>
          </template>

          <template v-else-if="currentSection === 'editor'">
            <a-card :title="$t('settings_page.editor_typography_group')" class="settings-card">
              <div class="setting-row">
                <div class="setting-meta">
                  <div class="setting-label">{{ $t('settings_page.editor_font') }}</div>
                  <div class="setting-help">{{ $t('settings_page.editor_font_help') }}</div>
                </div>
                <a-select v-model:value="editorFontModel" style="width: 320px" :options="editorFontOptions" />
              </div>

              <div class="setting-row">
                <div class="setting-meta">
                  <div class="setting-label">{{ $t('settings_page.font_size') }}</div>
                  <div class="setting-help">{{ $t('settings_page.font_size_help') }}</div>
                </div>
                <a-space>
                  <a-slider v-model:value="fontSizeModel" :min="12" :max="24" style="width: 220px" />
                  <a-input-number v-model:value="fontSizeModel" :min="12" :max="24" />
                </a-space>
              </div>
            </a-card>

            <a-card :title="$t('settings_page.editor_display_group')" class="settings-card secondary-card">
              <div class="setting-row">
                <div class="setting-meta">
                  <div class="setting-label">{{ $t('settings_page.line_numbers') }}</div>
                  <div class="setting-help">{{ $t('settings_page.line_numbers_help') }}</div>
                </div>
                <a-switch v-model:checked="lineNumbersEnabled" />
              </div>

              <div class="setting-row">
                <div class="setting-meta">
                  <div class="setting-label">{{ $t('settings_page.minimap') }}</div>
                  <div class="setting-help">{{ $t('settings_page.minimap_help') }}</div>
                </div>
                <a-switch v-model:checked="minimapModel" />
              </div>

              <div class="editor-preview">
                <div class="editor-preview-header">
                  <span>{{ $t('settings_page.editor_preview') }}</span>
                  <a-tag>{{ editorFontLabel }} · {{ fontSizeModel }}px</a-tag>
                </div>
                <pre class="editor-preview-code" :style="{ fontFamily: editorFontModel }">{{ previewSql }}</pre>
              </div>
            </a-card>
          </template>

          <template v-else>
            <a-card :title="$t('settings_page.database_connection_group')" class="settings-card">
              <div class="setting-row">
                <div class="setting-meta">
                  <div class="setting-label">{{ $t('settings_page.mysql_charset') }}</div>
                  <div class="setting-help">{{ $t('settings_page.mysql_charset_help') }}</div>
                </div>
                <a-select v-model:value="mysqlCharsetModel" style="width: 220px">
                  <a-select-option value="utf8mb4">utf8mb4</a-select-option>
                  <a-select-option value="utf8">utf8</a-select-option>
                  <a-select-option value="latin1">latin1</a-select-option>
                  <a-select-option value="gbk">gbk</a-select-option>
                </a-select>
              </div>

              <div class="setting-row">
                <div class="setting-meta">
                  <div class="setting-label">{{ $t('settings_page.mysql_init_sql') }}</div>
                  <div class="setting-help">{{ $t('settings_page.mysql_init_sql_help') }}</div>
                </div>
                <a-textarea v-model:value="mysqlInitSqlModel" :rows="4" style="width: 360px" :placeholder="$t('settings_page.mysql_init_sql_placeholder')" />
              </div>
            </a-card>

            <a-card :title="$t('settings_page.database_preview_group')" class="settings-card secondary-card">
              <div class="editor-preview">
                <div class="editor-preview-header">
                  <span>{{ $t('settings_page.database_preview_title') }}</span>
                  <a-tag>{{ mysqlCharsetModel }}</a-tag>
                </div>
                <pre class="editor-preview-code" :style="{ fontFamily: editorFontModel }">{{ databasePreviewSql }}</pre>
              </div>
            </a-card>
          </template>
        </main>
      </div>
    </a-layout-content>
  </a-layout>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { AppstoreOutlined, CodeOutlined, DatabaseOutlined } from '@ant-design/icons-vue'
import { useI18n } from 'vue-i18n'
import AppHeader from '@/components/layout/AppHeader.vue'
import { useAppStore, type Language, type LogLevel, type ThemeMode } from '@/stores/app'

const router = useRouter()
const appStore = useAppStore()
const { t } = useI18n()
const selectedKeys = ref<string[]>(['interface'])

const interfaceFontOptions = [
  { label: 'Inter / SF Pro', value: `Inter, "SF Pro Display", "Segoe UI", sans-serif` },
  { label: 'IBM Plex Sans', value: `"IBM Plex Sans", "Segoe UI", sans-serif` },
  { label: 'Source Sans 3', value: `"Source Sans 3", "Segoe UI", sans-serif` },
]

const editorFontOptions = [
  { label: 'JetBrains Mono', value: `"JetBrains Mono", "Fira Code", "Cascadia Code", monospace` },
  { label: 'Fira Code', value: `"Fira Code", "JetBrains Mono", "Cascadia Code", monospace` },
  { label: 'SF Mono', value: `"SFMono-Regular", "SF Mono", "Cascadia Code", monospace` },
  { label: 'Source Code Pro', value: `"Source Code Pro", "JetBrains Mono", monospace` },
]

const logLevelOptions = [
  { label: 'Error', value: 'error' },
  { label: 'Warn', value: 'warn' },
  { label: 'Info', value: 'info' },
  { label: 'Debug', value: 'debug' },
  { label: 'Trace', value: 'trace' },
]

const currentSection = computed(() => selectedKeys.value[0] || 'interface')
const currentSectionTitle = computed(() => {
  if (currentSection.value === 'interface') return t('settings_page.interface_preferences')
  if (currentSection.value === 'database') return t('settings_page.database_preferences')
  return t('settings_page.editor_preferences')
})
const currentSectionDescription = computed(() => {
  if (currentSection.value === 'interface') return t('settings_page.interface_description')
  if (currentSection.value === 'database') return t('settings_page.database_description')
  return t('settings_page.editor_description')
})
const currentThemeModeLabel = computed(() => {
  if (appStore.themeMode === 'dark') return t('settings_page.theme_dark')
  if (appStore.themeMode === 'system') return t('settings_page.theme_system')
  return t('settings_page.theme_light')
})

const themeModeModel = computed({
  get: () => appStore.themeMode,
  set: (value: ThemeMode) => appStore.setThemeMode(value),
})

const languageModel = computed({
  get: () => appStore.language,
  set: (value: Language) => appStore.setLanguage(value),
})

const logLevelModel = computed({
  get: () => appStore.logLevel,
  set: (value: LogLevel) => appStore.setLogLevel(value),
})

const fontSizeModel = computed({
  get: () => appStore.editorSettings.fontSize,
  set: (value: number) => appStore.setEditorFontSize(value),
})

const interfaceFontModel = computed({
  get: () => appStore.interfaceSettings.fontFamily,
  set: (value: string) => appStore.setInterfaceFontFamily(value),
})

const editorFontModel = computed({
  get: () => appStore.editorSettings.fontFamily,
  set: (value: string) => appStore.setEditorFontFamily(value),
})

const minimapModel = computed({
  get: () => appStore.editorSettings.minimap,
  set: (value: boolean) => appStore.setEditorMinimap(value),
})

const lineNumbersEnabled = computed({
  get: () => appStore.editorSettings.lineNumbers === 'on',
  set: (value: boolean) => appStore.setEditorLineNumbers(value ? 'on' : 'off'),
})

const mysqlCharsetModel = computed({
  get: () => appStore.databaseSettings.mysqlCharset,
  set: (value: string) => appStore.setMysqlCharset(value),
})

const mysqlInitSqlModel = computed({
  get: () => appStore.databaseSettings.mysqlInitSql,
  set: (value: string) => appStore.setMysqlInitSql(value),
})

const interfaceFontLabel = computed(() => interfaceFontOptions.find(option => option.value === interfaceFontModel.value)?.label || t('settings_page.custom_font'))
const editorFontLabel = computed(() => editorFontOptions.find(option => option.value === editorFontModel.value)?.label || t('settings_page.custom_font'))
const previewSql = computed(() => `SELECT id, name, status\nFROM users\nWHERE status = 'active'\nORDER BY created_at DESC\nLIMIT 50;`)
const databasePreviewSql = computed(() => `SET NAMES ${mysqlCharsetModel.value};\n${mysqlInitSqlModel.value || t('settings_page.database_preview_fallback')}`)

function goHome() {
  router.push({ name: 'Home' })
}

function noop() {}
</script>

<style scoped>
.settings-layout { min-height: 100vh; }

.settings-content {
  height: calc(100vh - 40px);
  overflow: hidden;
  background:
    radial-gradient(circle at top left, rgba(24, 144, 255, 0.12), transparent 35%),
    linear-gradient(180deg, #f5f7fa 0%, #edf1f5 100%);
}

.dark-mode .settings-content {
  background:
    radial-gradient(circle at top left, rgba(24, 144, 255, 0.16), transparent 35%),
    linear-gradient(180deg, #111827 0%, #0f172a 100%);
}

.settings-shell {
  display: flex;
  height: 100%;
  gap: 20px;
  padding: 20px;
  box-sizing: border-box;
}

.settings-sidebar {
  width: 240px;
  flex-shrink: 0;
  border: 1px solid rgba(15, 23, 42, 0.08);
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.82);
  backdrop-filter: blur(14px);
  padding: 16px;
}

.dark-mode .settings-sidebar {
  background: rgba(15, 23, 42, 0.72);
  border-color: rgba(148, 163, 184, 0.18);
}

.settings-sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.settings-sidebar-header h2 {
  margin: 0;
  font-size: 18px;
}

.settings-menu {
  border-inline-end: none;
  background: transparent;
}

.settings-main {
  flex: 1;
  min-width: 0;
  overflow: auto;
}

.settings-hero {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}

.hero-eyebrow {
  margin: 0 0 6px;
  font-size: 12px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: #2563eb;
  font-weight: 700;
}

.settings-hero h1 {
  margin: 0;
  font-size: 28px;
  line-height: 1.1;
}

.hero-description {
  margin: 8px 0 0;
  color: #64748b;
  max-width: 720px;
}

.hero-status {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.settings-card {
  border-radius: 18px;
  border: 1px solid rgba(15, 23, 42, 0.08);
  background: rgba(255, 255, 255, 0.88);
  box-shadow: 0 18px 48px rgba(15, 23, 42, 0.08);
  margin-bottom: 16px;
}

.dark-mode .settings-card {
  background: rgba(15, 23, 42, 0.78);
  border-color: rgba(148, 163, 184, 0.18);
  box-shadow: none;
}

.secondary-card {
  background: rgba(255, 255, 255, 0.74);
}

.dark-mode .secondary-card {
  background: rgba(15, 23, 42, 0.62);
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 18px 0;
  border-bottom: 1px solid rgba(15, 23, 42, 0.08);
}

.setting-row:last-child {
  border-bottom: none;
}

.dark-mode .setting-row {
  border-bottom-color: rgba(148, 163, 184, 0.16);
}

.setting-meta {
  flex: 1;
  min-width: 0;
}

.setting-label {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 4px;
}

.setting-help {
  color: #64748b;
  font-size: 13px;
}

.dark-mode .setting-help {
  color: #94a3b8;
}

.preview-surface,
.editor-preview {
  border: 1px solid rgba(15, 23, 42, 0.08);
  border-radius: 14px;
  background: linear-gradient(180deg, rgba(248, 250, 252, 0.9), rgba(241, 245, 249, 0.9));
  padding: 16px;
}

.dark-mode .preview-surface,
.dark-mode .editor-preview {
  background: linear-gradient(180deg, rgba(30, 41, 59, 0.9), rgba(15, 23, 42, 0.9));
  border-color: rgba(148, 163, 184, 0.16);
}

.preview-toolbar,
.editor-preview-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 14px;
}

.preview-body {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.preview-panel {
  padding: 14px;
  border-radius: 12px;
  background: rgba(255, 255, 255, 0.72);
  border: 1px solid rgba(15, 23, 42, 0.06);
}

.dark-mode .preview-panel {
  background: rgba(15, 23, 42, 0.72);
  border-color: rgba(148, 163, 184, 0.12);
}

.preview-panel p {
  margin: 6px 0 0;
  color: #64748b;
  font-size: 13px;
}

.dark-mode .preview-panel p {
  color: #94a3b8;
}

.editor-preview-code {
  margin: 0;
  padding: 14px;
  border-radius: 12px;
  background: rgba(15, 23, 42, 0.92);
  color: #e2e8f0;
  overflow: auto;
  font-size: 13px;
  line-height: 1.6;
}

@media (max-width: 900px) {
  .settings-shell {
    flex-direction: column;
  }

  .settings-sidebar {
    width: auto;
  }

  .setting-row {
    flex-direction: column;
    align-items: stretch;
  }

  .settings-hero,
  .preview-body {
    grid-template-columns: 1fr;
    flex-direction: column;
  }
}
</style>
