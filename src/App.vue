<template>
  <a-config-provider :theme="themeConfig">
    <div id="app" :class="{ 'dark-mode': isDark }" :style="appStyleVars">
      <router-view v-slot="{ Component, route }">
        <transition name="page-shell" mode="out-in">
          <keep-alive v-if="route.meta.keepAlive">
            <component :is="Component" :key="String(route.name || route.path)" />
          </keep-alive>
          <component v-else :is="Component" :key="String(route.name || route.path)" />
        </transition>
      </router-view>
    </div>
  </a-config-provider>
</template>

<script setup lang="ts">
import { computed, watch, onMounted } from 'vue'
import { theme as antTheme } from 'ant-design-vue'
import { useAppStore } from '@/stores/app'
import { VxeUI } from 'vxe-pc-ui'

const appStore = useAppStore()
const isDark = computed(() => appStore.theme === 'dark')

const themeConfig = computed(() => ({
  algorithm: isDark.value ? antTheme.darkAlgorithm : antTheme.defaultAlgorithm,
}))

const appStyleVars = computed(() => ({
  '--app-font-family': appStore.interfaceSettings.fontFamily,
}))

// 在组件挂载后再开始监听 Store 变化，确保 Pinia 已完全激活
onMounted(() => {
  watch(() => appStore.theme, (val) => {
    VxeUI.setTheme(val === 'dark' ? 'dark' : 'light')
  }, { immediate: true })
})
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

.page-shell-enter-active,
.page-shell-leave-active {
  transition: opacity 0.24s ease, transform 0.24s ease;
}

.page-shell-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.page-shell-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
