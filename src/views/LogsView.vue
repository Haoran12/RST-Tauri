<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import {
  NAlert,
  NButton,
  NButtonGroup,
  NCard,
  NDescriptions,
  NDescriptionsItem,
  NEmpty,
  NIcon,
  NInput,
  NPagination,
  NSelect,
  NScrollbar,
  NSpin,
  NTabPane,
  NTabs,
  NTag,
  useMessage,
} from 'naive-ui'
import {
  DownloadOutline,
  LockClosedOutline,
  LockOpenOutline,
  RefreshOutline,
  SearchOutline,
  TrashOutline,
} from '@vicons/ionicons5'
import {
  exportLogs,
  confirmLogCleanup,
  getLogRecordDetail,
  getLogStorageSummary,
  getStreamChunks,
  previewLogCleanup,
  queryLogRecords,
  setLogProtection,
  type LogRecordDetail,
  type LogRecordFilter,
  type LogRecordSummary,
  type LogStorageSummary,
  type StreamChunkDetail,
} from '@/services/logs'

const message = useMessage()

const PAGE_SIZE = 7

const sourceScope = ref<NonNullable<LogRecordFilter['source_scope']>>('all')
const recordKind = ref<NonNullable<LogRecordFilter['record_kind']>>('all')
const level = ref('')
const status = ref('')
const mode = ref('')
const worldId = ref('')
const provider = ref('')
const llmNode = ref('')
const search = ref('')
const timeRange = ref<'1h' | '24h' | '7d' | 'all'>('24h')

const records = ref<LogRecordSummary[]>([])
const selectedRecord = ref<LogRecordSummary | null>(null)
const detail = ref<LogRecordDetail | null>(null)
const storageSummary = ref<LogStorageSummary | null>(null)
const streamChunks = ref<StreamChunkDetail[]>([])
const currentPage = ref(1)
const totalCount = ref(0)
const isLoading = ref(false)
const isDetailLoading = ref(false)
const isSummaryLoading = ref(false)
const isExporting = ref(false)
const isRetentionRunning = ref(false)
const isTogglingProtection = ref(false)
const error = ref<string | null>(null)

const sourceOptions = [
  { label: '全部索引', value: 'all' },
  { label: '全局 Logs', value: 'global' },
  { label: 'World Logs', value: 'world' },
  { label: 'Agent Trace', value: 'trace' },
]

const kindOptions = [
  { label: '全部类型', value: 'all' },
  { label: 'LLM 调用', value: 'llm' },
  { label: '应用事件', value: 'event' },
  { label: 'Agent Trace', value: 'trace' },
]

const levelOptions = [
  { label: '全部级别', value: '' },
  { label: 'debug', value: 'debug' },
  { label: 'info', value: 'info' },
  { label: 'warn', value: 'warn' },
  { label: 'error', value: 'error' },
  { label: 'fatal', value: 'fatal' },
]

const statusOptions = [
  { label: '全部状态', value: '' },
  { label: 'success', value: 'success' },
  { label: 'succeeded', value: 'succeeded' },
  { label: 'failure', value: 'failure' },
  { label: 'failed', value: 'failed' },
  { label: 'cancelled', value: 'cancelled' },
  { label: 'skipped', value: 'skipped' },
  { label: 'fallback_used', value: 'fallback_used' },
]

const modeOptions = [
  { label: '全部模式', value: '' },
  { label: 'ST', value: 'ST' },
  { label: 'Agent', value: 'Agent' },
  { label: 'app', value: 'app' },
]

const timeRangeOptions = [
  { label: '最近 1 小时', value: '1h' },
  { label: '最近 24 小时', value: '24h' },
  { label: '最近 7 天', value: '7d' },
  { label: '全部时间', value: 'all' },
]

