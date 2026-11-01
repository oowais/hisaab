<script setup lang="ts">
import { getVersion } from '@tauri-apps/api/app'
import { AlarmClock, BatteryFull, Bell, BellRing, Check, Clock, Coins, Download, Power, WifiOff } from 'lucide-vue-next'
import type { DesktopPrefs, ReminderStatus, Settings } from '~/composables/api'

const version = ref('')
onMounted(async () => (version.value = await getVersion().catch(() => '')))
const update = useUpdate()
const updateNote = ref('')
const updateFailed = ref(false)
async function checkUpdate() {
  updateNote.value = ''
  updateFailed.value = false
  try {
    await update.check(true)
    if (update.pending.value) updateNote.value = `Hisaab ${update.pending.value} is still being built. Try again in a few minutes.`
    else if (!update.available.value) updateNote.value = 'Up to date'
  } catch (e) {
    console.warn('update check:', e)
    updateNote.value = /HTTP 403|rate/i.test(String(e))
      ? 'GitHub is busy. Try again in an hour.'
      : 'Couldn’t check for updates. Are you online?'
    updateFailed.value = true
  }
}
async function installUpdate() {
  updateNote.value = ''
  updateFailed.value = false
  try {
    const r = await update.install()
    if (r.needsPermission) updateNote.value = 'Allow Hisaab to install apps, then tap Update again'
  } catch (e) {
    console.warn('update:', e)
    updateNote.value = 'Update failed. Try again, or get it from the releases page.'
    updateFailed.value = true
  }
}
const { theme, setTheme } = useTheme()

// ---- Money and reminders (synced settings)
const settings = ref<Settings | null>(null)
const error = ref('')
onMounted(async () => (settings.value = await api.settings().catch(() => null)))
const pickingCurrency = ref(false)
const currencyName = computed(() => {
  const c = settings.value?.mainCurrency
  return c ? (new Intl.DisplayNames(undefined, { type: 'currency' }).of(c) ?? c) : ''
})
async function set(key: 'main_currency' | 'default_remind_days' | 'reminder_time', value: string | number) {
  try {
    await api.updateSetting(key, value)
    settings.value = await api.settings()
    error.value = ''
    changed()
  } catch (e) {
    error.value = friendlyError(e, 'change the setting')
  }
}
const REMIND = [0, 1, 2, 3, 5, 7, 14]
const pickingTime = ref(false)

// ---- Android: what reminders need. Re-read when back from the system's settings.
const status = ref<ReminderStatus | null>(null)
const loadStatus = async () => (status.value = await native.status().catch(() => null))
const onVisible = () => document.visibilityState === 'visible' && loadStatus()
onMounted(() => {
  loadStatus()
  document.addEventListener('visibilitychange', onVisible)
})
onUnmounted(() => document.removeEventListener('visibilitychange', onVisible))
async function allow(f: () => Promise<void>) {
  await f().catch(() => {})
  setTimeout(loadStatus, 800)
}

// ---- Desktop: this computer's startup
const desktop = ref<DesktopPrefs | null>(null)
onMounted(async () => (desktop.value = await api.desktopPrefs().catch(() => null)))
async function setDesktop(f: () => Promise<void>) {
  try {
    await f()
    desktop.value = await api.desktopPrefs()
  } catch (e) {
    error.value = friendlyError(e, 'change the setting')
  }
}
</script>

