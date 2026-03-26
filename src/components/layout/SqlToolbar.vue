<template>
  <div class="global-sql-toolbar" :class="{ vertical }">
    <div class="toolbar-left" :class="{ vertical }">
      <div class="toolbar-group" :class="{ vertical }">
        <a-tooltip :title="`${$t('common.run')} (F5)`">
          <a-button type="text" size="small" @click="$emit('action', 'executeQuery')" :loading="executing" class="btn-run">
            <template #icon><PlayCircleFilled /></template>
          </a-button>
        </a-tooltip>
        <a-tooltip :title="$t('common.explain')">
          <a-button type="text" size="small" @click="$emit('action', 'explainQuery')" :disabled="executing" class="btn-explain">
            <template #icon><SearchOutlined /></template>
          </a-button>
        </a-tooltip>
        <a-tooltip :title="$t('common.stop')">
          <a-button type="text" size="small" @click="$emit('action', 'stopExecution')" :disabled="!executing" class="btn-stop">
            <template #icon><StopOutlined /></template>
          </a-button>
        </a-tooltip>
      </div>
      <a-divider :type="vertical ? 'horizontal' : 'vertical'" />
      <div class="toolbar-group" :class="{ vertical }">
        <a-tooltip :title="`${$t('common.save')} (Ctrl+S)`">
          <a-button type="text" size="small" @click="$emit('action', 'handleSave')"><template #icon><SaveOutlined /></template></a-button>
        </a-tooltip>
        <a-tooltip :title="$t('common.format')">
          <a-button type="text" size="small" @click="$emit('action', 'formatSql')"><template #icon><FormatPainterOutlined /></template></a-button>
        </a-tooltip>
        <a-tooltip :title="$t('common.clear')">
          <a-button type="text" size="small" @click="$emit('action', 'clearEditor')"><template #icon><ClearOutlined /></template></a-button>
        </a-tooltip>
      </div>
      <a-divider :type="vertical ? 'horizontal' : 'vertical'" />
      <div class="toolbar-group" :class="{ vertical }">
        <a-tooltip :title="$t('common.history')">
          <a-button type="text" size="small" @click="$emit('action', 'openHistory')"><template #icon><HistoryOutlined /></template></a-button>
        </a-tooltip>
        <a-tooltip :title="$t('common.snippets')">
          <a-button type="text" size="small" @click="$emit('action', 'openSnippets')"><template #icon><CodeOutlined /></template></a-button>
        </a-tooltip>
        <a-tooltip :title="$t('common.refresh')">
          <a-button type="text" size="small" @click="$emit('action', 'refreshAutocomplete')"><template #icon><SyncOutlined /></template></a-button>
        </a-tooltip>
      </div>
    </div>

    <div class="toolbar-right" :class="{ vertical }">
      <template v-if="vertical">
        <a-dropdown placement="bottomLeft" trigger="click">
          <a-tooltip :title="selectedDatabase || (appStore.language === 'zh-CN' ? '默认数据库' : 'Default Database')">
            <a-button type="text" size="small" class="db-trigger">
              <template #icon><DatabaseOutlined /></template>
            </a-button>
          </a-tooltip>
          <template #overlay>
            <a-menu :selected-keys="[selectedDatabase || '__default__']" @click="handleDatabaseMenuClick">
              <a-menu-item key="__default__">{{ appStore.language === 'zh-CN' ? '默认' : 'Default' }}</a-menu-item>
              <a-menu-item v-for="db in databases" :key="db.name">{{ db.name }}</a-menu-item>
            </a-menu>
          </template>
        </a-dropdown>
      </template>
      <template v-else>
        <a-space :size="12">
          <span class="db-label">{{ $t('common.database') }}:</span>
          <a-select
            :value="selectedDatabase"
            :placeholder="$t('common.database')"
            size="small"
            style="width: 160px"
            @change="(val: unknown) => $emit('databaseChange', String(val ?? ''))"
          >
            <a-select-option value="">{{ appStore.language === 'zh-CN' ? '默认' : 'Default' }}</a-select-option>
            <a-select-option v-for="db in databases" :key="db.name" :value="db.name">{{ db.name }}</a-select-option>
          </a-select>
        </a-space>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import {
  PlayCircleFilled, StopOutlined, SaveOutlined,
  FormatPainterOutlined, ClearOutlined, HistoryOutlined, CodeOutlined, SyncOutlined, SearchOutlined, DatabaseOutlined
} from '@ant-design/icons-vue'
import { useAppStore } from '@/stores/app'
import type { DatabaseInfo } from '@/types/database'

withDefaults(defineProps<{
  executing: boolean
  selectedDatabase: string
  databases: DatabaseInfo[]
  vertical?: boolean
}>(), {
  vertical: false,
})

const emit = defineEmits<{
  action: [method: string]
  databaseChange: [value: string]
}>()

const appStore = useAppStore()

function handleDatabaseMenuClick({ key }: { key: string | number }) {
  emit('databaseChange', String(key) === '__default__' ? '' : String(key))
}
</script>

<style scoped>
.global-sql-toolbar { display: flex; justify-content: space-between; align-items: center; padding: 0 12px; height: 40px; background: #f5f5f5; border-bottom: 1px solid #d9d9d9; flex-shrink: 0; }
.dark-mode .global-sql-toolbar { background: #1a1a1a; border-bottom-color: #303030; }
.global-sql-toolbar.vertical { width: 40px; height: 100%; flex-direction: column; justify-content: flex-start; gap: 4px; padding: 4px 3px; border-right: 1px solid #d9d9d9; border-bottom: 0; background: #f7f7f7; }
.dark-mode .global-sql-toolbar.vertical { background: #181818; border-right-color: #303030; }
.toolbar-left { display: flex; align-items: center; }
.toolbar-left.vertical { flex-direction: column; width: 100%; }
.toolbar-group { display: flex; align-items: center; gap: 4px; }
.toolbar-group.vertical { width: 100%; flex-direction: column; gap: 1px; }
.toolbar-left :deep(.ant-btn-text),
.toolbar-right :deep(.ant-btn-text) { width: auto; min-width: 32px; padding: 0 8px; height: 32px; display: flex; align-items: center; justify-content: center; border-radius: 4px; color: #595959; font-size: 14px; }
.toolbar-left.vertical :deep(.ant-btn-text),
.toolbar-right.vertical :deep(.ant-btn-text) { width: 100%; min-width: 0; height: 24px; padding: 0; border-radius: 3px; font-size: 12px; }
.dark-mode .toolbar-left :deep(.ant-btn-text),
.dark-mode .toolbar-right :deep(.ant-btn-text) { color: #aaa; }
.toolbar-left :deep(.ant-btn-text:hover),
.toolbar-right :deep(.ant-btn-text:hover) { background: rgba(0,0,0,0.06); color: #1890ff; }
.dark-mode .toolbar-left :deep(.ant-btn-text:hover),
.dark-mode .toolbar-right :deep(.ant-btn-text:hover) { background: rgba(255, 255, 255, 0.08); }
.toolbar-right.vertical { width: 100%; margin-top: auto; }
.global-sql-toolbar.vertical :deep(.ant-divider-horizontal) { margin: 4px 0; min-width: 100%; }
.btn-run { color: #52c41a !important; font-weight: bold; }
.btn-run:hover { background: #f6ffed !important; }
.btn-stop { color: #ff4d4f !important; }
.btn-stop:hover { background: #fff1f0 !important; }
.db-label { font-size: 12px; color: #8c8c8c; margin-right: 8px; }
.db-trigger { width: 100%; }
</style>
