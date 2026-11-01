// Saving a category choice: a pre-made one is made the first time it's used.
import { SUGGESTED } from '~/utils/categories'

/** The id to store for `id`; a pre-made one is made (or brought back) first. */
export async function savedCategoryId(id: string | null): Promise<string | null> {
  const s = id ? SUGGESTED.find((x) => x.id === id) : null
  return s ? (await api.ensureCategory(s.id, s.name, s.color)).id : id
}
