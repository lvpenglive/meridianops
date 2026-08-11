import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import zhCn from 'element-plus/es/locale/lang/zh-cn'

import App from './App.vue'
import router from './router'
import { permission as permissionDirective } from './directives/permission'
import './style.css'

const app = createApp(App)

for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component as any)
}

app.use(createPinia())
app.use(router)
app.use(ElementPlus, { locale: zhCn })
// 全局按钮级权限指令：v-permission="'user:create'"
app.directive('permission', permissionDirective)

app.mount('#app')
