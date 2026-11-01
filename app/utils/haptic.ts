/** A short buzz (Android) for long-press and swipe thresholds. */
export const haptic = () => {
  try {
    navigator.vibrate?.(12)
  } catch {
    /* not supported */
  }
}
