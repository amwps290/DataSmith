<template>
  <div class="settings-editor" :class="{ embedded }">
    <aside class="settings-nav">
      <div class="settings-nav-title">{{ $t('common.settings') }}</div>
      <button type="button" class="settings-nav-item" :class="{ active: currentSection === 'interface' }" @click="selectedKeys = ['interface']">
        {{ $t('settings_page.interface_title') }}
      </button>
      <button type="button" class="settings-nav-item" :class="{ active: currentSection === 'editor' }" @click="selectedKeys = ['editor']">
        {{ $t('settings_page.editor_title') }}
      </button>
      <button type="button" class="settings-nav-item" :class="{ active: currentSection === 'database' }" @click="selectedKeys = ['database']">
        {{ $t('settings_page.database_title') }}
      </button>
    </aside>

    <main class="settings-main">
      <div class="settings-header">
        <div class="settings-heading">
          <h1>{{ currentSectionTitle }}</h1>
          <p>{{ currentSectionDescription }}</p>
          <div class="settings-subtle">{{ $t('settings_page.instant_save_hint') }}</div>
        </div>
      </div>

      <template v-if="currentSection === 'interface'">
        <section class="settings-group">
          <div class="settings-group-title">{{ $t('settings_page.appearance_group') }}</div>
          <div class="setting-row">
            <div class="setting-meta">
              <div class="setting-label">{{ $t('common.theme') }}</div>
              <div class="setting-help">{{ $t('settings_page.theme_help') }}</div>
            </div>
            <a-select v-model:value="themeModeModel" class="setting-select compact">
              <a-select-option value="light">{{ $t('settings_page.theme_light') }}</a-select-option>
              <a-select-option value="dark">{{ $t('settings_page.theme_dark') }}</a-select-option>
              <a-select-option value="system">{{ $t('settings_page.theme_system') }}</a-select-option>
            </a-select>
          </div>

          <div class="setting-row">
            <div class="setting-meta">
              <div class="setting-label">{{ $t('settings_page.current_theme') }}</div>
              <div class="setting-help">{{ $t('settings_page.current_theme_help') }}</div>
            </div>
            <a-input :value="appStore.theme === 'dark' ? $t('settings_page.theme_dark') : $t('settings_page.theme_light')" class="setting-select compact" readonly />
          </div>

          <div class="setting-row">
            <div class="setting-meta">
              <div class="setting-label">{{ $t('settings_page.interface_font') }}</div>
              <div class="setting-help">{{ $t('settings_page.interface_font_help') }}</div>
            </div>
            <a-select v-model:value="interfaceFontModel" class="setting-select" :options="interfaceFontOptions" />
          </div>

          <div class="setting-row">
            <div class="setting-meta">
              <div class="setting-label">{{ $t('common.language') }}</div>
              <div class="setting-help">{{ $t('settings_page.language_help') }}</div>
            </div>
            <a-select v-model:value="languageModel" class="setting-select compact">
              <a-select-option value="zh-CN">中文</a-select-option>
              <a-select-option value="en-US">English</a-select-option>
            </a-select>
          </div>

          <div class="setting-row">
            <div class="setting-meta">
              <div class="setting-label">{{ $t('settings_page.log_level') }}</div>
              <div class="setting-help">{{ $t('settings_page.log_level_help') }}</div>
            </div>
            <a-select v-model:value="logLevelModel" class="setting-select compact" :options="logLevelOptions" />
          </div>
        </section>
      </template>

      <template v-else-if="currentSection === 'editor'">
        <section class="settings-group">
          <div class="settings-group-title">{{ $t('settings_page.editor_typography_group') }}</div>
          <div class="setting-row">
            <div class="setting-meta">
              <div class="setting-label">{{ $t('settings_page.editor_font') }}</div>
              <div class="setting-help">{{ $t('settings_page.editor_font_help') }}</div>
            </div>
            <a-select v-model:value="editorFontModel" class="setting-select" :options="editorFontOptions" />
          </div>

          <div class="setting-row">
            <div class="setting-meta">
              <div class="setting-label">{{ $t('settings_page.font_size') }}</div>
              <div class="setting-help">{{ $t('settings_page.font_size_help') }}</div>
            </div>
            <a-input-number v-model:value="fontSizeModel" :min="12" :max="24" class="setting-select compact" />
          </div>

          <div class="setting-row">
            <div class="setting-meta">
              <div class="setting-label">{{ $t('settings_page.line_numbers') }}</div>
              <div class="setting-help">{{ $t('settings_page.line_numbers_help') }}</div>
            </div>
            <a-select v-model:value="lineNumbersModel" class="setting-select compact">
              <a-select-option value="on">{{ $t('settings_page.option_on') }}</a-select-option>
              <a-select-option value="off">{{ $t('settings_page.option_off') }}</a-select-option>
            </a-select>
          </div>

          <div class="setting-row">
            <div class="setting-meta">
              <div class="setting-label">{{ $t('settings_page.minimap') }}</div>
              <div class="setting-help">{{ $t('settings_page.minimap_help') }}</div>
            </div>
            <a-select v-model:value="minimapModeModel" class="setting-select compact">
              <a-select-option value="on">{{ $t('settings_page.option_on') }}</a-select-option>
              <a-select-option value="off">{{ $t('settings_page.option_off') }}</a-select-option>
            </a-select>
          </div>
        </section>
      </template>

      <template v-else>
        <section class="settings-group">
          <div class="settings-group-title">{{ $t('settings_page.database_connection_group') }}</div>
          <a-alert
            type="info"
            show-icon
            class="settings-note"
            :message="$t('settings_page.database_defaults_scope')"
          />

          <div class="setting-row">
            <div class="setting-meta">
              <div class="setting-label">{{ $t('settings_page.mysql_charset') }}</div>
              <div class="setting-help">{{ $t('settings_page.mysql_charset_help') }}</div>
            </div>
            <a-select v-model:value="mysqlCharsetModel" class="setting-select compact">
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
            <a-textarea
              v-model:value="mysqlInitSqlModel"
              :rows="4"
              class="setting-textarea"
              :placeholder="$t('settings_page.mysql_init_sql_placeholder')"
            />
          </div>
        </section>
      </template>
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore, type Language, type LogLevel, type ThemeMode } from '@/stores/app'

