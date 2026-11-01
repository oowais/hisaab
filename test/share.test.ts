import { describe, expect, test } from 'vite-plus/test'
import { fromShare } from '../app/utils/share'

describe('share', () => {
  test('a link to a known site finds its template', () => {
    const s = fromShare('https://www.netflix.com/youraccount')
    expect(s.template?.key).toBe('netflix')
    expect(s.name).toBe('Netflix')
    expect(s.url).toBe('https://www.netflix.com/youraccount')
    expect(s.amount).toBeNull()
  })
  test('a name in the text finds the template, longest name first', () => {
    expect(fromShare('Your Apple TV+ plan renews soon').template?.key).toBe('apple-tv')
    expect(fromShare('Thanks for joining Spotify Premium').template?.key).toBe('spotify')
    // Not inside another word.
    expect(fromShare('maximum effort').template).toBeNull()
  })
  test('amounts with symbol or code, before or after', () => {
    expect(fromShare('Your plan renews at €13.99 on 5 Dec').amount).toEqual({ minor: 1399, currency: 'EUR' })
    expect(fromShare('Betrag: 13,99 €').amount).toEqual({ minor: 1399, currency: 'EUR' })
    expect(fromShare('Total USD 1,299.00').amount).toEqual({ minor: 129900, currency: 'USD' })
    expect(fromShare('Charged 1500 JPY').amount).toEqual({ minor: 1500, currency: 'JPY' })
    expect(fromShare('Since 2024, no price here').amount).toBeNull()
    expect(fromShare('THE 5 best plans').amount).toBeNull()
  })
  test('unknown: subject, else first line, else the site', () => {
    expect(fromShare('https://example.org/x', 'Allotment garden fee').name).toBe('Allotment garden fee')
    expect(fromShare('Piano lessons\nsecond line').name).toBe('Piano lessons')
    expect(fromShare('https://account.proton.me/dashboard').name).toBe('Proton')
  })
})
