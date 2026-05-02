<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import {
  NForm,
  NFormItem,
  NInput,
  NButton,
  NSpace,
  NUpload,
  NText,
  useMessage,
  type UploadFileInfo,
} from 'naive-ui'
import { useCharactersStore } from '@/stores/characters'
import type { TavernCardV3 } from '@/types/st'

const props = defineProps<{
  characterId: string
}>()

const emit = defineEmits<{
  close: []
}>()

const store = useCharactersStore()
const message = useMessage()

const form = ref<TavernCardV3 | null>(null)
const avatarUrl = ref<string | null>(null)

onMounted(async () => {
  await store.loadCharacter(props.characterId)
  form.value = store.currentCharacter ? { ...store.currentCharacter } : null
  avatarUrl.value = await store.getAvatarUrl(props.characterId)
})

const hasEmbeddedWorldbook = computed(
  () => form.value?.data.character_book != null
)

async function handleSave() {
  if (!form.value) return

  try {
    store.currentCharacter = form.value
    await store.saveCurrentCharacter(props.characterId)
    message.success('保存成功')
  } catch (e) {
    message.error(`保存失败: ${e}`)
  }
}

async function handleAvatarUpload(options: { file: UploadFileInfo }) {
  const file = options.file.file
  if (!file) return

  try {
    await store.updateAvatar(props.characterId, file)
    avatarUrl.value = await store.getAvatarUrl(props.characterId)
    message.success('头像更新成功')
  } catch (e) {
    message.error(`头像更新失败: ${e}`)
  }
}

async function handleImportWorldbook() {
  try {
    const loreId = await store.importWorldbook(props.characterId)
    message.success(`世界书导入成功，ID: ${loreId}`)
    // Reload character to get updated extensions
    await store.loadCharacter(props.characterId)
    form.value = store.currentCharacter ? { ...store.currentCharacter } : null
  } catch (e) {
    message.error(`导入世界书失败: ${e}`)
  }
}
</script>

<template>
  <div class="character-editor">
    <div v-if="form" class="editor-content">
      <!-- Avatar Section -->
      <div class="avatar-section">
        <div class="avatar-preview">
          <img v-if="avatarUrl" :src="avatarUrl" class="avatar" />
          <div v-else class="avatar-placeholder">
            <NText>无头像</NText>
          </div>
        </div>
        <NUpload
          accept="image/png"
          :custom-request="handleAvatarUpload"
          :show-file-list="false"
        >
          <NButton size="small">更换头像</NButton>
        </NUpload>
      </div>

      <!-- Form Section -->
      <NForm label-placement="top">
        <NFormItem label="名称">
          <NInput v-model:value="form.data.name" />
        </NFormItem>

        <NFormItem label="描述">
          <NInput
            v-model:value="form.data.description"
            type="textarea"
            :rows="4"
          />
        </NFormItem>

        <NFormItem label="性格">
          <NInput
            v-model:value="form.data.personality"
            type="textarea"
            :rows="3"
          />
        </NFormItem>

        <NFormItem label="场景">
          <NInput
            v-model:value="form.data.scenario"
            type="textarea"
            :rows="3"
          />
        </NFormItem>

        <NFormItem label="第一条消息">
          <NInput
            v-model:value="form.data.first_mes"
            type="textarea"
            :rows="4"
          />
        </NFormItem>

        <NFormItem label="示例对话">
          <NInput
            v-model:value="form.data.mes_example"
            type="textarea"
            :rows="4"
          />
        </NFormItem>

        <NFormItem label="系统提示词">
          <NInput
            v-model:value="form.data.system_prompt"
            type="textarea"
            :rows="3"
          />
        </NFormItem>

        <NFormItem label="后历史指令">
          <NInput
            v-model:value="form.data.post_history_instructions"
            type="textarea"
            :rows="2"
          />
        </NFormItem>

        <NFormItem label="创作者备注">
          <NInput
            v-model:value="form.data.creator_notes"
            type="textarea"
            :rows="2"
          />
        </NFormItem>

        <NFormItem label="标签">
          <NInput
            :value="form.data.tags?.join(', ')"
            @update:value="
              (v: string) =>
                (form!.data.tags = v.split(',').map((t) => t.trim()).filter(Boolean))
            "
          />
        </NFormItem>

        <NFormItem label="创作者">
          <NInput v-model:value="form.data.creator" />
        </NFormItem>

        <NFormItem label="角色版本">
          <NInput v-model:value="form.data.character_version" />
        </NFormItem>
      </NForm>

      <!-- Embedded Worldbook Section -->
      <div v-if="hasEmbeddedWorldbook" class="worldbook-section">
        <NText strong>内嵌世界书</NText>
        <NText depth="3">
          该角色卡包含内嵌世界书，点击下方按钮可将其导入为外部世界书。
        </NText>
        <NButton type="info" @click="handleImportWorldbook">
          导入为外部世界书
        </NButton>
      </div>

      <!-- Actions -->
      <NSpace justify="end">
        <NButton @click="emit('close')">取消</NButton>
        <NButton type="primary" @click="handleSave">保存</NButton>
      </NSpace>
    </div>

    <div v-else>
      <NText>加载中...</NText>
    </div>
  </div>
</template>

<style scoped>
.character-editor {
  padding: 16px;
}

.editor-content {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.avatar-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.avatar-preview {
  width: 200px;
  height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f5f5f5;
  border-radius: 8px;
}

.avatar {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.avatar-placeholder {
  color: #999;
}

.worldbook-section {
  padding: 16px;
  background: #f9f9f9;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
</style>