withDefaults(defineProps<{
  embedded?: boolean
}>(), {
  embedded: false,
})

defineEmits<{
  close: []
}>()

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
  set: (value: number | null) => appStore.setEditorFontSize(Number(value || 14)),
})
const interfaceFontModel = computed({
  get: () => appStore.interfaceSettings.fontFamily,
  set: (value: string) => appStore.setInterfaceFontFamily(value),
})
const editorFontModel = computed({
  get: () => appStore.editorSettings.fontFamily,
  set: (value: string) => appStore.setEditorFontFamily(value),
})
const minimapModeModel = computed({
  get: () => appStore.editorSettings.minimap ? 'on' : 'off',
  set: (value: string) => appStore.setEditorMinimap(value === 'on'),
})
const lineNumbersModel = computed({
  get: () => appStore.editorSettings.lineNumbers,
  set: (value: 'on' | 'off') => appStore.setEditorLineNumbers(value),
})
const mysqlCharsetModel = computed({
  get: () => appStore.databaseSettings.mysqlCharset,
  set: (value: string) => appStore.setMysqlCharset(value),
})
const mysqlInitSqlModel = computed({
  get: () => appStore.databaseSettings.mysqlInitSql,
  set: (value: string) => appStore.setMysqlInitSql(value),
})
</script>

<style scoped>
.settings-editor {
  display: flex;
  height: 100%;
  min-height: 0;
  background: #ffffff;
  color: #262626;
}

.dark-mode .settings-editor {
  background: #1e1e1e;
  color: #e5e7eb;
}

