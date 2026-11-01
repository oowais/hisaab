// The overview behind Upcoming (and the tab badges later): loaded once,
// reloaded after every write (dataVersion), when the day changes, and when
// new FX rates come in.
import type { Overview } from './api'

const data = ref<Overview | null>(null)
const error = ref('')
const today = ref(todayIso())
let loading: Promise<void> | null = null

async function load() {
  today.value = todayIso()
  try {
    data.value = await api.overview(today.value)
    error.value = ''
  } catch (e) {
    error.value = friendlyError(e, 'load your payments')
  }
}

/** Latest rates at most once a day, when online; reloads when they arrive. */
export async function refreshRates() {
  if (typeof navigator !== 'undefined' && !navigator.onLine) return
  if (await api.refreshRates(todayIso()).catch(() => false)) await load()
}

export function useOverview() {
  watch(dataVersion, () => void load())
  if (!data.value && !loading) loading = load().finally(() => (loading = null))
  return { data, error, today, reload: load, ready: () => loading ?? Promise.resolve() }
}
