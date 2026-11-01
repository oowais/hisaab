// Wide window (desktop): Upcoming and a payment's Detail side by side.
// Phones and narrow windows keep separate screens.
export const WIDE = '(min-width: 1100px)'

const wide = ref(typeof window !== 'undefined' && window.matchMedia(WIDE).matches)
let watching = false

export function useWide() {
  if (!watching && typeof window !== 'undefined') {
    watching = true
    window.matchMedia(WIDE).addEventListener('change', (e) => (wide.value = e.matches))
  }
  return wide
}
