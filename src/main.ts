import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import { init, server } from '@neutralinojs/lib'
import NaiveUI from 'naive-ui'
import router from './router'

if (window?.NL_PORT) {
  init()
  // 挂载 data 目录，使缩略图可通过 HTTP 访问
  const nlPath = (window as any).NL_PATH as string | undefined
  if (nlPath) {
    server.mount('/data', nlPath + '/data').catch(() => {
      // 已挂载时忽略
    })
  }
}

const app = createApp(App)
app.use(NaiveUI)
app.use(router)
app.mount('#app')