const filter = computed<LogRecordFilter>(() => {
  const now = Date.now()
  const since =
    timeRange.value === '1h'
      ? new Date(now - 60 * 60 * 1000).toISOString()
      : timeRange.value === '24h'
        ? new Date(now - 24 * 60 * 60 * 1000).toISOString()
        : timeRange.value === '7d'
          ? new Date(now - 7 * 24 * 60 * 60 * 1000).toISOString()
          : undefined

  return compactFilter({
    source_scope: sourceScope.value,
    record_kind: recordKind.value,
    level: level.value || undefined,
    status: status.value || undefined,
    mode: mode.value || undefined,
    world_id: worldId.value.trim() || undefined,
    provider: provider.value.trim() || undefined,
    llm_node: llmNode.value.trim() || undefined,
    search: search.value.trim() || undefined,
    since,
  })
})

const selectedKind = computed(() => selectedRecord.value?.record_ref.record_kind)
const selectedSource = computed(() => selectedRecord.value?.record_ref.source.source_kind)
const selectedId = computed(() => selectedRecord.value?.record_ref.id)
const selectedLlm = computed(() => detail.value?.llm ?? null)
const selectedEvent = computed(() => detail.value?.event ?? null)
const selectedTrace = computed(() => detail.value?.trace ?? null)

const pageCount = computed(() => Math.ceil(totalCount.value / PAGE_SIZE) || 1)
const globalSummary = computed(() => storageSummary.value?.global ?? null)

function compactFilter(value: LogRecordFilter): LogRecordFilter {
  return Object.fromEntries(
    Object.entries(value).filter(([, entry]) => entry !== undefined && entry !== ''),
  ) as LogRecordFilter
}

function jsonText(value: unknown) {
  if (value === null || value === undefined) return ''
  if (typeof value === 'string') return value
  return JSON.stringify(value, null, 2)
}

function formatDate(value?: string | null) {
  if (!value) return '-'
  return new Date(value).toLocaleString()
}

function formatBytes(bytes?: number | null) {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let size = bytes
  let index = 0
  while (size >= 1024 && index < units.length - 1) {
    size /= 1024
    index += 1
  }
  return `${size.toFixed(index === 0 ? 0 : 1)} ${units[index]}`
}

function formatTokenUsage(usage: unknown): string {
  if (!usage || typeof usage !== 'object') return '-'
  const u = usage as { prompt_tokens?: number; completion_tokens?: number; total_tokens?: number }
  const parts: string[] = []
  if (u.prompt_tokens !== undefined) parts.push(`输入 ${u.prompt_tokens}`)
  if (u.completion_tokens !== undefined) parts.push(`输出 ${u.completion_tokens}`)
  if (u.total_tokens !== undefined) parts.push(`总计 ${u.total_tokens}`)
  return parts.join(' / ') || '-'
}

function statusType(value?: string | null) {
  if (!value) return 'default'
  if (['success', 'succeeded', 'canon'].includes(value)) return 'success'
  if (['failure', 'failed', 'error', 'fatal'].includes(value)) return 'error'
  if (['warn', 'warning', 'fallback_used'].includes(value)) return 'warning'
  return 'info'
}

function kindLabel(record: LogRecordSummary) {
  if (record.record_ref.record_kind === 'llm') return 'LLM'
  if (record.record_ref.record_kind === 'event') return '事件'
  return 'Trace'
}

function primaryContext(record: LogRecordSummary) {
  return record.request_id || record.trace_id || record.scene_turn_id || record.record_ref.id
}

function selectRecord(record: LogRecordSummary) {
  selectedRecord.value = record
  void loadDetail(record)
}

