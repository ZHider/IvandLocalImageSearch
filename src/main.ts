import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import { init } from '@neutralinojs/lib'
import NaiveUI from 'naive-ui'
import router from './router'

if (window?.NL_PORT) {
  init()
}

const app = createApp(App)
app.use(NaiveUI)
app.use(router)
app.mount('#app')