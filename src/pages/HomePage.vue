<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { app, os } from '@neutralinojs/lib'
const appVersion = ref('1.0.0')
const neutralinoReady = ref(false)
const currentDirectory = ref('')
const environment = ref('检测中...')

onMounted(async () => {
  if (typeof window !== 'undefined' && window.NL_PORT) {
    environment.value = 'Neutralinojs 桌面环境'
    try {
      const config = await app.getConfig()
      appVersion.value = config?.version || window.NL_APPVERSION || '1.0.0'
      neutralinoReady.value = true
      
      try {
        const entries = await os.execCommand('cd')
        currentDirectory.value = entries.stdOut.trim()
      } catch (err) {
        currentDirectory.value = '无法获取当前目录'
      }
      
      console.log('Neutralino is ready, config:', config)
    } catch (error) {
      console.log('Neutralino API 调用失败:', error)
      neutralinoReady.value = true
    }

  } else {
    environment.value = '浏览器环境'
    currentDirectory.value = window.location.origin
    neutralinoReady.value = true
  }
})
</script>

<template>
  <n-card title="Local Image Search" class="app-card">
    <n-space vertical>
      <n-space>
        <n-tag type="success">Neutralinojs</n-tag>
        <n-tag>Version: {{ appVersion }}</n-tag>
      </n-space>
      
      <n-statistic label="运行环境" :value="environment" />
      
      <n-statistic label="工作目录" :value="currentDirectory || 'Loading...'" />
    </n-space>
  </n-card>
</template>

<style scoped>
.app-card {
  max-width: 600px;
  margin: 60px auto;
}
</style>