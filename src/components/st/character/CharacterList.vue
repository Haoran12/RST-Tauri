<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  NButton,
  NEmpty,
  NSpin,
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
const route = useRoute()
const router = useRouter()

const showImportModal = ref(false)

onMounted(async () => {
  await store.loadCharacters()
})

const selectedCharacterId = computed(() => {
  const value = route.query.character
  return typeof value === 'string' ? value : null
})

const selectedCharacter = computed(() =>
  store.characters.find(item => item.id === selectedCharacterId.value) ?? null,
)

const selectedName = computed(() =>
  selectedCharacter.value?.character.data.name ?? '角色卡详情',
)

async function selectCharacter(id: string | null) {
  await router.replace({
    name: 'resources-characters',
    query: id ? { character: id } : {},
  })
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
      void selectCharacter(result.id)
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
      void selectCharacter(result.id)
    })
    .catch((e) => {
      message.error(`导入失败: ${e}`)
    })
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
    if (selectedCharacterId.value === id) {
      await selectCharacter(null)
    }
    message.success('删除成功')
  } catch (e) {
    message.error(`删除失败: ${e}`)
  }
}
</script>

<template>
  <div class="character-list">
    <NSpin :show="store.isLoading">
      <div class="header">
        <div class="header-main">
          <NText strong size="large">{{ selectedName }}</NText>
          <NText depth="3" size="small">
            {{ selectedCharacter ? '从左侧角色卡列表切换当前内容' : '从左侧选择一个角色卡查看内容' }}
          </NText>
        </div>
        <div class="header-actions">
          <NButton
            v-if="selectedCharacterId"
            secondary
            @click="handleExportPng(selectedCharacterId)"
          >
            导出 PNG
          </NButton>
          <NButton
            v-if="selectedCharacterId"
            secondary
            @click="handleExportJson(selectedCharacterId)"
          >
            导出 JSON
          </NButton>
          <NButton
            v-if="selectedCharacterId && selectedCharacter?.character.data.character_book"
            secondary
            type="info"
            @click="handleImportWorldbook(selectedCharacterId)"
          >
            导入世界书
          </NButton>
          <NButton
            v-if="selectedCharacterId"
            secondary
            type="error"
            @click="handleDeleteCharacter(selectedCharacterId)"
          >
            删除
          </NButton>
          <NButton type="primary" @click="showImportModal = true">
            导入角色卡
          </NButton>
        </div>
      </div>

      <div class="editor-panel">
        <CharacterEditor
          v-if="selectedCharacterId"
          :character-id="selectedCharacterId"
          :close-on-save="false"
          @close="selectCharacter(null)"
        />
        <NEmpty
          v-else-if="store.characters.length > 0"
          description="从左侧选择一个角色卡"
        />
        <NEmpty
          v-else
          description="暂无角色卡，请导入"
        />
      </div>

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
    </NSpin>
  </div>
</template>

<style scoped>
.character-list {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 24px;
}

.character-list :deep(.n-spin-container),
.character-list :deep(.n-spin-content) {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 16px;
  flex-shrink: 0;
}

.header-main {
  display: grid;
  gap: 4px;
  min-width: 0;
}

.header-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
  flex-shrink: 0;
}

.editor-panel {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
  background: var(--color-bg-surface, #fff);
  border: 1px solid var(--color-border-subtle, #e0e0e6);
  border-radius: 8px;
  scrollbar-width: thin;
  scrollbar-gutter: stable;
}

.editor-panel::-webkit-scrollbar {
  width: 8px;
}

.editor-panel::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.05);
  border-radius: 4px;
}

.editor-panel::-webkit-scrollbar-thumb {
  background: rgba(128, 128, 128, 0.5);
  border-radius: 4px;
  min-height: 30px;
}

.editor-panel::-webkit-scrollbar-thumb:hover {
  background: rgba(128, 128, 128, 0.7);
}

.import-options {
  display: flex;
  gap: 16px;
  justify-content: center;
}
</style>
