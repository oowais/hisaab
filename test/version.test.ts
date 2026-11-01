import { describe, expect, test } from 'vite-plus/test'
import { compareVersions } from '../app/utils/version'

describe('compareVersions', () => {
  test.each([
    ['0.2.0', '0.1.9', 1],
    ['0.1.9', '0.2.0', -1],
    ['0.10.0', '0.9.0', 1],
    ['0.1.0', '0.1.0', 0],
    ['0.1', '0.1.0', 0],
  ])('%s vs %s', (a, b, want) => expect(compareVersions(a, b)).toBe(want))
})
