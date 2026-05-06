import { createRouter, createWebHashHistory } from 'vue-router'
import { useAppShellStore } from '@/stores/appShell'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      redirect: '/mode-select',
    },
    {
      path: '/mode-select',
      name: 'mode-select',
      component: () => import('@/views/ModeSelectView.vue'),
      meta: { title: '模式选择' },
    },
    {
      path: '/st',
      name: 'st-home',
      component: () => import('@/views/STHomeView.vue'),
      meta: { title: 'ST 工作区' },
    },
    {
      path: '/chat/st/:sessionId?',
      redirect: to => {
        const sessionId = Array.isArray(to.params.sessionId) ? to.params.sessionId[0] : to.params.sessionId
        return sessionId ? `/st/chat/${sessionId}` : '/st/chat'
      },
    },
    {
      path: '/st/chat/:sessionId?',
      name: 'st-chat',
      component: () => import('@/views/STChatView.vue'),
      meta: { title: 'ST 聊天' },
    },
    {
      path: '/library',
      redirect: '/st',
    },
    {
      path: '/resources/characters',
      redirect: '/st/resources/characters',
    },
    {
      path: '/resources/worldbooks',
      redirect: '/st/resources/worldbooks',
    },
    {
      path: '/resources/presets',
      redirect: '/st/resources/presets',
    },
    {
      path: '/resources/regex',
      redirect: '/st/resources/regex',
    },
    {
      path: '/st/resources/characters',
      name: 'resources-characters',
      component: () => import('@/views/ResourcesCharactersView.vue'),
      meta: { title: '角色卡' },
    },
    {
      path: '/st/resources/worldbooks',
      name: 'resources-worldbooks',
      component: () => import('@/views/ResourcesWorldbooksView.vue'),
      meta: { title: '世界书' },
    },
    {
      path: '/st/resources/presets',
      name: 'resources-presets',
      component: () => import('@/views/ResourcesPresetsView.vue'),
      meta: { title: '预设' },
    },
    {
      path: '/st/resources/regex',
      name: 'resources-regex',
      component: () => import('@/views/ResourcesRegexView.vue'),
      meta: { title: 'Regex' },
    },
    {
      path: '/agent',
      name: 'agent-home',
      component: () => import('@/views/AgentHomeView.vue'),
      meta: { title: 'Agent 工作区' },
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
      path: '/agent/worlds/:worldId/sessions/:sessionId',
      name: 'agent-chat',
      component: () => import('@/views/AgentChatView.vue'),
      meta: { title: 'Agent 会话' },
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

router.afterEach((to) => {
  const appShell = useAppShellStore()
  if (to.path.startsWith('/st')) {
    appShell.setCurrentMode('st')
    appShell.rememberModeRoute('st', to.fullPath)
  } else if (to.path.startsWith('/agent')) {
    appShell.setCurrentMode('agent')
    appShell.rememberModeRoute('agent', to.fullPath)
  }
})

export default router
