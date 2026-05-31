import { createRouter, createWebHashHistory } from 'vue-router'
import HomePage from '../pages/HomePage.vue'
import SearchPage from '../pages/SearchPage.vue'
import SettingsPage from '../pages/SettingsPage.vue'
import IndexPage from '../pages/IndexPage.vue'

const routes = [
  {
    path: '/',
    redirect: '/home',
  },
  {
    path: '/home',
    name: 'Home',
    component: HomePage,
  },
  {
    path: '/search',
    name: 'Search',
    component: SearchPage,
  },
  {
    path: '/index',
    name: 'Index',
    component: IndexPage,
  },
  {
    path: '/settings',
    name: 'Settings',
    component: SettingsPage,
  },
]

const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

export default router
