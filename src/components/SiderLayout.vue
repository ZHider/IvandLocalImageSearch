<script setup lang="ts">
import type { MenuOption } from 'naive-ui'

defineProps<{
  activeTab: string
  menuOptions: MenuOption[]
  menuDisabled?: boolean
  contentMaxWidth?: number
}>()

const emit = defineEmits<{
  'update:activeTab': [value: string]
}>()
</script>

<template>
  <div class="sider-container">
    <n-layout class="sider-layout" has-sider>
      <n-layout-sider
        bordered
        content-style="padding: 0;"
        width="200"
        :native-scrollbar="false"
      >
        <n-menu
          :value="activeTab"
          @update:value="(v: string) => emit('update:activeTab', v)"
          :options="menuOptions"
          :collapsed="false"
          :disabled="menuDisabled"
          :class="{ 'side-menu--gray': menuDisabled }"
        />
      </n-layout-sider>
      <n-layout-content class="sider-content" :native-scrollbar="false">
        <div class="sider-body">
          <Transition name="slide" mode="out-in">
            <div
              :key="activeTab"
              class="sider-section"
              :style="{ maxWidth: contentMaxWidth ? contentMaxWidth + 'px' : undefined }"
            >
              <slot />
            </div>
          </Transition>
        </div>
      </n-layout-content>
    </n-layout>
  </div>
</template>


<style scoped>
.slide-enter-active,
.slide-leave-active {
  transition: all 0.1s ease;
}

.slide-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.slide-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

.sider-container {
  height: 100%;
}

.sider-layout {
  height: 100%;
}

.sider-content {
  padding: 0;
  display: flex;
  flex-direction: column;
}

.sider-body {
  flex: 1;
  padding: 20px 24px;
  overflow-y: auto;
}

/* 索引进行时，子侧边栏菜单灰色禁用 */
.side-menu--gray {
  opacity: 0.4;
  pointer-events: none;
  transition: opacity 0.25s;
}
</style>