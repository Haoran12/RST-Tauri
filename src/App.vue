<script setup lang="ts">
import { NConfigProvider, NMessageProvider, NDialogProvider, darkTheme, lightTheme } from 'naive-ui'
import { computed } from 'vue'
import { useAppShellStore } from '@/stores/appShell'
import AppLayout from '@/components/layout/AppLayout.vue'

const appShell = useAppShellStore()

const theme = computed(() => {
  if (appShell.theme === 'dark') {
    return darkTheme
  }
  if (appShell.theme === 'light') {
    return lightTheme
  }
  // system - for now default to light
  // TODO: implement system theme detection
  return lightTheme
})
</script>

<template>
  <NConfigProvider :theme="theme">
    <NMessageProvider>
      <NDialogProvider>
        <AppLayout />
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
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
