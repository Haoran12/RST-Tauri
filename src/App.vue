<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import StartupShell from '@/components/layout/StartupShell.vue'

const AppProviders = defineAsyncComponent({
  loader: () => import('@/AppProviders.vue'),
  loadingComponent: StartupShell,
  errorComponent: StartupShell,
  delay: 0,
  timeout: 20000,
})
</script>

<template>
  <AppProviders />
</template>

<style>
html, body, #app {
  margin: 0;
  padding: 0;
  height: 100%;
  width: 100%;
  overflow: hidden;
}
</style>