async function loadRecords(reset = true) {
  if (reset) {
    currentPage.value = 1
    records.value = []
    selectedRecord.value = null
    detail.value = null
    streamChunks.value = []
  }

  isLoading.value = true
  error.value = null
  try {
    const offset = (currentPage.value - 1) * PAGE_SIZE
    const page = await queryLogRecords(filter.value, {
      offset,
      limit: PAGE_SIZE,
    })
    records.value = page.records
    totalCount.value = page.total_count ?? page.records.length
    if (reset && page.records[0]) {
      selectRecord(page.records[0])
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    isLoading.value = false
  }
}

async function loadDetail(record: LogRecordSummary) {
  isDetailLoading.value = true
  streamChunks.value = []
  try {
    detail.value = await getLogRecordDetail(record.record_ref)
    if (record.record_ref.record_kind === 'llm' && (record.stream_chunk_count ?? 0) > 0) {
      const chunks = await getStreamChunks(record.record_ref.id, record.record_ref.source, {
        offset: 0,
        limit: 100,
      })
      streamChunks.value = chunks.chunks
    }
  } catch (e) {
    message.error(String(e))
  } finally {
    isDetailLoading.value = false
  }
}

async function loadStorageSummary() {
  isSummaryLoading.value = true
  try {
    storageSummary.value = await getLogStorageSummary()
  } catch (e) {
    message.error(String(e))
  } finally {
    isSummaryLoading.value = false
  }
}

async function refreshAll() {
  await Promise.all([loadRecords(true), loadStorageSummary()])
}

function handlePageChange(page: number) {
  currentPage.value = page
  void loadRecords(false)
}

async function runRetention() {
  isRetentionRunning.value = true
  try {
    const plan = await previewLogCleanup()
    const confirmed = window.confirm(
      [
        `将清理 ${plan.older_than_days} 天前的全局运行 Logs。`,
        `LLM 调用：${plan.llm_logs_to_delete} 条`,
        `应用事件：${plan.event_logs_to_delete} 条`,
        `关联 stream chunks：${plan.stream_chunks_affected} 条`,
        '不会自动删除 World 内 Agent Trace。',
      ].join('\n'),
    )
    if (!confirmed) return

    const result = await confirmLogCleanup(plan.plan_id)
    message.success(`Retention 完成：LLM ${result.llm_logs_deleted} 条，事件 ${result.event_logs_deleted} 条`)
    await refreshAll()
  } catch (e) {
    message.error(String(e))
  } finally {
    isRetentionRunning.value = false
  }
}

async function handleExport(format: 'json' | 'jsonl' | 'csv') {
  isExporting.value = true
  try {
    const result = await exportLogs(filter.value, format)
    const blob = new Blob([result.content], {
      type: format === 'csv' ? 'text/csv;charset=utf-8' : 'application/json;charset=utf-8',
    })
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = result.filename
    link.click()
    URL.revokeObjectURL(url)
    message.success(`已导出 ${result.record_count} 条日志`)
  } catch (e) {
    message.error(String(e))
  } finally {
    isExporting.value = false
  }
}

async function toggleProtection(record: LogRecordSummary) {
  const kind = record.record_ref.record_kind
  if (kind !== 'llm' && kind !== 'event') {
    message.warning('仅支持保护 LLM 调用和应用事件日志')
    return
  }

  isTogglingProtection.value = true
  try {
    const newProtected = !record.protected
    await setLogProtection({
      record_kind: kind as 'llm' | 'event',
      record_id: record.record_ref.id,
      protected: newProtected,
    })
    // 更新本地状态
    record.protected = newProtected
    message.success(newProtected ? '已保护此日志' : '已取消保护')
  } catch (e) {
    message.error(String(e))
  } finally {
    isTogglingProtection.value = false
  }
}

function filterByTrace(traceId?: string | null, world?: string | null) {
  if (!traceId) return
  sourceScope.value = 'trace'
  recordKind.value = 'trace'
  search.value = traceId
  worldId.value = world ?? ''
  void loadRecords(true)
}

function filterByRequest(requestId?: string | null, world?: string | null) {
  if (!requestId) return
  sourceScope.value = world ? 'world' : 'global'
  recordKind.value = 'llm'
  search.value = requestId
  worldId.value = world ?? ''
  void loadRecords(true)
}

let searchTimer: number | undefined
watch(
  () => [
    sourceScope.value,
    recordKind.value,
    level.value,
    status.value,
    mode.value,
    worldId.value,
    provider.value,
    llmNode.value,
    search.value,
    timeRange.value,
  ],
  () => {
    window.clearTimeout(searchTimer)
    searchTimer = window.setTimeout(() => void loadRecords(true), 250)
  },
)

onMounted(refreshAll)
</script>

<template>
  <div class="logs-view">
    <header class="logs-toolbar">
      <div>
        <h1>日志</h1>
        <p>运行 Logs、Provider 请求响应与 Agent Trace 调试视图</p>
      </div>
      <div class="toolbar-actions">
        <NButton :loading="isLoading || isSummaryLoading" @click="refreshAll">
          <template #icon>
            <NIcon :component="RefreshOutline" />
          </template>
        </NButton>
        <NButtonGroup>
          <NButton :loading="isExporting" @click="handleExport('json')">
            <template #icon>
              <NIcon :component="DownloadOutline" />
            </template>
            JSON
          </NButton>
          <NButton :loading="isExporting" @click="handleExport('csv')">
            CSV
          </NButton>
        </NButtonGroup>
        <NButton type="warning" secondary :loading="isRetentionRunning" @click="runRetention">
          <template #icon>
            <NIcon :component="TrashOutline" />
          </template>
          Retention
        </NButton>
      </div>
    </header>

    <div class="summary-strip">
      <NCard size="small" embedded>
        <div class="summary-title">全局 Logs</div>
        <div class="summary-value">{{ formatBytes(globalSummary?.size_bytes) }}</div>
        <div class="summary-meta">
          {{ globalSummary?.llm_count ?? 0 }} LLM / {{ globalSummary?.event_count ?? 0 }} 事件
        </div>
      </NCard>
      <NCard size="small" embedded>
        <div class="summary-title">World 日志库</div>
        <div class="summary-value">{{ storageSummary?.worlds.length ?? 0 }}</div>
        <div class="summary-meta">
          {{ storageSummary?.worlds.filter((item) => item.stale_prompt_required).length ?? 0 }} 个需要提示
        </div>
      </NCard>
      <NCard size="small" embedded>
        <div class="summary-title">当前结果</div>
        <div class="summary-value">{{ totalCount }}</div>
        <div class="summary-meta">第 {{ currentPage }} / {{ pageCount }} 页</div>
      </NCard>
    </div>

    <NAlert v-if="error" type="error" class="logs-alert">
      {{ error }}
    </NAlert>

    <main class="logs-workspace">
      <aside class="filter-panel">
        <NScrollbar>
          <div class="filter-stack">
            <NInput v-model:value="search" clearable placeholder="request / trace / event / 文本">
              <template #prefix>
                <NIcon :component="SearchOutline" />
              </template>
            </NInput>
            <NSelect v-model:value="sourceScope" :options="sourceOptions" />
            <NSelect v-model:value="recordKind" :options="kindOptions" />
            <NSelect v-model:value="timeRange" :options="timeRangeOptions" />
            <NSelect v-model:value="level" :options="levelOptions" clearable />
            <NSelect v-model:value="status" :options="statusOptions" clearable />
            <NSelect v-model:value="mode" :options="modeOptions" clearable />
            <NInput v-model:value="worldId" clearable placeholder="world_id" />
            <NInput v-model:value="provider" clearable placeholder="provider" />
            <NInput v-model:value="llmNode" clearable placeholder="llm_node / source_module" />
          </div>
        </NScrollbar>
        <div class="pagination-area">
          <NPagination
            v-model:page="currentPage"
            :page-count="pageCount"
            :page-slot="5"
            size="small"
            @update:page="handlePageChange"
          />
        </div>
      </aside>

      <section class="record-list">
        <NSpin :show="isLoading" class="record-list-spin">
          <NScrollbar class="record-list-scroll">
            <div v-if="records.length === 0 && !isLoading" class="empty-area">
              <NEmpty description="没有匹配的日志记录" />
            </div>
            <div class="record-list-content">
              <button
                v-for="record in records"
                :key="`${record.record_ref.source.source_kind}-${record.record_ref.id}`"
                type="button"
                class="record-row"
                :class="{ active: selectedId === record.record_ref.id && selectedKind === record.record_ref.record_kind }"
                @click="selectRecord(record)"
              >
                <div class="record-row-top">
                  <div class="record-title">
                    <NTag size="small" :bordered="false">{{ kindLabel(record) }}</NTag>
                    <span>{{ record.title }}</span>
                  </div>
                  <div class="record-tags">
                    <NTag
                      v-if="record.status || record.level"
                      size="small"
                      :type="statusType(record.status || record.level)"
                    >
                      {{ record.status || record.level }}
                    </NTag>
                    <NTag v-if="record.protected === true" size="small" type="success">
                      <template #icon>
                        <NIcon :component="LockClosedOutline" />
                      </template>
                      已保护
                    </NTag>
                  </div>
                </div>
                <div class="record-summary">{{ record.summary || primaryContext(record) }}</div>
                <div class="record-meta">
                  <span>{{ record.record_ref.source.source_kind }}</span>
                  <span>{{ formatDate(record.created_at) }}</span>
                  <span v-if="record.latency_ms">{{ record.latency_ms }}ms</span>
                  <span v-if="record.stream_chunk_count">{{ record.stream_chunk_count }} chunks</span>
                  <span v-if="record.step_count">{{ record.step_count }} steps</span>
                </div>
                <div v-if="record.record_ref.record_kind === 'llm' || record.record_ref.record_kind === 'event'" class="record-actions">
                  <NButton
                    size="tiny"
                    :type="record.protected ? 'success' : 'default'"
                    :loading="isTogglingProtection"
                    @click.stop="toggleProtection(record)"
                  >
                    <template #icon>
                      <NIcon :component="record.protected ? LockClosedOutline : LockOpenOutline" />
                    </template>
                    {{ record.protected ? '取消保护' : '保护' }}
                  </NButton>
                </div>
              </button>
            </div>
          </NScrollbar>
        </NSpin>
      </section>

      <section class="detail-panel">
        <NSpin :show="isDetailLoading" class="detail-spin">
          <div v-if="!selectedRecord" class="empty-area">
            <NEmpty description="选择一条日志查看详情" />
          </div>

          <div v-else class="detail-content">
            <div class="detail-heading">
              <div>
                <h2>{{ selectedRecord.title }}</h2>
                <p>{{ primaryContext(selectedRecord) }}</p>
              </div>
              <NTag :type="statusType(selectedRecord.status || selectedRecord.level)">
                {{ selectedRecord.status || selectedRecord.level || selectedSource }}
              </NTag>
            </div>

            <div class="detail-tabs-wrapper">
              <NTabs type="line" animated class="detail-tabs">
                <NTabPane name="summary" tab="摘要">
                  <NScrollbar class="tab-scroll">
                    <NDescriptions :column="2" size="small" bordered>
                      <NDescriptionsItem label="来源">{{ selectedSource }}</NDescriptionsItem>
                      <NDescriptionsItem label="类型">{{ selectedKind }}</NDescriptionsItem>
                      <NDescriptionsItem label="创建时间">{{ formatDate(selectedRecord.created_at) }}</NDescriptionsItem>
                      <NDescriptionsItem label="World">{{ selectedRecord.world_id || '-' }}</NDescriptionsItem>
                      <NDescriptionsItem label="Session">{{ selectedRecord.session_id || '-' }}</NDescriptionsItem>
                      <NDescriptionsItem label="Turn">{{ selectedRecord.scene_turn_id || '-' }}</NDescriptionsItem>
                      <NDescriptionsItem label="Trace">{{ selectedRecord.trace_id || '-' }}</NDescriptionsItem>
                      <NDescriptionsItem label="Request">{{ selectedRecord.request_id || '-' }}</NDescriptionsItem>
                    </NDescriptions>
                    <div v-if="selectedLlm" class="llm-summary-section">
                      <NDescriptions :column="2" size="small" bordered>
                        <NDescriptionsItem label="Provider">{{ selectedLlm.provider }}</NDescriptionsItem>
                        <NDescriptionsItem label="Model">{{ selectedLlm.model }}</NDescriptionsItem>
                        <NDescriptionsItem label="请求 URL">{{ selectedLlm.request_url || '-' }}</NDescriptionsItem>
                        <NDescriptionsItem label="调用类型">{{ selectedLlm.call_type }}</NDescriptionsItem>
                        <NDescriptionsItem label="状态">
                          <NTag size="small" :type="statusType(selectedLlm.status)">{{ selectedLlm.status }}</NTag>
                        </NDescriptionsItem>
                        <NDescriptionsItem label="延迟">{{ selectedLlm.latency_ms ? `${selectedLlm.latency_ms}ms` : '-' }}</NDescriptionsItem>
                        <NDescriptionsItem label="Token 用量">
                          <template v-if="selectedLlm.token_usage">
                            {{ formatTokenUsage(selectedLlm.token_usage) }}
                          </template>
                          <template v-else>-</template>
                        </NDescriptionsItem>
                        <NDescriptionsItem label="节点">{{ selectedLlm.llm_node }}</NDescriptionsItem>
                      </NDescriptions>
                    </div>
                    <div class="detail-actions">
                      <NButton
                        v-if="selectedKind === 'llm' || selectedKind === 'event'"
                        size="small"
                        :type="selectedRecord.protected ? 'success' : 'default'"
                        :loading="isTogglingProtection"
                        @click="toggleProtection(selectedRecord)"
                      >
                        <template #icon>
                          <NIcon :component="selectedRecord.protected ? LockClosedOutline : LockOpenOutline" />
                        </template>
                        {{ selectedRecord.protected ? '取消保护' : '保护日志' }}
                      </NButton>
                      <NButton
                        v-if="selectedRecord.trace_id"
                        size="small"
                        @click="filterByTrace(selectedRecord.trace_id, selectedRecord.world_id)"
                      >
                        查看 Trace
                      </NButton>
                      <NButton
                        v-if="selectedRecord.request_id"
                        size="small"
                        @click="filterByRequest(selectedRecord.request_id, selectedRecord.world_id)"
                      >
                        查看 Request
                      </NButton>
                    </div>
                  </NScrollbar>
                </NTabPane>

                <NTabPane v-if="selectedLlm" name="request" tab="原始请求">
                  <NScrollbar class="tab-scroll">
                    <pre class="json-block">{{ jsonText(selectedLlm.request_json) }}</pre>
                  </NScrollbar>
                </NTabPane>
                <NTabPane v-if="selectedLlm" name="response" tab="原始响应">
                  <NScrollbar class="tab-scroll">
                    <NAlert type="info" class="inline-alert">
                      原始响应可能包含 prompt、角色卡、世界书或 Agent 私有上下文；数据库写入层已执行凭证脱敏。
                    </NAlert>
                    <pre class="json-block">{{ jsonText(selectedLlm.response_json) }}</pre>
                  </NScrollbar>
                </NTabPane>
                <NTabPane v-if="selectedLlm" name="readable" tab="可读内容">
                  <NScrollbar class="tab-scroll">
                    <div v-if="selectedLlm.reasoning_text" class="reasoning-section">
                      <div class="section-label">推理过程</div>
                      <pre class="text-block reasoning-block">{{ selectedLlm.reasoning_text }}</pre>
                    </div>
                    <div v-if="selectedLlm.readable_text">
                      <div class="section-label">对话内容</div>
                      <pre class="text-block">{{ selectedLlm.readable_text }}</pre>
                    </div>
                    <div v-else class="empty-area compact">
                      <NEmpty description="没有可读内容" />
                    </div>
                  </NScrollbar>
                </NTabPane>
                <NTabPane v-if="selectedLlm" name="schema" tab="Schema">
                  <NScrollbar class="tab-scroll">
                    <pre class="json-block">{{ jsonText(selectedLlm.schema_json) || '无结构化 schema' }}</pre>
                  </NScrollbar>
                </NTabPane>
                <NTabPane v-if="selectedLlm" name="stream" tab="Stream">
                  <NScrollbar class="tab-scroll">
                    <div v-if="streamChunks.length === 0" class="empty-area compact">
                      <NEmpty description="没有 stream chunk" />
                    </div>
                    <div v-else class="chunk-list">
                      <div v-for="chunk in streamChunks" :key="chunk.chunk_id" class="chunk-item">
                        <div class="chunk-meta">#{{ chunk.chunk_index }} · {{ formatDate(chunk.received_at) }}</div>
                        <pre>{{ chunk.raw_chunk }}</pre>
                      </div>
                    </div>
                  </NScrollbar>
                </NTabPane>

                <NTabPane v-if="selectedEvent" name="event" tab="事件">
                  <NScrollbar class="tab-scroll">
                    <NDescriptions :column="1" size="small" bordered>
                      <NDescriptionsItem label="事件">{{ selectedEvent.event_type }}</NDescriptionsItem>
                      <NDescriptionsItem label="级别">{{ selectedEvent.level }}</NDescriptionsItem>
                      <NDescriptionsItem label="模块">{{ selectedEvent.source_module }}</NDescriptionsItem>
                      <NDescriptionsItem label="消息">{{ selectedEvent.message }}</NDescriptionsItem>
                    </NDescriptions>
                    <pre class="json-block">{{ jsonText(selectedEvent.detail_json) || '无 detail_json' }}</pre>
                  </NScrollbar>
                </NTabPane>

                <NTabPane v-if="selectedTrace" name="trace" tab="Trace">
                  <NScrollbar class="tab-scroll">
                    <pre class="json-block">{{ jsonText(selectedTrace.summary) }}</pre>
                    <div class="step-list">
                      <div v-for="step in selectedTrace.steps" :key="step.step_trace_id" class="step-item">
                        <div class="step-title">
                          <strong>{{ step.step_name }}</strong>
                          <NTag size="small" :type="statusType(step.step_status)">{{ step.step_status }}</NTag>
                        </div>
                        <div class="record-meta">
                          <span>{{ formatDate(step.created_at) }}</span>
                          <button
                            v-if="step.linked_request_id"
                            type="button"
                            class="link-button"
                            @click="filterByRequest(step.linked_request_id, selectedRecord.world_id)"
                          >
                            {{ step.linked_request_id }}
                          </button>
                        </div>
                        <pre class="json-block small">{{ jsonText({
                          input_summary: step.input_summary,
                          output_summary: step.output_summary,
                          decision_json: step.decision_json,
                          error_event_id: step.error_event_id,
                        }) }}</pre>
                      </div>
                    </div>
                  </NScrollbar>
                </NTabPane>
              </NTabs>
            </div>
          </div>
        </NSpin>
      </section>
    </main>
  </div>
</template>

<style scoped>
.logs-view {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--n-color);
  overflow: hidden;
}

