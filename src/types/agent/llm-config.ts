// Agent LLM Node Configuration Types

/**
 * Agent LLM node types that can have separate API configurations
 */
export type AgentLlmNodeType =
  | 'SceneInitializer'
  | 'SceneStateExtractor'
  | 'CharacterCognitivePass'
  | 'OutcomePlanner'
  | 'SurfaceRealizer'

/**
 * Agent LLM profile configuration
 * Maps each node type to an optional API config ID
 */
export interface AgentLlmProfile {
  world_id: string
  default_api_config_id: string | null
  scene_initializer_api_config_id: string | null
  scene_state_extractor_api_config_id: string | null
  character_cognitive_pass_api_config_id: string | null
  outcome_planner_api_config_id: string | null
  surface_realizer_api_config_id: string | null
  created_at: string
  updated_at: string
}

/**
 * Create a default Agent LLM profile
 */
export function createDefaultAgentLlmProfile(worldId: string): AgentLlmProfile {
  const now = new Date().toISOString()
  return {
    world_id: worldId,
    default_api_config_id: null,
    scene_initializer_api_config_id: null,
    scene_state_extractor_api_config_id: null,
    character_cognitive_pass_api_config_id: null,
    outcome_planner_api_config_id: null,
    surface_realizer_api_config_id: null,
    created_at: now,
    updated_at: now,
  }
}

/**
 * Node display information
 */
export const AGENT_LLM_NODE_INFO: Record<AgentLlmNodeType, {
  label: string
  description: string
  permission: string
}> = {
  SceneInitializer: {
    label: '场景初始化器',
    description: '新建场景、切场景、大幅跳时时生成候选场景草案',
    permission: '公开上下文 + 场景相关私有约束',
  },
  SceneStateExtractor: {
    label: '场景提取器',
    description: '解析用户自由文本，输出场景更新与用户意图',
    permission: '场景域 God-read',
  },
  CharacterCognitivePass: {
    label: '人物认知处理器',
    description: '更新角色主观感知、信念与意图',
    permission: '只读 L2 + prior L3',
  },
  OutcomePlanner: {
    label: '结果规划器',
    description: '综合场景真相、角色意图，产出候选结果与状态更新',
    permission: 'God-read，输出候选',
  },
  SurfaceRealizer: {
    label: '叙事渲染器',
    description: '将结构化结果转为用户可读的叙事文本',
    permission: '只读 NarrationScope 派生输入',
  },
}
