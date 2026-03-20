<template>
  <a-config-provider :theme="themeConfig">
    <div id="app" :class="{ 'dark-mode': isDark }">
      <router-view />
    </div>
  </a-config-provider>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'
import { theme as antTheme } from 'ant-design-vue'
import { useAppStore } from '@/stores/app'
import { VxeUI } from 'vxe-pc-ui'

const appStore = useAppStore()
const isDark = computed(() => appStore.theme === 'dark')

const themeConfig = computed(() => ({
  algorithm: isDark.value ? antTheme.darkAlgorithm : antTheme.defaultAlgorithm,
}))

// 同步 vxe-table 主题
watch(() => appStore.theme, (val) => {
  VxeUI.setTheme(val === 'dark' ? 'dark' : 'light')
}, { immediate: true })
</script>

<style>
#app {
  width: 100%;
  height: 100vh;
  overflow: hidden;
}

.dark-mode {
  background-color: #141414;
  color: rgba(255, 255, 255, 0.85);
}
</style>

