import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import { init } from '@neutralinojs/lib'
import NaiveUI from 'naive-ui'

const app = createApp(App)
app.use(NaiveUI)
app.mount('#app')

// 仅在 Neutralinojs 环境下初始化
if (window?.NL_PORT) {
  init()
}
