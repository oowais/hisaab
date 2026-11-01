<script setup lang="ts">
const route = useRoute()
// The splash stays until we know which screen opens (see useSplash).
const releaseSplash = holdSplash()
const whatsNew = useWhatsNew()

onMounted(async () => {
  // Desktop: our own right-click menus only (see useContextMenu).
  document.addEventListener('contextmenu', suppressDefaultMenu)
  useUpdate()
    .check()
    .catch(() => {})
  // Long-running desktop windows: the check's own limits keep this cheap.
  window.addEventListener('focus', () =>
    useUpdate()
      .check()
      .catch(() => {}),
  )
  handleBackButton().catch(() => {})
  // The first screen's data, then the splash goes; rates catch up after.
  await useOverview().ready()
  releaseSplash()
  void refreshRates()
  window.addEventListener('online', () => void refreshRates())
  // After an update, on the home screen (not over a notification's screen).
  if (route.path === '/') whatsNew.checkAfterUpdate()
})
</script>

<template>
  <NuxtPage />
  <TabBar />
  <ContextMenu />
  <WhatsNewSheet
    v-if="whatsNew.shown.value"
    :releases="whatsNew.shown.value.releases"
    :since="whatsNew.shown.value.since"
    @close="whatsNew.close"
  />
  <AppToast />
</template>
