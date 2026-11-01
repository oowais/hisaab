import { describe, expect, test } from 'vite-plus/test'
import { formatMoney, minorDigits, parseAmount } from '../app/utils/money'
import { addDays, daysBetween, intervalText, relativeDay } from '../app/utils/days'
import { glyphOn } from '../app/utils/icons'

describe('money', () => {
  test('minor digits', () => {
    expect([minorDigits('EUR'), minorDigits('JPY'), minorDigits('KWD')]).toEqual([2, 0, 3])
  })
  test('format', () => {
    expect(formatMoney(999, 'EUR', { locale: 'en' })).toBe('€9.99')
    expect(formatMoney(1500, 'JPY', { locale: 'en' })).toBe('¥1,500')
    expect(formatMoney(1200, 'EUR', { locale: 'en', compact: true })).toBe('€12')
  })
  test.each([
    ['9.99', 'EUR', 999],
    ['9,99', 'EUR', 999],
    ['9,9', 'EUR', 990],
    ['12', 'EUR', 1200],
    ['1.500,50', 'EUR', 150050],
    ['1,500.50', 'USD', 150050],
    ['1.500', 'EUR', 150000],
    ['1500', 'JPY', 1500],
    ['1,500', 'JPY', 1500],
    ['1.234', 'KWD', 1234],
    ['abc', 'EUR', null],
    ['', 'EUR', null],
  ])('parse %s %s', (text, cur, want) => expect(parseAmount(text, cur as string)).toBe(want))
})

describe('days', () => {
  test('arithmetic across months', () => {
    expect(addDays('2026-01-31', 1)).toBe('2026-02-01')
    expect(daysBetween('2026-02-27', '2026-03-01')).toBe(2)
  })
  test('relative labels', () => {
    expect(relativeDay('2026-03-10', '2026-03-10')).toBe('today')
    expect(relativeDay('2026-03-11', '2026-03-10')).toBe('tomorrow')
    expect(relativeDay('2026-03-13', '2026-03-10')).toBe('in 3 days')
    expect(relativeDay('2026-03-08', '2026-03-10')).toBe('2 days ago')
  })
  test('intervals', () => {
    expect([intervalText(1, 'month'), intervalText(3, 'month'), intervalText(1, 'year')]).toEqual(['monthly', 'every 3 months', 'yearly'])
  })
})

test('glyph colour on brand tiles', () => {
  expect([glyphOn('#000000'), glyphOn('#F8F8F5'), glyphOn('#E50914'), glyphOn('#FFB319')]).toEqual(['#fff', '#111', '#fff', '#111'])
})

test('template search: word starts first, short queries not mid-word', async () => {
  const { searchTemplates } = await import('../app/utils/templates')
  const names = (q: string) => searchTemplates(q).map((t) => t.name)
  expect(names('net')[0]).toBe('Netflix')
  expect(names('tv')).not.toContain('Haftpflichtversicherung')
  expect(names('tv')).toContain('Apple TV+')
  expect(names('versicherung')).toContain('Hausratversicherung')
  expect(names('strom')).toEqual(['Electricity'])
})
