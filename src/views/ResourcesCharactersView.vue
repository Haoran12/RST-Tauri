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
  overflow: auto;
}
</style>