<script setup lang="ts">
import { ref, watch } from 'vue'
import { RouterView, useRouter, useRoute } from 'vue-router'
import type { MenuOption } from 'naive-ui'

const router = useRouter()
const route = useRoute()

const menuOptions: MenuOption[] = [
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
    if (path.startsWith('/search')) {
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
</script>

<template>
  <n-config-provider>
    <n-layout class="app-layout">
      <n-layout-header bordered>
        <div class="header-bar">
          <n-space align="center" :size="8">
            <span class="app-logo">🔍</span>
            <span class="app-title">本地智能图片搜索</span>
          </n-space>
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
          <RouterView />
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
}

.header-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 56px;
  padding: 0 20px;
  max-width: 100%;
}

.app-logo {
  font-size: 24px;
}

.app-title {
  font-size: 16px;
  font-weight: 600;
  white-space: nowrap;
}

.nav-menu {
  flex-shrink: 0;
}

.content {
  flex: 1;
  overflow: hidden;
}
</style>