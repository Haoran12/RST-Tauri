<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import {
  NCard,
  NForm,
  NFormItem,
  NInput,
  NInputNumber,
  NSwitch,
  NSelect,
  NButton,
  NSpace,
  NDivider,
  NCollapse,
  NCollapseItem,
  NDynamicTags,
  NCheckbox,
  NEmpty,
} from 'naive-ui'
import type { WorldInfoEntry } from '@/types/st'
import { WorldInfoLogic, WorldInfoPosition, ExtensionPromptRole, createWorldInfoEntry } from '@/types/st'

const props = defineProps<{
  entry: WorldInfoEntry | null
  groups: string[]
}>()

const emit = defineEmits<{
  (e: 'update', entry: WorldInfoEntry): void
  (e: 'delete'): void
}>()

// Local state for editing
const localEntry = ref<WorldInfoEntry>(createWorldInfoEntry(0))

// Watch for entry changes
watch(
  () => props.entry,
  (newEntry) => {
    if (newEntry) {
      localEntry.value = { ...newEntry }
    } else {
      localEntry.value = createWorldInfoEntry(0)
    }
  },
  { immediate: true }
)

// Position options
const positionOptions = [
  { label: 'Before Character', value: WorldInfoPosition.BEFORE_CHAR },
  { label: 'After Character', value: WorldInfoPosition.AFTER_CHAR },
  { label: 'Author\'s Note Top', value: WorldInfoPosition.AN_TOP },
  { label: 'Author\'s Note Bottom', value: WorldInfoPosition.AN_BOTTOM },
  { label: 'At Depth', value: WorldInfoPosition.AT_DEPTH },
  { label: 'Example Message Top', value: WorldInfoPosition.EM_TOP },
  { label: 'Example Message Bottom', value: WorldInfoPosition.EM_BOTTOM },
  { label: 'Outlet', value: WorldInfoPosition.OUTLET },
]

// Logic options
const logicOptions = [
  { label: 'AND ANY (all primary + any secondary)', value: WorldInfoLogic.AND_ANY },
  { label: 'NOT ALL (not all primary)', value: WorldInfoLogic.NOT_ALL },
  { label: 'NOT ANY (not any primary)', value: WorldInfoLogic.NOT_ANY },
  { label: 'AND ALL (all primary + all secondary)', value: WorldInfoLogic.AND_ALL },
]

// Role options
const roleOptions = [
  { label: 'System', value: ExtensionPromptRole.SYSTEM },
  { label: 'User', value: ExtensionPromptRole.USER },
  { label: 'Assistant', value: ExtensionPromptRole.ASSISTANT },
]

// Group options (existing groups + new)
const groupOptions = computed(() => {
  const existing = props.groups.map((g) => ({ label: g, value: g }))
  return [{ label: '(No Group)', value: '' }, ...existing]
})

// Is AT_DEPTH position
const isAtDepth = computed(() => localEntry.value.position === WorldInfoPosition.AT_DEPTH)

// Is OUTLET position
const isOutlet = computed(() => localEntry.value.position === WorldInfoPosition.OUTLET)

// Has group
const hasGroup = computed(() => localEntry.value.group && localEntry.value.group.trim())

// Save changes
function saveChanges() {
  emit('update', { ...localEntry.value })
}

// Delete entry
function deleteEntry() {
  emit('delete')
}
</script>

