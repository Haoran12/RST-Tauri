// Agent Session API calls

import { invoke } from '@tauri-apps/api/core'
import type {
  AgentSession,
  SessionTurn,
  WorldMainlineCursor,
  TimeAnchor,
  PlayerMode,
  RuntimeTurnCanonStatus,
  TruthGuidance,
  OpenDetailSlot,
  ProvisionalSessionTruth,
  DetailSlotFillResult,
  PromotionEvaluationResult,
  PromotionResult,
  ConflictReport,
} from '@/types/agent/session'
import type { CharacterRecord } from '@/types/agent/character'
import type { AgentWorldListItem, CreateAgentWorldInput } from '@/types/agent/world'

// ===== World List =====

export async function listAgentWorlds(): Promise<AgentWorldListItem[]> {
  return await invoke<AgentWorldListItem[]>('list_agent_worlds')
}

export async function createAgentWorld(input: CreateAgentWorldInput): Promise<AgentWorldListItem> {
  return await invoke<AgentWorldListItem>('create_agent_world', { input })
}

// ===== Session Management =====

export interface CreateSessionInput {
  world_id: string
  title: string
  player_mode: PlayerMode
  player_character_id: string | null
  period_anchor: TimeAnchor
}

export async function createAgentSession(input: CreateSessionInput): Promise<AgentSession> {
  return await invoke<AgentSession>('create_agent_session', { input })
}

export async function listAgentSessions(worldId: string): Promise<AgentSession[]> {
  return await invoke<AgentSession[]>('list_agent_sessions', { worldId })
}

export async function getAgentSession(
  worldId: string,
  sessionId: string
): Promise<AgentSession | null> {
  return await invoke<AgentSession | null>('get_agent_session', { worldId, sessionId })
}

export async function listAgentSessionTurns(
  worldId: string,
  sessionId: string
): Promise<SessionTurn[]> {
  return await invoke<SessionTurn[]>('list_agent_session_turns', { worldId, sessionId })
}

export interface UpdateAgentSessionTurnInput {
  world_id: string
  session_id: string
  session_turn_id: string
  content: string
}

export async function updateAgentSessionTurn(
  input: UpdateAgentSessionTurnInput
): Promise<SessionTurn> {
  return await invoke<SessionTurn>('update_agent_session_turn', { input })
}

export interface DeleteAgentSessionTurnInput {
  world_id: string
  session_id: string
  session_turn_id: string
}

export async function deleteAgentSessionTurn(input: DeleteAgentSessionTurnInput): Promise<void> {
  return await invoke<void>('delete_agent_session_turn', { input })
}

export interface DeleteAgentSessionInput {
  world_id: string
  session_id: string
}

export async function deleteAgentSession(input: DeleteAgentSessionInput): Promise<void> {
  return await invoke<void>('delete_agent_session', { input })
}

export interface AgentTurnResult {
  scene_turn_id: string
  narrative_text: string
  canon_status: RuntimeTurnCanonStatus
  runtime_config_snapshot_id: string
  world_rules_snapshot_id: string | null
}

export interface ProcessAgentTurnInput {
  world_id: string
  session_id: string
  content: string
}

export interface ProcessAgentTurnOutput {
  result: AgentTurnResult
  user_turn: SessionTurn
  assistant_turn: SessionTurn
}

export async function processAgentTurn(
  input: ProcessAgentTurnInput
): Promise<ProcessAgentTurnOutput> {
  return await invoke<ProcessAgentTurnOutput>('process_agent_turn', { input })
}

export interface UpdatePlayerModeInput {
  world_id: string
  session_id: string
  player_mode: PlayerMode
  player_character_id: string | null
}

export async function updateSessionPlayerMode(input: UpdatePlayerModeInput): Promise<AgentSession> {
  return await invoke<AgentSession>('update_session_player_mode', { input })
}

// ===== Timeline =====

export async function getWorldMainlineCursor(worldId: string): Promise<WorldMainlineCursor> {
  return await invoke<WorldMainlineCursor>('get_world_mainline_cursor', { worldId })
}