.settings-editor.embedded {
  border-top: 1px solid #e5e7eb;
}

.dark-mode .settings-editor.embedded {
  border-top-color: #303030;
}

.settings-nav {
  width: 188px;
  flex-shrink: 0;
  padding: 8px 8px;
  border-right: 1px solid #e5e7eb;
  background: transparent;
}

.dark-mode .settings-nav {
  border-right-color: #303030;
}

.settings-nav-title {
  margin-bottom: 8px;
  padding: 0 6px;
  font-size: 12px;
  font-weight: 700;
  color: #6b7280;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.dark-mode .settings-nav-title {
  color: #9ca3af;
}

.settings-nav-item {
  width: 100%;
  padding: 6px 8px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: #374151;
  font-size: 13px;
  text-align: left;
  cursor: pointer;
}

.settings-nav-item + .settings-nav-item {
  margin-top: 2px;
}

.settings-nav-item:hover {
  background: rgba(59, 130, 246, 0.08);
}

.settings-nav-item.active {
  background: rgba(59, 130, 246, 0.12);
  color: #1d4ed8;
}

.dark-mode .settings-nav-item {
  color: #d1d5db;
}

.dark-mode .settings-nav-item:hover {
  background: rgba(96, 165, 250, 0.12);
}

.dark-mode .settings-nav-item.active {
  background: rgba(96, 165, 250, 0.14);
  color: #93c5fd;
}

.settings-main {
  flex: 1;
  min-width: 0;
  overflow: auto;
  padding: 10px 16px 16px;
  background: transparent;
}

.settings-header {
  margin-bottom: 10px;
  padding-bottom: 8px;
  border-bottom: 1px solid #ececec;
}

.dark-mode .settings-header {
  border-bottom-color: #2c2c2c;
}

.settings-heading h1 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}

.settings-heading p {
  margin: 4px 0 0;
  color: #6b7280;
  font-size: 12px;
  line-height: 1.5;
}

.dark-mode .settings-heading p {
  color: #9ca3af;
}

.settings-subtle {
  margin-top: 6px;
  color: #2563eb;
  font-size: 11px;
}

.settings-group {
  margin-bottom: 10px;
  border: 0;
  border-radius: 0;
  overflow: visible;
  background: transparent;
}

.dark-mode .settings-group {
  background: transparent;
}

.settings-group-title {
  padding: 4px 0 6px;
  border-bottom: 1px solid #ececec;
  background: transparent;
  font-size: 12px;
  font-weight: 600;
  color: #6b7280;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.dark-mode .settings-group-title {
  border-bottom-color: #2c2c2c;
  color: #9ca3af;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 10px 0;
  border-bottom: 1px solid #f2f2f2;
}

.setting-row:last-child {
  border-bottom: none;
}

.dark-mode .setting-row {
  border-bottom-color: #2a2a2a;
}

.setting-meta {
  flex: 1;
  min-width: 0;
}

.setting-label {
  margin-bottom: 2px;
  font-size: 13px;
  font-weight: 500;
}

.setting-help {
  color: #6b7280;
  font-size: 11px;
  line-height: 1.4;
}

.dark-mode .setting-help {
  color: #9ca3af;
}

.setting-select {
  width: 240px;
}

.setting-select.compact {
  width: 180px;
}

.setting-textarea {
  width: 320px;
}

.settings-note {
  margin: 16px;
  margin-bottom: 0;
}

@media (max-width: 900px) {
  .settings-editor {
    flex-direction: column;
  }

  .settings-nav {
    width: 100%;
    border-right: 0;
    border-bottom: 1px solid #e5e7eb;
  }

  .dark-mode .settings-nav {
    border-bottom-color: #303030;
  }

  .settings-header,
  .setting-row {
    flex-direction: column;
    align-items: stretch;
  }

  .setting-select,
  .setting-select.compact,
  .setting-textarea {
    width: 100%;
  }
}
</style>