<template>
  <Screen title="Settings">
    <p v-if="error" class="error">{{ error }}</p>
    <template v-if="settings">
      <h2 class="section-title">Money</h2>
      <div class="pane rows">
        <button class="item" @click="pickingCurrency = true">
          <Coins :size="20" class="muted" />
          <span class="flex stack">
            <span>Main currency</span>
            <span class="muted small">Totals and converted amounts</span>
          </span>
          <span class="muted">{{ settings.mainCurrency }} · {{ currencyName }}</span>
        </button>
      </div>

      <h2 class="section-title">Reminders</h2>
      <div class="pane rows">
        <div class="item remind">
          <Bell :size="20" class="muted" />
          <span class="flex">Remind before a charge</span>
        </div>
        <div class="chips remind-chips" role="radiogroup" aria-label="Days before">
          <button
            v-for="n in REMIND"
            :key="n"
            class="chip"
            role="radio"
            :aria-checked="settings.defaultRemindDays === n"
            :class="{ chosen: settings.defaultRemindDays === n }"
            @click="set('default_remind_days', n)"
          >
            {{ n === 0 ? 'Same day' : n === 1 ? '1 day' : `${n} days` }}
          </button>
        </div>
        <button class="item" @click="pickingTime = true">
          <Clock :size="20" class="muted" />
          <span class="flex">At</span>
          <span class="muted">{{ settings.reminderTime }}</span>
        </button>
        <template v-if="status">
          <div
            v-for="p in [
              { icon: BellRing, label: 'Notifications', ok: status.notifications, allow: native.requestNotifications },
              { icon: AlarmClock, label: 'Exact alarms', ok: status.exactAlarms, allow: native.openExactAlarmSettings },
              { icon: BatteryFull, label: 'Unrestricted battery', ok: status.batteryUnrestricted, allow: native.openBatterySettings },
            ]"
            :key="p.label"
            class="item"
          >
            <component :is="p.icon" :size="20" class="muted" />
            <span class="flex">{{ p.label }}</span>
            <span v-if="p.ok" class="checkbox checked"><Check :size="14" :stroke-width="3" /></span>
            <button v-else class="btn" @click="allow(p.allow)">Allow</button>
          </div>
        </template>
      </div>
      <TimeSheet
        v-if="pickingTime"
        :model-value="settings.reminderTime"
        title="Remind at"
        @update:model-value="set('reminder_time', $event)"
        @close="pickingTime = false"
      />

      <template v-if="desktop">
        <h2 class="section-title">Startup</h2>
        <div class="pane rows">
          <div class="item">
            <Power :size="20" class="muted" />
            <span class="flex stack">
              <span>Start at login</span>
              <span class="muted small">So reminders show with the window closed</span>
            </span>
            <button
              class="chip"
              :class="{ chosen: desktop.autostart }"
              :aria-pressed="desktop.autostart"
              @click="setDesktop(() => api.setAutostart(!desktop!.autostart))"
            >
              {{ desktop.autostart ? 'On' : 'Off' }}
            </button>
          </div>
          <div v-if="desktop.autostart" class="item">
            <span class="flex stack">
              <span>Start hidden</span>
              <span class="muted small">In the tray, no window</span>
            </span>
            <button
              class="chip"
              :class="{ chosen: desktop.startHidden }"
              :aria-pressed="desktop.startHidden"
              @click="setDesktop(() => api.setStartHidden(!desktop!.startHidden))"
            >
              {{ desktop.startHidden ? 'On' : 'Off' }}
            </button>
          </div>
        </div>
      </template>
      <CurrencySheet
        v-if="pickingCurrency"
        :model-value="settings.mainCurrency"
        @update:model-value="set('main_currency', $event)"
        @close="pickingCurrency = false"
      />
    </template>

    <h2 class="section-title">Theme</h2>
    <div class="seg" role="radiogroup" aria-label="Theme">
      <button
        v-for="t in ['system', 'light', 'dark'] as const"
        :key="t"
        type="button"
        role="radio"
        class="opt"
        :class="{ chosen: theme === t }"
        :aria-checked="theme === t"
        @click="setTheme(t)"
      >
        {{ { system: 'System', light: 'Light', dark: 'Dark' }[t] }}
      </button>
    </div>

    <template v-if="version">
      <div v-if="update.available.value" class="pane rows update">
        <div class="item">
          <Download :size="20" class="muted" />
          <span class="flex stack">
            <span>Hisaab {{ update.available.value }} is out</span>
            <span class="muted small">{{
              update.canInstall.value ? `You have ${version}` : 'Update with your package manager or the install script'
            }}</span>
          </span>
          <button
            v-if="update.canInstall.value"
            class="btn primary"
            :disabled="update.installing.value || !update.online.value"
            @click="installUpdate"
          >
            {{ update.installing.value ? 'Downloading…' : 'Update' }}
          </button>
        </div>
        <div v-if="update.installing.value" class="progress-row">
          <div
            class="bar"
            role="progressbar"
            aria-label="Downloading update"
            :aria-valuenow="update.progress.value == null ? undefined : Math.round(update.progress.value * 100)"
            aria-valuemin="0"
            aria-valuemax="100"
          >
            <span
              :class="{ indeterminate: update.progress.value == null }"
              :style="update.progress.value == null ? undefined : { width: `${update.progress.value * 100}%` }"
            />
          </div>
          <span class="muted small pct">{{ update.progress.value == null ? '' : `${Math.round(update.progress.value * 100)}%` }}</span>
        </div>
      </div>
      <div class="muted small version" :class="{ tight: update.available.value }">
        <p v-if="!update.available.value">Hisaab {{ version }}</p>
        <button class="btn" :disabled="update.checking.value || update.installing.value || !update.online.value" @click="checkUpdate">
          {{ update.checking.value ? 'Checking…' : 'Check for updates' }}
        </button>
        <button class="whats-new" @click="useWhatsNew().openAll()">What's new</button>
        <p v-if="!update.online.value" class="offline"><WifiOff :size="14" /> No internet connection</p>
        <p v-else-if="updateNote" :class="{ error: updateFailed }">{{ updateNote }}</p>
      </div>
    </template>
  </Screen>
