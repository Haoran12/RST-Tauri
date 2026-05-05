<script setup lang="ts">
import CharacterList from '@/components/st/character/CharacterList.vue'
</script>

<template>
  <div class="characters-view">
    <CharacterList />
  </div>
</template>

<style scoped>
.characters-view {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>