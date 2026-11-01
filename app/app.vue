<script setup lang="ts">
import { addPluginListener } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

const route = useRoute()
const router = useRouter()
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
  window.addEventListener('keydown', searchKey)
  // A reminder opens the app on its subscription (or its snooze sheet):
  // Android notification (cold start or while open), desktop notification.
  const launch = await native.takeLaunchRoute().then(
    (r) => r.route,
    () => null,
  )
  if (launch) await router.push(launch)
  addPluginListener('reminders', 'launchRoute', (e: { route: string }) => router.push(e.route)).catch(() => {})
  listen<string>('open-route', (e) => router.push(e.payload)).catch(() => {})
  // Another device's changes merged (sync): screens reload.
  listen('data-changed', changed).catch(() => {})
  // The first screen's data, then the splash goes; rates catch up after.
  await useOverview().ready()
  releaseSplash()
  void refreshRates()
  window.addEventListener('online', () => void refreshRates())
  // After an update, on the home screen (not over a notification's screen).
  if (route.path === '/') whatsNew.checkAfterUpdate()
})

/** / or Ctrl+F: search (computers), unless typing or a sheet is open. */
function searchKey(e: KeyboardEvent) {
  const k = e.key.toLowerCase()
  const isSearch = k === '/' ? !e.ctrlKey && !e.metaKey : k === 'f' && (e.ctrlKey || e.metaKey)
  if (!isSearch || e.altKey || e.repeat) return
  const el = e.target as HTMLElement | null
  if (el?.closest('input, textarea, select, [contenteditable="true"]') || document.querySelector('.sheet')) return
  e.preventDefault()
  if (route.path !== '/search') router.push('/search')
}
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
