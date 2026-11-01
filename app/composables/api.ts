// Typed wrappers around the Tauri commands in src-tauri/src/lib.rs. Dates
// are local calendar days, 'YYYY-MM-DD'; money is integer minor units.
import { invoke } from '@tauri-apps/api/core'

export const isAndroid = typeof navigator !== 'undefined' && /Android/i.test(navigator.userAgent)

export type Kind = 'subscription' | 'bill'
export type Unit = 'day' | 'week' | 'month' | 'year'
export type PeriodStatus = 'trial' | 'active' | 'paused'
export type Status = 'trial' | 'active' | 'paused' | 'ended'
export type OverrideKind = 'skipped' | 'moved' | 'amount_changed' | 'settled'
export type Action = 'price' | 'pause' | 'resume' | 'cancel' | 'restart' | 'convert'

export type Terms = {
  amountMinor: number
  currency: string
  everyN: number
  unit: Unit
  planLabel: string | null
  amountKind: 'fixed' | 'estimated'
}

export type Period = Terms & {
  id: string
  subscriptionId: string
  status: PeriodStatus
  anchorDate: string
  startDate: string
  endDate: string | null
}

export type SubscriptionFields = {
  kind: Kind | null
  name: string
  templateKey: string | null
  icon: string | null
  color: string | null
  categoryId: string | null
  paymentMethodId: string | null
  url: string | null
  notes: string
  remindDaysBefore: number | null
}

export type Subscription = SubscriptionFields & { id: string; kind: Kind; archivedAt: number | null }

export type Charge = {
  subscriptionId: string
  periodId: string
  dueDate: string
  scheduledDate: string
  amountMinor: number
  currency: string
  settledAmountMinor: number | null
  settledCurrency: string | null
  estimated: boolean
  status: 'upcoming' | 'paid'
  isOverride: boolean
}

export type SubscriptionView = Subscription & {
  status: Status
  current: Period | null
  nextCharge: Charge | null
  nextInMain: number | null
  yearlyInMain: number | null
}

export type UpcomingCharge = Charge & { inMain: number | null }

export type Overview = {
  mainCurrency: string
  subscriptions: SubscriptionView[]
  upcoming: UpcomingCharge[]
  yearlyTotal: number
  excludedCurrencies: string[]
  ratesDate: string | null
  ratesStale: boolean
}

export type TimelineLane = SubscriptionView & { periods: Period[] }

export type HistoryCharge = Charge & { inMain: number | null; overrideId: string | null }

export type Detail = SubscriptionView & {
  periods: Period[]
  history: HistoryCharge[]
  totalSpent: number
  excludedCurrencies: string[]
  historyFrom: string | null
}

export type Category = { id: string; name: string; color: string; sort: number }
export type PaymentMethod = { id: string; name: string }
export type Settings = { mainCurrency: string; defaultRemindDays: number; lastCurrency: string | null; reminderTime: string }
export type SnoozePick = { label: string; atMs: number }
export type DesktopPrefs = { autostart: boolean; startHidden: boolean }
/** Android: what reminders need (desktop: none of this). */
export type ReminderStatus = { notifications: boolean; exactAlarms: boolean; batteryUnrestricted: boolean }

export type SyncMode = 'off' | 'relay'
export type Peer = {
  endpoint: string
  device: string
  name: string
  theirSeq: number
  pairedAt: number
  lastSyncAt: number | null
  /** 'phone' | 'computer', '' until that device has told us. */
  kind: string
}
export type SyncStatus = {
  mode: SyncMode
  name: string
  running: boolean
  syncing: boolean
  endpoint: string
  peers: Peer[]
  pairing: { untilMs: number; link: string } | null
  /** Worded for the user. */
  lastError: string | null
  /** The same, raw (network / protocol). */
  lastErrorDetail: string | null
  conflicts: number
}
/** A period as an overlap conflict shows it. */
export type ConflictPeriod = {
  start: string
  end: string | null
  amountMinor: number
  currency: string
  everyN: number
  unit: Unit
  status: PeriodStatus
}
export type Conflict = {
  id: string
  table: string
  rowId: string
  /** A column, or 'overlap' (two periods covering the same days; values are ConflictPeriods). */
  field: string
  label: string
  lostValue: unknown
  wonValue: unknown
  wonBy: string | null
  at: number
}
export type BackupStatus = { folder: string | null; lastAt: number | null; lastError: string | null }

export type NewSubscription = {
  fields: SubscriptionFields
  terms: Terms
  startDate: string
  anchorDate: string | null
  trialEnd: string | null
}

export type NewOverride = {
  periodId: string
  dueDate: string
  kind: OverrideKind
  newDueDate?: string | null
  actualAmountMinor?: number | null
  actualCurrency?: string | null
  settledAmountMinor?: number | null
  settledCurrency?: string | null
}

