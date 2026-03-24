<template>
  <div class="global-sql-toolbar">
    <div class="toolbar-left">
      <a-space :size="8">
        <a-button-group>
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
        </a-button-group>
        <a-divider type="vertical" />
        <a-button-group>
          <a-tooltip :title="`${$t('common.save')} (Ctrl+S)`">
            <a-button type="text" size="small" @click="$emit('action', 'handleSave')"><template #icon><SaveOutlined /></template></a-button>
          </a-tooltip>
          <a-tooltip :title="$t('common.format')">
            <a-button type="text" size="small" @click="$emit('action', 'formatSql')"><template #icon><FormatPainterOutlined /></template></a-button>
          </a-tooltip>
          <a-tooltip :title="$t('common.clear')">
            <a-button type="text" size="small" @click="$emit('action', 'clearEditor')"><template #icon><ClearOutlined /></template></a-button>
          </a-tooltip>
        </a-button-group>
        <a-divider type="vertical" />
        <a-button-group>
          <a-tooltip :title="$t('common.history')">
            <a-button type="text" size="small" @click="$emit('action', 'openHistory')"><template #icon><HistoryOutlined /></template></a-button>
          </a-tooltip>
          <a-tooltip :title="$t('common.snippets')">
            <a-button type="text" size="small" @click="$emit('action', 'openSnippets')"><template #icon><CodeOutlined /></template></a-button>
          </a-tooltip>
          <a-tooltip :title="$t('common.refresh')">
            <a-button type="text" size="small" @click="$emit('action', 'refreshAutocomplete')"><template #icon><SyncOutlined /></template></a-button>
          </a-tooltip>
        </a-button-group>
      </a-space>
    </div>
    <div class="toolbar-right">
      <a-space>
        <span class="db-label">{{ $t('common.database') }}:</span>
        <a-select
          :value="selectedDatabase"
          :placeholder="$t('common.database')"
          size="small"
          style="width: 160px"
          @change="(val: any) => $emit('databaseChange', String(val ?? ''))"
        >
          <a-select-option value="">{{ appStore.language === 'zh-CN' ? '默认' : 'Default' }}</a-select-option>
          <a-select-option v-for="db in databases" :key="db.name" :value="db.name">{{ db.name }}</a-select-option>
        </a-select>
      </a-space>
    </div>
  </div>
</template>

<script setup lang="ts">
import {
  PlayCircleFilled, StopOutlined, SaveOutlined,
  FormatPainterOutlined, ClearOutlined, HistoryOutlined, CodeOutlined, SyncOutlined, SearchOutlined
} from '@ant-design/icons-vue'
import { useAppStore } from '@/stores/app'
import type { DatabaseInfo } from '@/types/database'

defineProps<{
  executing: boolean
  selectedDatabase: string
  databases: DatabaseInfo[]
}>()

defineEmits<{
  action: [method: string]
  databaseChange: [value: string]
}>()

const appStore = useAppStore()
</script>

<style scoped>
.global-sql-toolbar { display: flex; justify-content: space-between; align-items: center; padding: 0 12px; height: 40px; background: #f5f5f5; border-bottom: 1px solid #d9d9d9; flex-shrink: 0; }
.dark-mode .global-sql-toolbar { background: #1a1a1a; border-bottom-color: #303030; }
.toolbar-left :deep(.ant-btn-text) { width: auto; min-width: 32px; padding: 0 8px; height: 32px; display: flex; align-items: center; justify-content: center; border-radius: 4px; color: #595959; font-size: 14px; }
.dark-mode .toolbar-left :deep(.ant-btn-text) { color: #aaa; }
.toolbar-left :deep(.ant-btn-text:hover) { background: rgba(0,0,0,0.06); color: #1890ff; }
.btn-run { color: #52c41a !important; font-weight: bold; }
.btn-run:hover { background: #f6ffed !important; }
.btn-stop { color: #ff4d4f !important; }
.btn-stop:hover { background: #fff1f0 !important; }
.db-label { font-size: 12px; color: #8c8c8c; margin-right: 8px; }
</style>
