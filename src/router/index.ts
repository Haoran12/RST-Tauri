import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      redirect: '/library',
    },
    {
      path: '/library',
      name: 'library',
      component: () => import('@/views/LibraryView.vue'),
      meta: { title: '资源工作台' },
    },
    {
      path: '/chat/st/:sessionId?',
      name: 'st-chat',
      component: () => import('@/views/STChatView.vue'),
      meta: { title: 'ST 聊天' },
    },
    {
      path: '/agent/worlds/:worldId?',
      name: 'agent-worlds',
      component: () => import('@/views/AgentWorldView.vue'),
      meta: { title: 'Agent 工作区' },
    },
    {
      path: '/agent/worlds/:worldId/editor',
      name: 'agent-world-editor',
      component: () => import('@/views/AgentWorldEditorView.vue'),
      meta: { title: 'Agent World Editor' },
    },
    {
      path: '/resources/characters',
      name: 'resources-characters',
      component: () => import('@/views/ResourcesCharactersView.vue'),
      meta: { title: '角色卡' },
    },
    {
      path: '/resources/worldbooks',
      name: 'resources-worldbooks',
      component: () => import('@/views/ResourcesWorldbooksView.vue'),
      meta: { title: '世界书' },
    },
    {
      path: '/resources/presets',
      name: 'resources-presets',
      component: () => import('@/views/ResourcesPresetsView.vue'),
      meta: { title: '预设' },
    },
    {
      path: '/resources/regex',
      name: 'resources-regex',
      component: () => import('@/views/ResourcesRegexView.vue'),
      meta: { title: 'Regex' },
    },
    {
      path: '/api-configs',
      name: 'api-configs',
      component: () => import('@/views/ApiConfigsView.vue'),
      meta: { title: 'API 配置' },
    },
    {
      path: '/logs',
      name: 'logs',
      component: () => import('@/views/LogsView.vue'),
      meta: { title: '日志' },
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/views/SettingsView.vue'),
      meta: { title: '设置' },
    },
  ],
})

export default router