</template>

<style scoped>
.whats-new {
  color: var(--text);
  text-decoration: underline;
  text-underline-offset: 3px;
  font-size: 0.9rem;
  padding: 4px 8px;
}
.item {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 56px;
  width: 100%;
}
.flex {
  flex: 1;
}
.version {
  margin-top: 40px;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}
.version p {
  margin: 0;
}
.version.tight {
  margin-top: 12px;
}
.update {
  margin-top: 40px;
}
.offline {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.update .progress-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-bottom: 12px;
  border-top: 0;
}
.bar {
  flex: 1;
  height: 6px;
  border-radius: 999px;
  overflow: hidden;
  position: relative;
  box-shadow:
    inset 0 1px 1.5px var(--well-shade),
    inset 0 0 0 1px var(--glass-edge);
}
.bar span {
  position: absolute;
  inset: 0 auto 0 0;
  border-radius: inherit;
  background: color-mix(in oklch, var(--accent) var(--primary-solid), transparent);
  transition: width 0.2s ease-out;
}
.bar span.indeterminate {
  width: 35%;
  animation: slide 1.2s ease-in-out infinite;
}
@keyframes slide {
  from {
    left: -35%;
  }
  to {
    left: 100%;
  }
}
@media (prefers-reduced-motion: reduce) {
  .bar span.indeterminate {
    animation: none;
    width: 100%;
    opacity: 0.5;
  }
}
.pct {
  min-width: 3ch;
  text-align: right;
  font-variant-numeric: tabular-nums;
}
.seg {
  display: inline-flex;
  gap: 2px;
  padding: 3px;
  border-radius: 999px;
  box-shadow:
    inset 0 1px 0 var(--glass-control-rim),
    inset 0 0 0 1px var(--line);
}
.opt {
  padding: 6px 14px;
  border-radius: 999px;
  font-size: 0.9rem;
  color: var(--muted);
  white-space: nowrap;
}
.opt.chosen {
  color: var(--text);
}
.stack {
  display: flex;
  flex-direction: column;
  gap: 2px;
  text-align: left;
}
.remind {
  min-height: 48px;
}
.remind-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 0 0 14px;
  border-top: 0;
}
</style>
