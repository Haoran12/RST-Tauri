<template>
  <div class="panel-loading">
    <div class="panel-loading-line panel-loading-title" />
    <div v-for="item in 6" :key="item" class="panel-loading-line" />
  </div>
</template>

<style scoped>
.panel-loading {
  height: 100%;
  min-height: 0;
  padding: 14px 12px;
  display: grid;
  align-content: start;
  gap: 10px;
  background: var(--color-bg-surface, #fff);
}

.panel-loading-line {
  height: 32px;
  border-radius: 8px;
  background: linear-gradient(
    90deg,
    var(--color-bg-subtle, #f5f7fa),
    rgba(24, 160, 88, 0.12),
    var(--color-bg-subtle, #f5f7fa)
  );
  background-size: 220% 100%;
  animation: panel-loading-shimmer 1.2s ease-in-out infinite;
}

.panel-loading-title {
  width: 58%;
}

@keyframes panel-loading-shimmer {
  0% {
    background-position: 120% 0;
  }
  100% {
    background-position: -120% 0;
  }
}
</style>
