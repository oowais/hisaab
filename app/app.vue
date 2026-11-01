<script setup lang="ts">
const route = useRoute()
// The splash stays until we know which screen opens (see useSplash).
const releaseSplash = holdSplash()
const whatsNew = useWhatsNew()

onMounted(() => {
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
  releaseSplash()
  // After an update, on the home screen (not over a notification's screen).
  if (route.path === '/') whatsNew.checkAfterUpdate()
})
</script>

<template>
  <NuxtPage />
  <TabBar />
  <WhatsNewSheet
    v-if="whatsNew.shown.value"
    :releases="whatsNew.shown.value.releases"
    :since="whatsNew.shown.value.since"
    @close="whatsNew.close"
  />
  <AppToast />
</template>
