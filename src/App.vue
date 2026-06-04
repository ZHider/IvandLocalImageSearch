<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, provide } from 'vue'
import { RouterView, useRouter, useRoute } from 'vue-router'
import type { MenuOption } from 'naive-ui'
import { useExtension } from './composables/useExtension'

const router = useRouter()
const route = useRoute()
const { on } = useExtension()

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
const indexing = ref(false)

// 提供给子组件（如 IndexPage）使用，确保索引状态全局同步
provide('indexing', indexing)

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
  if (indexing.value) return
  router.push(`/${key}`)
}

const cleanups: (() => void)[] = []

onMounted(() => {
  cleanups.push(
    on('indexProgress', () => {
      indexing.value = true
    }),
    on('indexComplete', () => {
      indexing.value = false
    }),
    on('indexError', () => {
      indexing.value = false
    }),
  )
})

onUnmounted(() => {
  cleanups.forEach(stop => stop())
  indexing.value = false
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
            :disabled="indexing"
            @update:value="handleMenuUpdate"
          />
          <n-tag v-if="indexing" type="warning" size="small" class="indexing-tag">
            🔒 索引进行中
          </n-tag>
        </div>
      </n-layout-header>
      <n-layout-content class="content">
        <n-message-provider>
          <RouterView v-slot="{ Component }">
            <Transition name="fade" mode="out-in">
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
  transition: opacity 0.2s;
}

.nav-menu:deep(.n-menu--disabled) {
  opacity: 0.45;
  cursor: not-allowed;
}

.nav-menu:deep(.n-menu--disabled .n-menu-item-content) {
  cursor: not-allowed !important;
}


.indexing-tag {
  flex-shrink: 0;
  margin-left: 12px;
}

.content {
  flex: 1;
  overflow: hidden;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.1s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>