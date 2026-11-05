import { describe, expect, test } from 'claude-code/testing'

import { line } from '../hooks/register'

const now = Date.parse('2026-11-05T12:00:00Z')

describe('quota line', () => {
  test('shows usage and time to reset per window', async () => {
    const text = line(
      {
        rateLimits: [
          { kind: 'five_hour', percentUsed: 42.5, resetsAt: '2026-11-05T14:10:00Z' },
          { kind: 'seven_day', percentUsed: 18, resetsAt: '2026-11-08T13:00:00Z' },
        ],
      },
      null,
      now,
    )
    expect(text).toBe('5h 42.5% · resets in 2h 10m  |  7d 18% · resets in 3d 1h')
  })

  test('shows credit used past the limit once quota is used up', async () => {
    const text = line(
      {
        rateLimits: [{ kind: 'five_hour', percentUsed: 100, resetsAt: '2026-11-05T12:45:00Z' }],
        cost: { usd: 3.75 },
      },
      { baseUsd: 1.5 },
      now,
    )
    expect(text).toBe('⛔ 5h quota used up · credit used this session $2.25 · resets in 45m')
  })

  test('shows nothing without rate-limit readings', async () => {
    expect(line({ rateLimits: [] }, null, now)).toBe(undefined)
  })
})
