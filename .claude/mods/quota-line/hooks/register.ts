import { atom, read, update } from 'claude-code'
import type { EngineInterface as Engine, Register, SessionRateLimit } from 'claude-code'

import type { Overage } from '../types'

/** The figures the line is drawn from (`session.measure`'s input or `$.session.usage()`). */
type Figures = { rateLimits: SessionRateLimit[]; cost?: { usd: number } }

const overage = atom({ plugin: 'quota-line', key: 'overage' } as const, null as Overage | null)

const LABELS: Record<string, string> = { five_hour: '5h', seven_day: '7d', spend_limit: 'spend' }

const label = (kind: string): string => LABELS[kind] ?? kind

const usd = (n: number): string => `$${n.toFixed(2)}`

const isExhausted = (w: SessionRateLimit): boolean => w.percentUsed >= 100

/** " · resets in 2h 10m" for an ISO time, relative to `now` (ms); empty when unknown. */
export const resetIn = (iso: string | undefined, now: number): string => {
  if (iso === undefined) return ''
  const ms = Date.parse(iso) - now
  if (Number.isNaN(ms)) return ''
  if (ms <= 0) return ' · resetting'
  const mins = Math.ceil(ms / 60_000)
  const d = Math.floor(mins / 1440)
  const h = Math.floor((mins % 1440) / 60)
  const m = mins % 60
  const span = d > 0 ? `${d}d ${h}h` : h > 0 ? `${h}h ${m}m` : `${m}m`
  return ` · resets in ${span}`
}

/** The status text for the figures and the remembered cost at exhaustion. */
export const line = (f: Figures, over: Overage | null, now: number): string | undefined => {
  if (f.rateLimits.length === 0) return undefined
  const exhausted = f.rateLimits.filter(isExhausted)
  if (exhausted.length === 0) {
    return f.rateLimits
      .map(w => `${label(w.kind)} ${w.percentUsed}%${resetIn(w.resetsAt, now)}`)
      .join('  |  ')
  }
  // Usage resumes only when the last exhausted window resets.
  const resetsAt = exhausted
    .map(w => w.resetsAt)
    .filter((s): s is string => s !== undefined)
    .sort()
    .at(-1)
  const spent = f.cost !== undefined && over !== null ? Math.max(0, f.cost.usd - over.baseUsd) : 0
  const names = exhausted.map(w => label(w.kind)).join('+')
  return `⛔ ${names} quota used up · credit used this session ${usd(spent)}${resetIn(resetsAt, now)}`
}

/** Remembers the session cost when quota runs out; forgets it once no window is used up. */
const track = async ($: Engine, f: Figures): Promise<Overage | null> => {
  const isOut = f.rateLimits.some(isExhausted)
  const current = await read($, overage)
  if (isOut === (current !== null)) return current
  const next: Overage | null = isOut ? { baseUsd: f.cost?.usd ?? 0 } : null
  await update($, overage, () => next)
  return next
}

const refresh = async ($: Engine, given?: Figures): Promise<void> => {
  const f: Figures = given ?? (await $.session.usage())
  const over = await track($, f)
  $.ui.status(line(f, over, await $.clock.now()))
}

export const register: Register = on => {
  let tick: { cancel: () => void } | undefined

  on('session.start', async ($, e, next) => {
    const result = await next(e)
    await refresh($)
    tick?.cancel()
    // The countdown moves even when no new measurement arrives.
    tick = $.clock.every(60_000, () => void refresh($))
    return result
  })

  on('session.measure', async ($, e, next) => {
    await refresh($, e)
    return next(e)
  })
}
