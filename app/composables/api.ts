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
  /** Shared: the percent that's yours (1–99); null = all of it. */
  myShare: number | null
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
  /** A one-time charge (not from the schedule): its id; `periodId` is ''. */
  extraId: string | null
  /** A one-time charge's note. */
  label: string | null
}

/** A one-time charge on a subscription, besides its schedule. */
export type ExtraCharge = {
  id: string
  subscriptionId: string
  dueDate: string
  amountMinor: number
  currency: string
  label: string
}
export type NewExtraCharge = Omit<ExtraCharge, 'id'>

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
  monthlyBudget: number | null
}

export type TimelineLane = SubscriptionView & { periods: Period[] }

export type HistoryCharge = Charge & { inMain: number | null; overrideId: string | null }

export type Detail = SubscriptionView & {
  periods: Period[]
  history: HistoryCharge[]
  /** Coming charges to the end of next month, soonest first. */
  coming: HistoryCharge[]
  totalSpent: number
  excludedCurrencies: string[]
  historyFrom: string | null
}

export type Category = { id: string; name: string; color: string; sort: number }
export type PaymentMethod = { id: string; name: string }
export type Settings = {
  mainCurrency: string
  remindersOn: boolean
  defaultRemindDays: number
  lastCurrency: string | null
  reminderTime: string
  /** Most payments should cost a month, minor units of the main currency; null = none. */
  monthlyBudget: number | null
}
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

export type MonthCharges = { charges: UpcomingCharge[]; total: number; excludedCurrencies: string[] }
export type Slice = { key: string | null; amount: number }
export type MonthSpend = { month: string; total: number; byCategory: Slice[] }
export type Stats = {
  mainCurrency: string
  months: MonthSpend[]
  /** Paid in the months above; this month and this year so far. */
  total: number
  thisMonth: number
  thisYear: number
  byCategory: Slice[]
  byMethod: Slice[]
  byCurrency: Slice[]
  saved: number
  excludedCurrencies: string[]
  from: string | null
  monthlyBudget: number | null
}

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
  archived: (today: string) => invoke<SubscriptionView[]>('archived', { today }),
  search: (query: string, today: string) => invoke<SubscriptionView[]>('search', { query, today }),
  stats: (today: string, months: number) => invoke<Stats>('stats', { today, months }),
  monthCharges: (day: string, today: string) => invoke<MonthCharges>('month_charges', { day, today }),
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
  setOverride: (o: NewOverride) => invoke<{ id: string }>('set_override', { o }),
  deleteOverride: (id: string) => invoke<void>('delete_override', { id }),
  addExtra: (e: NewExtraCharge) => invoke<ExtraCharge>('add_extra', { e }),
  updateExtra: (e: ExtraCharge) => invoke<void>('update_extra', { e }),
  deleteExtra: (id: string) => invoke<void>('delete_extra', { id }),
  restoreExtra: (id: string) => invoke<void>('restore_extra', { id }),
  addCategory: (name: string, color = '') => invoke<Category>('add_category', { name, color }),
  ensureCategory: (id: string, name: string, color: string) => invoke<Category>('ensure_category', { id, name, color }),
  addPaymentMethod: (name: string) => invoke<PaymentMethod>('add_payment_method', { name }),
  updateCategory: (cat: Category) => invoke<void>('update_category', { cat }),
  deleteCategory: (id: string) => invoke<string[]>('delete_category', { id }),
  restoreCategory: (id: string, subs: string[]) => invoke<void>('restore_category', { id, subs }),
  sortCategories: (ids: string[]) => invoke<void>('sort_categories', { ids }),
  renamePaymentMethod: (id: string, name: string) => invoke<void>('rename_payment_method', { id, name }),
  deletePaymentMethod: (id: string) => invoke<string[]>('delete_payment_method', { id }),
  restorePaymentMethod: (id: string, subs: string[]) => invoke<void>('restore_payment_method', { id, subs }),
  updateSetting: (key: string, value: unknown) => invoke<void>('update_setting', { key, value }),
  monthlyInMain: (amountMinor: number, currency: string, everyN: number, unit: Unit) =>
    invoke<number | null>('monthly_in_main', { amountMinor, currency, everyN, unit }),
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

/** A web page in the browser (desktop: the app's command; Android: the plugin's). */
export const openUrl = (url: string) => invoke<void>(isAndroid ? 'plugin:reminders|open_url' : 'open_url', { url })

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
