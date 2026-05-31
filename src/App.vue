<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { RouterView, useRouter, useRoute } from 'vue-router'
import { server, filesystem } from '@neutralinojs/lib'
import type { MenuOption } from 'naive-ui'
import { useConfig } from './composables/useConfig'

const router = useRouter()
const route = useRoute()

const menuOptions: MenuOption[] = [
  {
    label: '🏠 首页',
    key: 'home',
  },
  {
    label: '🔍 搜索',
    key: 'search',
  },
  {
    label: '📂 索引管理',
    key: 'index',
  },
  {
    label: '⚙️ 设置',
    key: 'settings',
  },
]

const activeKey = ref<string>('search')

watch(
  () => route.path,
  (path) => {
    if (path === '/home') {
      activeKey.value = 'home'
    } else if (path.startsWith('/search')) {
      activeKey.value = 'search'
    } else if (path.startsWith('/index')) {
      activeKey.value = 'index'
    } else if (path.startsWith('/settings')) {
      activeKey.value = 'settings'
    } else {
      activeKey.value = 'search'
    }
  },
  { immediate: true }
)

function handleMenuUpdate(key: string) {
  router.push(`/${key}`)
}

const { load: loadConfig } = useConfig()
loadConfig()

onMounted(async () => {
  if (typeof window !== 'undefined' && window.NL_PORT) {
    const thumbDir = window.NL_PATH + '/data/thumbnails'
    try {
      await filesystem.createDirectory(thumbDir)
    } catch {
      // 目录已存在，忽略
    }
    try {
      await server.mount('/thumbnails', thumbDir)
    } catch (e) {
      console.warn('挂载缩略图目录失败:', e)
    }
  }
})
</script>

<template>
  <n-config-provider>
    <n-layout class="app-layout">
      <n-layout-header bordered>
        <div class="header-bar">
          <n-menu
            v-model:value="activeKey"
            :options="menuOptions"
            mode="horizontal"
            class="nav-menu"
            @update:value="handleMenuUpdate"
          />
        </div>
      </n-layout-header>
      <n-layout-content class="content">
        <n-message-provider>
          <RouterView v-slot="{ Component }">
            <Transition name="page" mode="out-in">
              <component :is="Component" />
            </Transition>
          </RouterView>
        </n-message-provider>
      </n-layout-content>
    </n-layout>
  </n-config-provider>
</template>
<style scoped>
.app-layout {
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: clip;
}

.header-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 56px;
  padding: 0 20px;
  max-width: 100%;
}


.nav-menu {
  flex-shrink: 0;
}
/* ---- 页面切换动画 ---- */
.page-enter-active,
.page-leave-active {
  transition: opacity 0.12s ease;
}
.page-enter-from,
.page-leave-to {
  opacity: 0;
}

/* ---- 导航栏悬浮动画 ---- */
.nav-menu :deep(.n-menu-item) {
  transition: transform 0.1s ease;
}
.nav-menu :deep(.n-menu-item:hover) {
  transform: translateY(-1px);
}

.content {
  flex: 1;
  overflow: hidden;
}
</style>