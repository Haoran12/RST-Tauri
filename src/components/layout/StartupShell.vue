<template>
  <div class="startup-shell" aria-label="RST 正在启动">
    <aside class="startup-nav">
      <div class="startup-logo">R</div>
      <div v-for="item in 8" :key="item" class="startup-nav-dot" />
    </aside>
    <main class="startup-main">
      <header class="startup-header">
        <div>
          <div class="startup-title">RST</div>
          <div class="startup-subtitle">正在加载工作区...</div>
        </div>
        <div class="startup-pill" />
      </header>
      <section class="startup-content">
        <div class="startup-card startup-card-wide" />
        <div class="startup-card" />
        <div class="startup-card" />
        <div class="startup-card startup-card-wide" />
      </section>
    </main>
  </div>
</template>

<style scoped>
.startup-shell {
  height: 100vh;
  width: 100vw;
  display: flex;
  overflow: hidden;
  color: var(--color-text-primary, #1f2937);
  background: var(--color-bg-app, #f0f2f5);
}

.startup-nav {
  width: 72px;
  height: 100%;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 16px 0;
  background: var(--color-bg-surface, #fff);
  border-right: 1px solid var(--color-border-subtle, #e0e0e6);
}

.startup-logo {
  width: 38px;
  height: 38px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  font-weight: 700;
  color: #fff;
  background: #18a058;
}

.startup-nav-dot {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: var(--color-bg-subtle, #f5f7fa);
}

.startup-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.startup-header {
  min-height: 82px;
  padding: 20px 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  background: var(--color-bg-surface, #fff);
  border-bottom: 1px solid var(--color-border-subtle, #e0e0e6);
}

.startup-title {
  font-size: 20px;
  font-weight: 700;
}

.startup-subtitle {
  margin-top: 4px;
  color: var(--color-text-secondary, #6b7280);
}

.startup-pill {
  width: 132px;
  height: 30px;
  border-radius: 8px;
  background: linear-gradient(
    90deg,
    var(--color-bg-subtle, #f5f7fa),
    rgba(24, 160, 88, 0.18),
    var(--color-bg-subtle, #f5f7fa)
  );
  background-size: 220% 100%;
  animation: startup-shimmer 1.2s ease-in-out infinite;
}

.startup-content {
  padding: 18px 20px;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.startup-card {
  min-height: 142px;
  border-radius: 8px;
  background: linear-gradient(
    90deg,
    var(--color-bg-surface, #fff),
    var(--color-bg-subtle, #f5f7fa),
    var(--color-bg-surface, #fff)
  );
  background-size: 220% 100%;
  border: 1px solid var(--color-border-subtle, #e0e0e6);
  animation: startup-shimmer 1.2s ease-in-out infinite;
}

.startup-card-wide {
  grid-column: 1 / -1;
}

@keyframes startup-shimmer {
  0% {
    background-position: 120% 0;
  }
  100% {
    background-position: -120% 0;
  }
}

@media (max-width: 760px) {
  .startup-content {
    grid-template-columns: 1fr;
  }
}
</style>
