<template>
  <header class="header">
    <!-- 专门的拖拽背景层 -->
    <div class="header-drag-handle" data-tauri-drag-region></div>

    <div class="header-content">
      <!-- Logo 区域 (支持拖拽) -->
      <div class="logo" data-tauri-drag-region>
        <DatabaseOutlined style="font-size: 18px; margin-right: 6px" data-tauri-drag-region />
        <span class="title" data-tauri-drag-region>DataSmith</span>
      </div>

      <!-- 菜单区域 (宽度自适应，不阻挡两侧拖拽) -->
      <div class="header-menu">
        <a-menu mode="horizontal" :selected-keys="[]" class="top-menu">
          <a-sub-menu key="file">
            <template #title>{{ $t('common.file') }}</template>
            <a-menu-item key="new-connection" @click="$emit('newConnection')">
              <PlusOutlined /> {{ $t('connection.new') }}
            </a-menu-item>
            <a-menu-divider />
            <a-menu-item key="settings" @click="$emit('openSettings')">
              <SettingOutlined /> {{ $t('common.settings') }}
            </a-menu-item>
          </a-sub-menu>
          <a-sub-menu key="view">
            <template #title>{{ $t('common.view') }}</template>
            <a-menu-item key="toggle-sidebar" @click="appStore.toggleSidebar()">
              <MenuOutlined /> {{ appStore.sidebarCollapsed ? $t('common.show_sidebar') : $t('common.hide_sidebar') }}
            </a-menu-item>
            <a-menu-divider />
            <a-menu-item key="theme" @click="appStore.toggleTheme()">
              <BulbOutlined /> {{ appStore.theme === 'light' ? $t('common.dark_theme') : $t('common.light_theme') }}
            </a-menu-item>
          </a-sub-menu>
        </a-menu>
      </div>

      <!-- 核心：中间大面积可拖拽空白区 -->
      <div class="header-drag-spacer" data-tauri-drag-region></div>

      <!-- 动作区与窗口控制 -->
      <div class="header-actions">
        <a-space :size="0">
          <a-button type="text" size="small" @click="$emit('openSearch')" class="search-btn">
            <template #icon><SearchOutlined /></template>
          </a-button>

          <div class="window-controls">
            <div class="win-btn" title="最小化" @click="minimizeWindow">
              <Icon icon="fluent:subtract-16-filled" />
            </div>
            <div class="win-btn" title="最大化/还原" @click="toggleMaximize">
              <Icon :icon="isMaximized ? 'fluent:square-multiple-16-regular' : 'fluent:square-16-regular'" />
            </div>
            <div class="win-btn close" title="关闭" @click="closeWindow">
              <Icon icon="fluent:dismiss-16-filled" />
            </div>
          </div>
        </a-space>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
import { Icon } from '@iconify/vue'
import {
  DatabaseOutlined, BulbOutlined, PlusOutlined, SettingOutlined,
  MenuOutlined, SearchOutlined,
} from '@ant-design/icons-vue'
import { useAppStore } from '@/stores/app'
import { useWindowControls } from '@/composables/useWindowControls'
import { onMounted } from 'vue'

defineEmits<{
  newConnection: []
  openSettings: []
  openSearch: []
}>()

const appStore = useAppStore()
const { isMaximized, minimizeWindow, toggleMaximize, closeWindow, setupMaximizeListener } = useWindowControls()

onMounted(() => {
  setupMaximizeListener()
})
</script>

<style scoped>
.header {
  height: 40px;
  background: #fff;
  border-bottom: 1px solid #f0f0f0;
  flex-shrink: 0;
  z-index: 100;
  padding: 0;
  overflow: hidden;
  position: relative;
}
.dark-mode .header { background: #1f1f1f; border-bottom-color: #303030; }

.header-drag-handle { position: absolute; top: 0; left: 0; right: 0; bottom: 0; z-index: 1; }

.header-content { display: flex; justify-content: space-between; align-items: center; height: 100%; width: 100%; position: relative; z-index: 2; pointer-events: none; }

.logo { display: flex; align-items: center; font-size: 16px; font-weight: bold; color: #1890ff; padding: 0 16px; height: 100%; pointer-events: auto; }
.header-menu { height: 100%; display: flex; align-items: center; pointer-events: auto; }
.top-menu { border-bottom: none; background: transparent; height: 100%; line-height: 40px; }
.top-menu :deep(.ant-menu-submenu-title) { height: 40px !important; line-height: 40px !important; padding: 0 12px; }

.header-drag-spacer { flex: 1; height: 100%; pointer-events: auto; }

.header-actions { display: flex; align-items: center; height: 100%; pointer-events: auto; padding-right: 0; }
.search-btn { margin-right: 8px; }

.window-controls { display: flex; height: 100%; }
.win-btn { display: inline-flex; justify-content: center; align-items: center; width: 46px; height: 100%; cursor: pointer; transition: background-color 0.2s; font-size: 14px; color: #595959; }
.dark-mode .win-btn { color: #aaa; }
.win-btn:hover { background-color: rgba(0, 0, 0, 0.05); }
.dark-mode .win-btn:hover { background-color: rgba(255, 255, 255, 0.1); }
.win-btn.close:hover { background-color: #e81123 !important; color: #fff !important; }
</style>
