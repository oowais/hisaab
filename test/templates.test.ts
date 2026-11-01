import { describe, expect, test } from 'vite-plus/test'
import { iconSpec, PALETTE, randomColor } from '../app/utils/icons'
import { TEMPLATES } from '../app/utils/templates'

describe('templates', () => {
  test('keys are unique', () => {
    expect(new Set(TEMPLATES.map((t) => t.key)).size).toBe(TEMPLATES.length)
  })
  test.each(TEMPLATES.map((t) => [t.key, t] as const))('%s has a bundled icon', (_, t) => {
    expect(iconSpec(t.icon, t.name).kind).not.toBe('letter')
  })
  test('random colour is a tile swatch', () => {
    for (let i = 0; i < 50; i++) expect(PALETTE.slice(0, 8)).toContain(randomColor())
  })
})
