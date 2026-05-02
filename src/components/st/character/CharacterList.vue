<script setup lang="ts">
import { ref, onMounted } from 'vue'
import {
  NCard,
  NButton,
  NEmpty,
  NSpin,
  NGrid,
  NGi,
  NUpload,
  NModal,
  NText,
  useMessage,
  type UploadFileInfo,
} from 'naive-ui'
import { useCharactersStore } from '@/stores/characters'
import CharacterEditor from './CharacterEditor.vue'

const store = useCharactersStore()
const message = useMessage()

const showImportModal = ref(false)
const showEditorModal = ref(false)
const selectedCharacterId = ref<string | null>(null)

// Avatar URLs cache
const avatarUrls = ref<Map<string, string>>(new Map())

onMounted(async () => {
  await store.loadCharacters()
})

async function loadAvatarUrl(id: string) {
  const url = await store.getAvatarUrl(id)
  if (url) {
    avatarUrls.value.set(id, url)
  }
}

function handleImportPng(options: { file: UploadFileInfo }) {
  const file = options.file.file
  if (!file) return

  store
    .importFromPng(file)
    .then((result) => {
      message.success(`角色卡 "${result.character.data.name}" 导入成功`)
      if (result.has_embedded_worldbook) {
        message.info('该角色卡包含内嵌世界书，可手动导入')
      }
      showImportModal.value = false
    })
    .catch((e) => {
      message.error(`导入失败: ${e}`)
    })
}

function handleImportJson(options: { file: UploadFileInfo }) {
  const file = options.file.file
  if (!file) return

  store
    .importFromJson(file)
    .then((result) => {
      message.success(`角色卡 "${result.character.data.name}" 导入成功`)
      if (result.has_embedded_worldbook) {
        message.info('该角色卡包含内嵌世界书，可手动导入')
      }
      showImportModal.value = false
    })
    .catch((e) => {
      message.error(`导入失败: ${e}`)
    })
}

function handleEditCharacter(id: string) {
  selectedCharacterId.value = id
  showEditorModal.value = true
}

async function handleExportPng(id: string) {
  try {
    const blob = await store.exportToPng(id)
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${id}.png`
    a.click()
    URL.revokeObjectURL(url)
    message.success('导出成功')
  } catch (e) {
    message.error(`导出失败: ${e}`)
  }
}

async function handleExportJson(id: string) {
  try {
    const blob = await store.exportToJson(id)
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${id}.json`
    a.click()
    URL.revokeObjectURL(url)
    message.success('导出成功')
  } catch (e) {
    message.error(`导出失败: ${e}`)
  }
}

async function handleImportWorldbook(id: string) {
  try {
    const loreId = await store.importWorldbook(id)
    message.success(`世界书导入成功，ID: ${loreId}`)
  } catch (e) {
    message.error(`导入世界书失败: ${e}`)
  }
}

async function handleDeleteCharacter(id: string) {
  try {
    await store.deleteCharacterById(id)
    message.success('删除成功')
  } catch (e) {
    message.error(`删除失败: ${e}`)
  }
}
</script>

<template>
  <div class="character-list">
    <NSpin :show="store.isLoading">
      <!-- Header -->
      <div class="header">
        <NText strong size="large">角色卡列表</NText>
        <NButton type="primary" @click="showImportModal = true">
          导入角色卡
        </NButton>
      </div>

      <!-- Character Grid -->
      <NGrid v-if="store.characters.length > 0" :cols="4" :x-gap="16" :y-gap="16">
        <NGi v-for="(character, index) in store.characters" :key="index">
          <NCard
            :title="character.data.name"
            hoverable
            @click="handleEditCharacter(String(index))"
          >
            <template #cover>
              <div class="avatar-container">
                <img
                  v-if="avatarUrls.get(String(index))"
                  :src="avatarUrls.get(String(index))"
                  class="avatar"
                  @load="loadAvatarUrl(String(index))"
                />
                <div v-else class="avatar-placeholder">
                  <NText>无头像</NText>
                </div>
              </div>
            </template>

            <div class="card-content">
              <NText depth="3" :line-clamp="2">
                {{ character.data.description || '无描述' }}
              </NText>

              <div v-if="character.data.character_book" class="worldbook-badge">
                <NText type="info" size="small">含内嵌世界书</NText>
              </div>
            </div>

            <template #action>
              <div class="card-actions">
                <NButton size="small" @click.stop="handleExportPng(String(index))">
                  导出 PNG
                </NButton>
                <NButton size="small" @click.stop="handleExportJson(String(index))">
                  导出 JSON
                </NButton>
                <NButton
                  v-if="character.data.character_book"
                  size="small"
                  type="info"
                  @click.stop="handleImportWorldbook(String(index))"
                >
                  导入世界书
                </NButton>
                <NButton
                  size="small"
                  type="error"
                  @click.stop="handleDeleteCharacter(String(index))"
                >
                  删除
                </NButton>
              </div>
            </template>
          </NCard>
        </NGi>
      </NGrid>

      <!-- Empty State -->
      <NEmpty v-else description="暂无角色卡，请导入" />

      <!-- Import Modal -->
      <NModal
        v-model:show="showImportModal"
        preset="card"
        title="导入角色卡"
        style="width: 600px"
      >
        <div class="import-options">
          <NUpload
            accept=".png"
            :custom-request="handleImportPng"
            :show-file-list="false"
          >
            <NButton>导入 PNG 角色卡</NButton>
          </NUpload>

          <NUpload
            accept=".json"
            :custom-request="handleImportJson"
            :show-file-list="false"
          >
            <NButton>导入 JSON 角色卡</NButton>
          </NUpload>
        </div>
      </NModal>

      <!-- Editor Modal -->
      <NModal
        v-model:show="showEditorModal"
        preset="card"
        title="编辑角色卡"
        style="width: 800px"
      >
        <CharacterEditor
          v-if="selectedCharacterId"
          :character-id="selectedCharacterId"
        />
      </NModal>
    </NSpin>
  </div>
</template>

<style scoped>
.character-list {
  padding: 24px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.avatar-container {
  width: 100%;
  height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f5f5f5;
}

.avatar {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.avatar-placeholder {
  color: #999;
}

.card-content {
  padding: 8px 0;
}

.worldbook-badge {
  margin-top: 8px;
}

.card-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.import-options {
  display: flex;
  gap: 16px;
  justify-content: center;
}
</style>