.logs-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 18px;
  border-bottom: 1px solid var(--n-border-color);
  flex-shrink: 0;
}

.logs-toolbar h1 {
  margin: 0;
  font-size: 18px;
  line-height: 1.25;
}

.logs-toolbar p,
.detail-heading p {
  margin: 4px 0 0;
  color: var(--n-text-color-3);
  font-size: 12px;
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.summary-strip {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  padding: 12px 18px;
  border-bottom: 1px solid var(--n-border-color);
  flex-shrink: 0;
}

.summary-title,
.summary-meta,
.record-meta,
.chunk-meta {
  color: var(--n-text-color-3);
  font-size: 12px;
}

.summary-value {
  margin-top: 4px;
  font-size: 20px;
  font-weight: 700;
}

.logs-alert {
  margin: 10px 18px 0;
  flex-shrink: 0;
}

.logs-workspace {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 250px minmax(280px, 0.9fr) minmax(360px, 1.25fr);
  overflow: hidden;
}

.filter-panel,
.record-list,
.detail-panel {
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border-right: 1px solid var(--n-border-color);
  display: flex;
  flex-direction: column;
}

.detail-panel {
  border-right: 0;
}

.filter-stack {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
}

.pagination-area {
  padding: 12px;
  border-top: 1px solid var(--n-border-color);
  display: flex;
  justify-content: center;
}

/* filter-panel NScrollbar 高度 - NScrollbar 默认 height:100%，需要父级有明确高度 */
.filter-panel > :deep(.n-scrollbar) {
  flex: 1;
  min-height: 0;
}

.filter-panel :deep(.n-scrollbar-container) {
  height: 100%;
}

/* record-list 高度链 */
.record-list-spin {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.record-list-spin :deep(.n-spin-container) {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.record-list-spin :deep(.n-spin-content) {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.record-list-scroll {
  flex: 1;
  min-height: 0;
}

.record-list-scroll :deep(.n-scrollbar-container) {
  height: 100%;
}

/* detail-panel 高度链 */
.detail-spin {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.detail-spin :deep(.n-spin-container) {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.detail-spin :deep(.n-spin-content) {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.record-list-content {
  min-height: 100%;
}

.record-row {
  display: block;
  width: 100%;
  padding: 12px;
  border: 0;
  border-bottom: 1px solid var(--n-border-color);
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.record-row:hover,
.record-row.active {
  background: rgba(32, 128, 240, 0.1);
}

.record-row-top,
.record-title,
.record-meta,
.detail-heading,
.detail-actions,
.step-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.record-row-top,
.detail-heading,
.step-title {
  justify-content: space-between;
}

.record-title {
  min-width: 0;
  font-weight: 600;
}

.record-title span,
.record-summary {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.record-summary {
  margin-top: 6px;
  color: var(--n-text-color-2);
  font-size: 13px;
}

.record-meta {
  flex-wrap: wrap;
  margin-top: 8px;
}

.record-tags {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.record-actions {
  margin-top: 8px;
  display: flex;
  gap: 6px;
}

.empty-area {
  padding: 20px;
  display: flex;
  justify-content: center;
}

.empty-area {
  height: 100%;
  align-items: center;
}

.empty-area.compact {
  height: auto;
}

.detail-content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.detail-heading {
  flex-shrink: 0;
  padding: 14px 14px 0;
}

.detail-tabs-wrapper {
  flex: 1;
  min-height: 0;
  padding: 8px 14px 14px;
  overflow: hidden;
}

.detail-tabs {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.detail-tabs :deep(.n-tabs-nav) {
  flex-shrink: 0;
}

.detail-tabs :deep(.n-tabs-pane-wrapper) {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.detail-tabs :deep(.n-tabs-pane-wrapper > div) {
  height: 100%;
  min-height: 0;
}

.detail-tabs :deep(.n-tab-pane) {
  height: 100%;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* TabPane 内 NScrollbar 高度链 */
.tab-scroll {
  flex: 1;
  min-height: 0;
}

.tab-scroll :deep(.n-scrollbar-container) {
  height: 100%;
}

.tab-scroll :deep(.n-scrollbar-content) {
  padding: 0 4px;
}

.detail-heading h2 {
  margin: 0;
  font-size: 16px;
  line-height: 1.3;
}

.detail-actions {
  margin-top: 12px;
  padding-bottom: 4px;
}

.inline-alert {
  margin-bottom: 10px;
}

.llm-summary-section {
  margin-top: 16px;
}

.section-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--n-text-color-2);
  margin-bottom: 8px;
}

.reasoning-section {
  margin-bottom: 16px;
}

.reasoning-block {
  background: rgba(255, 193, 7, 0.1);
  border-color: rgba(255, 193, 7, 0.3);
}

.json-block,
.text-block,
.chunk-item pre {
  margin: 10px 0 0;
  padding: 10px;
  border: 1px solid var(--n-border-color);
  border-radius: 6px;
  background: var(--n-code-color);
  color: var(--n-text-color);
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
}

.json-block.small {
  max-height: 220px;
  overflow: auto;
}

.chunk-list,
.step-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.chunk-item,
.step-item {
  padding: 10px;
  border: 1px solid var(--n-border-color);
  border-radius: 8px;
}

.link-button {
  border: 0;
  padding: 0;
  background: transparent;
  color: var(--n-primary-color);
  font: inherit;
  cursor: pointer;
}

@media (max-width: 1180px) {
  .logs-workspace {
    grid-template-columns: 220px minmax(260px, 1fr);
  }

  .detail-panel {
    grid-column: 1 / -1;
    border-top: 1px solid var(--n-border-color);
  }
}

@media (max-width: 760px) {
  .logs-toolbar,
  .summary-strip {
    grid-template-columns: 1fr;
  }

  .logs-toolbar {
    align-items: flex-start;
    flex-direction: column;
  }

  .logs-workspace {
    grid-template-columns: 1fr;
  }
}
</style>