export interface AdvanceMainlineInput {
  world_id: string
  turn_id: string
  new_time_anchor: TimeAnchor
}

export async function advanceWorldMainline(input: AdvanceMainlineInput): Promise<WorldMainlineCursor> {
  return await invoke<WorldMainlineCursor>('advance_world_mainline', { input })
}

// ===== Characters =====

export async function listWorldCharacters(worldId: string): Promise<CharacterRecord[]> {
  return await invoke<CharacterRecord[]>('list_world_characters', { worldId })
}

// ===== Time Anchor Helpers =====

export interface CreateTimeAnchorInput {
  ordinal: number
  display_text: string
  precision?: 'Exact' | 'Day' | 'Period' | 'Era'
  calendar_id?: string
}

export async function createTimeAnchor(input: CreateTimeAnchorInput): Promise<TimeAnchor> {
  return await invoke<TimeAnchor>('create_time_anchor', { input })
}

export async function compareTimeAnchors(anchor1: TimeAnchor, anchor2: TimeAnchor): Promise<number> {
  return await invoke<number>('compare_time_anchors', { input: { anchor1, anchor2 } })
}

// ===== Past Timeline (Retrospective Session) =====

export interface GetTruthGuidanceInput {
  world_id: string
  session_id: string
}

export async function getTruthGuidance(input: GetTruthGuidanceInput): Promise<TruthGuidance> {
  return await invoke<TruthGuidance>('get_truth_guidance', { input })
}

export interface GetOpenDetailSlotsInput {
  world_id: string
  session_id: string
}

export async function getOpenDetailSlots(input: GetOpenDetailSlotsInput): Promise<OpenDetailSlot[]> {
  return await invoke<OpenDetailSlot[]>('get_open_detail_slots', { input })
}

export interface FillDetailSlotInput {
  world_id: string
  session_id: string
  session_turn_id: string
  scene_turn_id: string | null
  event_id: string
  slot_id: string
  detail_kind: 'motive' | 'dialogue' | 'witness' | 'route' | 'local_cause'
  fill_content: unknown
}

export async function fillDetailSlot(input: FillDetailSlotInput): Promise<DetailSlotFillResult> {
  return await invoke<DetailSlotFillResult>('fill_detail_slot', { input })
}

export interface GetProvisionalCandidatesInput {
  world_id: string
  session_id: string
  status_filter?: 'pending' | 'promoted' | 'blocked_conflict' | 'noncanon'
}

export async function getProvisionalCandidates(input: GetProvisionalCandidatesInput): Promise<ProvisionalSessionTruth[]> {
  return await invoke<ProvisionalSessionTruth[]>('get_provisional_candidates', { input })
}

export interface PromoteCandidatesInput {
  world_id: string
  provisional_ids: string[]
  scene_turn_id: string
}

export async function promoteProvisionalCandidates(input: PromoteCandidatesInput): Promise<string[]> {
  return await invoke<string[]>('promote_provisional_candidates', { input })
}

export interface MarkNonCanonInput {
  world_id: string
  provisional_id: string
}

export async function markProvisionalNonCanon(input: MarkNonCanonInput): Promise<void> {
  return await invoke<void>('mark_provisional_non_canon', { input })
}

// ===== Canon Status =====

export interface EvaluateCanonEligibilityInput {
  world_id: string
  session_id: string
}

export async function evaluateCanonEligibility(input: EvaluateCanonEligibilityInput): Promise<PromotionEvaluationResult> {
  return await invoke<PromotionEvaluationResult>('evaluate_canon_eligibility', { input })
}

export interface PromoteToCanonInput {
  world_id: string
  session_id: string
  scene_turn_id: string
}

export async function promoteToCanon(input: PromoteToCanonInput): Promise<PromotionResult> {
  return await invoke<PromotionResult>('promote_to_canon', { input })
}

export interface GetSessionConflictsInput {
  world_id: string
  session_id: string
}

export async function getSessionConflicts(input: GetSessionConflictsInput): Promise<ConflictReport[]> {
  return await invoke<ConflictReport[]>('get_session_conflicts', { input })
}
