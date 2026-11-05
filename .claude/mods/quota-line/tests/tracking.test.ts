import { describe, expect, mock, test } from 'claude-code/testing'
import type { On, SessionRateLimit } from 'claude-code'

const NOW = Date.parse('2026-11-05T12:00:00Z')
const CONTEXT = { window: 200_000 }

const fiveHour = (percentUsed: number): SessionRateLimit => ({
  kind: 'five_hour',
  percentUsed,
  resetsAt: '2026-11-05T12:45:00Z',
})

/** Stands in for the engine: records each status line; `usage` is what `$.session.usage()` reports. */
const engine = (on: On, usage: SessionRateLimit[] = []) => {
  const statuses: string[] = []
  const clock = mock.clock(on, { now: NOW })
  on('ui.status', (_, e) => {
    if (e.text !== undefined) statuses.push(e.text)
    return { value: undefined }
  })
  on('session.usage', () => ({ value: { startedAt: NOW, context: CONTEXT, rateLimits: usage } }))
  on('session.start', (_, e) => ({ cwd: e.cwd }))
  on('session.measure', (_, e) => ({ changed: e.changed }))
  return { statuses, clock }
}

describe('quota line cost tracking', () => {
  test('counts credit from the moment quota runs out', async ($, on) => {
    const { statuses } = engine(on)
    await $.session.measure({ context: CONTEXT, rateLimits: [fiveHour(90)], cost: { usd: 1.5 }, changed: ['cost'] })
    await $.session.measure({ context: CONTEXT, rateLimits: [fiveHour(100)], cost: { usd: 2 }, changed: ['rateLimits'] })
    await $.session.measure({ context: CONTEXT, rateLimits: [fiveHour(100)], cost: { usd: 3.25 }, changed: ['cost'] })

    expect(statuses.at(-2)).toBe('⛔ 5h quota used up · credit used this session $0.00 · resets in 45m')
    expect(statuses.at(-1)).toBe('⛔ 5h quota used up · credit used this session $1.25 · resets in 45m')
  })

  test('starts counting afresh after the quota resets and runs out again', async ($, on) => {
    const { statuses } = engine(on)
    await $.session.measure({ context: CONTEXT, rateLimits: [fiveHour(100)], cost: { usd: 1 }, changed: ['cost'] })
    await $.session.measure({ context: CONTEXT, rateLimits: [fiveHour(5)], cost: { usd: 4 }, changed: ['rateLimits'] })
    expect(statuses.at(-1)).toBe('5h 5% · resets in 45m')

    await $.session.measure({ context: CONTEXT, rateLimits: [fiveHour(100)], cost: { usd: 6 }, changed: ['rateLimits'] })
    await $.session.measure({ context: CONTEXT, rateLimits: [fiveHour(100)], cost: { usd: 6.5 }, changed: ['cost'] })
    expect(statuses.at(-1)).toBe('⛔ 5h quota used up · credit used this session $0.50 · resets in 45m')
  })

  test('keeps the cost at exhaustion across a reload', async ($, on) => {
    const { statuses } = engine(on)
    on('state.get', () => ({ value: { value: { baseUsd: 2 }, version: 1 } }))
    await $.session.measure({ context: CONTEXT, rateLimits: [fiveHour(100)], cost: { usd: 2.75 }, changed: ['cost'] })

    expect(statuses.at(-1)).toBe('⛔ 5h quota used up · credit used this session $0.75 · resets in 45m')
  })

  test('the countdown refreshes every minute without a new measurement', async ($, on) => {
    const { statuses, clock } = engine(on, [fiveHour(40)])
    await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
    expect(statuses.at(-1)).toBe('5h 40% · resets in 45m')

    await clock.advance(60_000)
    expect(statuses.at(-1)).toBe('5h 40% · resets in 44m')
  })
})
