import type { TimeAnchor } from '@/types/agent/session'

export interface AgentWorldListItem {
  world_id: string
  session_count: number
  active_session_count: number
  character_count: number
  mainline_time_anchor: TimeAnchor | null
  updated_at: string | null
}

export interface CreateAgentWorldInput {
  name: string
}