<template>
  <NCard v-if="entry" class="entry-editor" size="small">
    <NForm label-placement="left" label-width="100px" size="small">
      <!-- Basic Info -->
      <NFormItem label="Comment">
        <NInput
          v-model:value="localEntry.comment"
          placeholder="Entry name/comment"
          @blur="saveChanges"
        />
      </NFormItem>

      <NFormItem label="Content">
        <NInput
          v-model:value="localEntry.content"
          type="textarea"
          placeholder="Lore content..."
          :rows="6"
          @blur="saveChanges"
        />
      </NFormItem>

      <NDivider />

      <!-- Keywords -->
      <NFormItem label="Primary Keys">
        <NDynamicTags v-model:value="localEntry.key" @change="saveChanges" />
      </NFormItem>

      <NFormItem label="Secondary">
        <NDynamicTags v-model:value="localEntry.keysecondary" @change="saveChanges" />
      </NFormItem>

      <NFormItem label="Selective">
        <NSwitch v-model:value="localEntry.selective" @update:value="saveChanges" />
        <span class="hint">Require secondary keys</span>
      </NFormItem>

      <NFormItem v-if="localEntry.selective" label="Logic">
        <NSelect
          v-model:value="localEntry.selective_logic"
          :options="logicOptions"
          @update:value="saveChanges"
        />
      </NFormItem>

      <NDivider />

      <!-- Position & Order -->
      <NCollapse>
        <NCollapseItem title="Position & Order" name="position">
          <NFormItem label="Position">
            <NSelect
              v-model:value="localEntry.position"
              :options="positionOptions"
              @update:value="saveChanges"
            />
          </NFormItem>

          <NFormItem v-if="isAtDepth" label="Depth">
            <NInputNumber
              v-model:value="localEntry.depth"
              :min="0"
              :max="999"
              @blur="saveChanges"
            />
          </NFormItem>

          <NFormItem v-if="isAtDepth" label="Role">
            <NSelect
              v-model:value="localEntry.role"
              :options="roleOptions"
              @update:value="saveChanges"
            />
          </NFormItem>

          <NFormItem v-if="isOutlet" label="Outlet Name">
            <NInput
              v-model:value="localEntry.outlet_name"
              placeholder="Outlet identifier"
              @blur="saveChanges"
            />
          </NFormItem>

          <NFormItem label="Order">
            <NInputNumber
              v-model:value="localEntry.order"
              :min="0"
              :max="999"
              @blur="saveChanges"
            />
            <span class="hint">Lower = earlier in prompt</span>
          </NFormItem>
        </NCollapseItem>

        <!-- Probability & Budget -->
        <NCollapseItem title="Probability & Budget" name="probability">
          <NFormItem label="Probability">
            <NInputNumber
              v-model:value="localEntry.probability"
              :min="0"
              :max="100"
              :step="1"
              @blur="saveChanges"
            />
            <span class="hint">%</span>
          </NFormItem>

          <NFormItem label="Use Probability">
            <NSwitch v-model:value="localEntry.use_probability" @update:value="saveChanges" />
          </NFormItem>

          <NFormItem label="Ignore Budget">
            <NSwitch v-model:value="localEntry.ignore_budget" @update:value="saveChanges" />
          </NFormItem>
        </NCollapseItem>

        <!-- Group -->
        <NCollapseItem title="Group" name="group">
          <NFormItem label="Group">
            <NSelect
              v-model:value="localEntry.group"
              :options="groupOptions"
              :tag="true"
              filterable
              @update:value="saveChanges"
            />
          </NFormItem>

          <NFormItem v-if="hasGroup" label="Override">
            <NSwitch v-model:value="localEntry.group_override" @update:value="saveChanges" />
            <span class="hint">Replace other entries in group</span>
          </NFormItem>

          <NFormItem v-if="hasGroup" label="Weight">
            <NInputNumber
              v-model:value="localEntry.group_weight"
              :min="0"
              :max="100"
              @blur="saveChanges"
            />
          </NFormItem>
        </NCollapseItem>

        <!-- Recursion -->
        <NCollapseItem title="Recursion" name="recursion">
          <NFormItem label="Exclude Recursion">
            <NSwitch v-model:value="localEntry.exclude_recursion" @update:value="saveChanges" />
          </NFormItem>

          <NFormItem label="Prevent Recursion">
            <NSwitch v-model:value="localEntry.prevent_recursion" @update:value="saveChanges" />
          </NFormItem>

          <NFormItem label="Delay Until">
            <NInputNumber
              :value="typeof localEntry.delay_until_recursion === 'number' ? localEntry.delay_until_recursion : 0"
              :min="0"
              :max="999"
              @update:value="(v: number | null) => { if (v !== null) { localEntry.delay_until_recursion = v; saveChanges() } }"
            />
            <span class="hint">Recursion depth</span>
          </NFormItem>
        </NCollapseItem>

        <!-- Scan Settings -->
        <NCollapseItem title="Scan Settings" name="scan">
          <NFormItem label="Scan Depth">
            <NInputNumber
              :value="localEntry.scan_depth ?? undefined"
              :min="0"
              :max="999"
              placeholder="Use global"
              @update:value="(v: number | null) => { localEntry.scan_depth = v; saveChanges() }"
            />
          </NFormItem>

          <NFormItem label="Case Sensitive">
            <NSwitch
              :value="localEntry.case_sensitive ?? false"
              @update:value="(v: boolean) => { localEntry.case_sensitive = v; saveChanges() }"
            />
          </NFormItem>

          <NFormItem label="Match Whole">
            <NSwitch
              :value="localEntry.match_whole_words ?? false"
              @update:value="(v: boolean) => { localEntry.match_whole_words = v; saveChanges() }"
            />
          </NFormItem>
        </NCollapseItem>

        <!-- Time Control -->
        <NCollapseItem title="Time Control" name="time">
          <NFormItem label="Sticky">
            <NInputNumber
              :value="localEntry.sticky ?? undefined"
              :min="0"
              :max="999"
              placeholder="Disabled"
              @update:value="(v: number | null) => { localEntry.sticky = v; saveChanges() }"
            />
            <span class="hint">Stay active for N turns</span>
          </NFormItem>

          <NFormItem label="Cooldown">
            <NInputNumber
              :value="localEntry.cooldown ?? undefined"
              :min="0"
              :max="999"
              placeholder="Disabled"
              @update:value="(v: number | null) => { localEntry.cooldown = v; saveChanges() }"
            />
            <span class="hint">Wait N turns before re-activation</span>
          </NFormItem>

          <NFormItem label="Delay">
            <NInputNumber
              :value="localEntry.delay ?? undefined"
              :min="0"
              :max="999"
              placeholder="Disabled"
              @update:value="(v: number | null) => { localEntry.delay = v; saveChanges() }"
            />
            <span class="hint">Wait N turns before first activation</span>
          </NFormItem>
        </NCollapseItem>

        <!-- Match Targets -->
        <NCollapseItem title="Match Targets" name="targets">
          <NSpace vertical>
            <NCheckbox
              :checked="localEntry.match_persona_description"
              @update:checked="(v: boolean) => { localEntry.match_persona_description = v; saveChanges() }"
            >
              Persona Description
            </NCheckbox>
            <NCheckbox
              :checked="localEntry.match_character_description"
              @update:checked="(v: boolean) => { localEntry.match_character_description = v; saveChanges() }"
            >
              Character Description
            </NCheckbox>
            <NCheckbox
              :checked="localEntry.match_character_personality"
              @update:checked="(v: boolean) => { localEntry.match_character_personality = v; saveChanges() }"
            >
              Character Personality
            </NCheckbox>
            <NCheckbox
              :checked="localEntry.match_character_depth_prompt"
              @update:checked="(v: boolean) => { localEntry.match_character_depth_prompt = v; saveChanges() }"
            >
              Character Depth Prompt
            </NCheckbox>
            <NCheckbox
              :checked="localEntry.match_scenario"
              @update:checked="(v: boolean) => { localEntry.match_scenario = v; saveChanges() }"
            >
              Scenario
            </NCheckbox>
            <NCheckbox
              :checked="localEntry.match_creator_notes"
              @update:checked="(v: boolean) => { localEntry.match_creator_notes = v; saveChanges() }"
            >
              Creator Notes
            </NCheckbox>
          </NSpace>
        </NCollapseItem>

        <!-- Triggers -->
        <NCollapseItem title="Triggers" name="triggers">
          <NFormItem label="Triggers">
            <NDynamicTags v-model:value="localEntry.triggers" @change="saveChanges" />
          </NFormItem>

          <NFormItem label="Automation ID">
            <NInput
              v-model:value="localEntry.automation_id"
              placeholder="Automation identifier"
              @blur="saveChanges"
            />
          </NFormItem>
        </NCollapseItem>

        <!-- Status -->
        <NCollapseItem title="Status" name="status">
          <NFormItem label="Disabled">
            <NSwitch v-model:value="localEntry.disable" @update:value="saveChanges" />
          </NFormItem>

          <NFormItem label="Constant">
            <NSwitch v-model:value="localEntry.constant" @update:value="saveChanges" />
            <span class="hint">Always include in context</span>
          </NFormItem>

          <NFormItem label="Vectorized">
            <NSwitch v-model:value="localEntry.vectorized" @update:value="saveChanges" />
          </NFormItem>
        </NCollapseItem>
      </NCollapse>

      <NDivider />

      <!-- Actions -->
      <NSpace justify="end">
        <NButton type="error" size="small" @click="deleteEntry">
          Delete Entry
        </NButton>
      </NSpace>
    </NForm>
  </NCard>
  <NCard v-else class="entry-editor" size="small">
    <NEmpty description="Select an entry to edit" />
  </NCard>
</template>

<style scoped>
.entry-editor {
  height: 100%;
  overflow-y: auto;
}

.hint {
  color: #999;
  font-size: 12px;
  margin-left: 8px;
}
</style>