export const api = {
  overview: (today: string) => invoke<Overview>('overview', { today }),
  timeline: (today: string) => invoke<TimelineLane[]>('timeline', { today }),
  detail: (id: string, today: string) => invoke<Detail>('detail', { id, today }),
  defaults: (id: string, today: string) => invoke<{ nextCharge: string | null; cycleEnd: string }>('defaults', { id, today }),
  categories: () => invoke<Category[]>('categories'),
  paymentMethods: () => invoke<PaymentMethod[]>('payment_methods'),
  settings: () => invoke<Settings>('settings'),
  addSubscription: (n: NewSubscription) => invoke<string>('add_subscription', { new: n }),
  updateSubscription: (id: string, fields: SubscriptionFields) => invoke<void>('update_subscription', { id, fields }),
  changeTerms: (id: string, from: string, terms: Terms) => invoke<void>('change_terms', { id, from, terms }),
  pause: (id: string, from: string, resumeOn: string | null) => invoke<void>('pause', { id, from, resumeOn }),
  resume: (id: string, on: string, anchor: string | null = null) => invoke<void>('resume', { id, on, anchor }),
  cancel: (id: string, endsOn: string) => invoke<void>('cancel', { id, endsOn }),
  restart: (id: string, on: string, terms: Terms | null = null) => invoke<void>('restart', { id, on, terms }),
  convertTrial: (id: string, on: string, terms: Terms | null = null) => invoke<void>('convert_trial', { id, on, terms }),
  setArchived: (id: string, archived: boolean) => invoke<void>('set_archived', { id, archived }),
  deleteSubscription: (id: string) => invoke<void>('delete_subscription', { id }),
  restoreSubscription: (id: string) => invoke<void>('restore_subscription', { id }),
  setOverride: (o: NewOverride) => invoke<unknown>('set_override', { o }),
  deleteOverride: (id: string) => invoke<void>('delete_override', { id }),
  addCategory: (name: string, color = '') => invoke<Category>('add_category', { name, color }),
  addPaymentMethod: (name: string) => invoke<PaymentMethod>('add_payment_method', { name }),
  updateSetting: (key: string, value: unknown) => invoke<void>('update_setting', { key, value }),
  snoozePicks: () => invoke<SnoozePick[]>('snooze_picks'),
  snooze: (id: string, untilMs: number) => invoke<void>('snooze', { id, untilMs }),
  reminderAt: (day: string) => invoke<number>('reminder_at', { day }),
  desktopPrefs: () => invoke<DesktopPrefs>('desktop_prefs'),
  setAutostart: (on: boolean) => invoke<void>('set_autostart', { on }),
  setStartHidden: (on: boolean) => invoke<void>('set_start_hidden', { on }),
  refreshRates: (today: string) => invoke<boolean>('refresh_rates', { today }),
  ensureRates: (from: string, today: string) => invoke<boolean>('ensure_rates', { from, today }),
  importBackup: (path: string) => invoke<void>('import_backup', { path }),
  syncStatus: () => invoke<SyncStatus>('sync_status'),
  setSyncMode: (mode: SyncMode) => invoke<void>('set_sync_mode', { mode }),
  setDeviceName: (name: string) => invoke<void>('set_device_name', { name }),
  startPairing: () => invoke<void>('start_pairing'),
  stopPairing: () => invoke<void>('stop_pairing'),
  pairWithLink: (link: string) => invoke<void>('pair_with_link', { link }),
  unpair: (endpoint: string) => invoke<void>('unpair', { endpoint }),
  syncNow: () => invoke<void>('sync_now'),
  conflicts: () => invoke<Conflict[]>('conflicts'),
  resolveConflict: (id: string, useMine: boolean) => invoke<void>('resolve_conflict', { id, useMine }),
}

/** Backups: the app's commands on desktop, the Android plugin's on a phone. */
const backupCmd = (name: string) => (isAndroid ? `plugin:reminders|${name}` : name)
export const backups = {
  status: () => invoke<BackupStatus>(backupCmd('backup_status')),
  pickFolder: () => invoke<BackupStatus>(backupCmd('pick_backup_folder')),
  clearFolder: () => invoke<BackupStatus>(backupCmd('clear_backup_folder')),
  now: () => invoke<BackupStatus>(backupCmd('backup_now')),
  exportCopy: () => invoke<{ saved: boolean }>(backupCmd('export_backup')),
  pickImportFile: () => invoke<{ path?: string }>(backupCmd('pick_import_file')),
}

/** The Android plugin's own commands (they reject on desktop). */
export const native = {
  status: () => invoke<ReminderStatus>('plugin:reminders|status'),
  requestNotifications: () => invoke<void>('plugin:reminders|request_notifications'),
  openExactAlarmSettings: () => invoke<void>('plugin:reminders|open_exact_alarm_settings'),
  openBatterySettings: () => invoke<void>('plugin:reminders|open_battery_settings'),
  takeLaunchRoute: () => invoke<{ route: string | null }>('plugin:reminders|take_launch_route'),
}

/** Bumped after every write: screens reload their data when it changes. */
export const dataVersion = ref(0)
export const changed = () => dataVersion.value++
