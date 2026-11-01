// Charges on screen: scheduled ones (moved by an override) and one-time
// ones (moved by editing them), handled the same way by the lists.
import type { Charge, ExtraCharge } from '~/composables/api'

/** A list key: one-time charges can share a day. */
export const chargeKey = (c: Charge) => c.extraId ?? `${c.periodId}-${c.scheduledDate}`

/** The one-time charge behind `c` (it has `extraId`). */
export const extraOf = (c: Charge): ExtraCharge => ({
  id: c.extraId!,
  subscriptionId: c.subscriptionId,
  dueDate: c.dueDate,
  amountMinor: c.amountMinor,
  currency: c.currency,
  label: c.label ?? '',
})

/** Move a charge to `day`; a toast says so (or why not). */
export async function moveCharge(c: Charge, day: string) {
  const toast = useToast()
  try {
    if (c.extraId) await api.updateExtra({ ...extraOf(c), dueDate: day })
    else await api.setOverride({ periodId: c.periodId, dueDate: c.scheduledDate, kind: 'moved', newDueDate: day })
    changed()
    toast.show({ message: `Moved to ${relativeDay(day)}` })
  } catch (e) {
    toast.show({ message: friendlyError(e, 'move the charge') })
  }
}
