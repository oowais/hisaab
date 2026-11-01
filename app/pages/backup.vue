<script setup lang="ts">
// Daily backup to a folder, export a copy, restore from a backup (Befikr's).
import { Check, Download, FolderSync, RefreshCw, Upload } from 'lucide-vue-next'
import type { BackupStatus } from '~/composables/api'

const { show } = useToast()
const error = ref('')
const backup = ref<BackupStatus | null>(null)
const busy = ref(false)
const importPath = ref<string | null>(null)

const load = async () =>
  (backup.value = await backups.status().catch((e) => ((error.value = friendlyError(e, 'load the backup status')), null)))
onMounted(load)

async function toggle() {
  if (backup.value?.folder) return action(backups.clearFolder, 'Daily backup off')
  await action(backups.pickFolder)
  if (backup.value?.folder) show({ message: 'Daily backup on' })
}

async function action(f: () => Promise<BackupStatus>, done?: string) {
  busy.value = true
  error.value = ''
  try {
    backup.value = await f()
    if (done) show({ message: done })
  } catch (e) {
    error.value = friendlyError(e, 'back up')
  } finally {
    busy.value = false
    backup.value = await backups.status().catch(() => backup.value)
  }
}
async function exportCopy() {
  const r = await backups.exportCopy().catch((e) => ((error.value = friendlyError(e, 'export a copy')), null))
  if (r?.saved) show({ message: 'Backup saved' })
}
async function chooseImport() {
  const r = await backups.pickImportFile().catch((e) => ((error.value = friendlyError(e, 'open the file')), null))
  if (r?.path) importPath.value = r.path
}
async function confirmImport() {
  const path = importPath.value
  importPath.value = null
  if (!path) return
  try {
    await api.importBackup(path)
    changed()
    await load()
    show({ message: 'Backup restored' })
  } catch (e) {
    error.value = friendlyError(e, 'restore the backup')
  }
}
</script>

<template>
  <Screen title="Backup">
    <p v-if="error" class="error">{{ error }}</p>

    <template v-if="backup">
      <div class="pane rows">
        <div class="item">
          <FolderSync :size="20" class="muted" />
          <span class="flex stack">
            <span>Daily backup</span>
            <span v-if="backup.folder" class="muted small">{{ isAndroid ? 'Every night' : 'Once a day' }}, keeps the newest 7</span>
            <span v-else class="muted small">Saves a copy each {{ isAndroid ? 'night' : 'day' }} to a folder you pick, e.g. Syncthing</span>
          </span>
          <button class="chip" :class="{ chosen: backup.folder }" :aria-pressed="!!backup.folder" :disabled="busy" @click="toggle">
            {{ backup.folder ? 'On' : 'Off' }}
          </button>
        </div>
        <div v-if="backup.folder" class="details">
          <div class="detail">
            <span class="muted">Folder</span>
            <span class="flex value">{{ backup.folder }}</span>
            <button class="btn" :disabled="busy" @click="action(backups.pickFolder)">Change</button>
          </div>
          <div class="detail">
            <span class="muted">Last</span>
            <span class="flex stack value">
              <span v-if="backup.lastError" class="error">{{ friendlyBackupError(backup.lastError) }}</span>
              <span v-else-if="backup.lastAt" class="ok"><Check :size="15" :stroke-width="3" /> {{ instantLabel(backup.lastAt) }}</span>
              <span v-else>Not yet</span>
            </span>
            <button class="btn" :disabled="busy" @click="action(backups.now, 'Backed up')">
              <RefreshCw :size="16" :class="{ spin: busy }" /> Back up now
            </button>
          </div>
          <p class="muted small note">Turning it off keeps the backups already in the folder.</p>
        </div>
        <button class="item" @click="exportCopy"><Upload :size="20" class="muted" /><span class="flex left">Export a copy…</span></button>
        <button class="item" @click="chooseImport">
          <Download :size="20" class="muted" /><span class="flex left">Restore from a backup…</span>
        </button>
      </div>
      <div v-if="importPath" class="pane confirm">
        <b>Replace all payments with this backup?</b>
        <span class="muted small">What’s here now is saved first, just in case.</span>
        <div class="row end">
          <button class="btn" @click="importPath = null">Cancel</button>
          <button class="btn primary" @click="confirmImport">Replace</button>
        </div>
      </div>
    </template>
  </Screen>
</template>

<style scoped>
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
.left {
  text-align: left;
}
.row {
  display: flex;
  gap: 8px;
}
.stack {
  display: flex;
  flex-direction: column;
  gap: 2px;
  text-align: left;
}
.confirm {
  margin-top: 10px;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.end {
  justify-content: flex-end;
}
.details {
  padding: 4px 0 12px 32px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.detail {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px 12px;
  min-height: 44px;
}
.detail > .btn {
  margin-left: auto;
}
.detail > .muted:first-child {
  width: 48px;
  flex: none;
  font-size: 0.9rem;
}
.value {
  min-width: 8rem;
  overflow-wrap: anywhere;
}
.ok {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
}
.note {
  margin: 0;
}
.spin {
  animation: spin 1s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
