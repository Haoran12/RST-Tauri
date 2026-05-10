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
      meta: { title: 'Agent 首页' },
    },
    {
      path: '/agent/locations',
      name: 'agent-locations',
      component: () => import('@/views/AgentLocationsView.vue'),
      meta: { title: '地点' },
    },
    {
      path: '/agent/knowledge',
      name: 'agent-knowledge',
      component: () => import('@/views/AgentKnowledgeView.vue'),
      meta: { title: 'Knowledge' },
    },
    {
      path: '/agent/characters',
      name: 'agent-characters',
      component: () => import('@/views/AgentCharactersView.vue'),
      meta: { title: '人物' },
    },
    {
      path: '/agent/relationships',
      name: 'agent-relationships',
      component: () => import('@/views/AgentRelationshipsView.vue'),
      meta: { title: '关系' },
    },
    {
      path: '/agent/rules',
      name: 'agent-rules',
      component: () => import('@/views/AgentRulesView.vue'),
      meta: { title: '世界规则' },
    },
    {
      path: '/agent/sessions',
      name: 'agent-sessions',
      component: () => import('@/views/AgentSessionsView.vue'),
      meta: { title: '会话' },
    },
    {
      path: '/agent/chat/:worldId/:sessionId',
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
