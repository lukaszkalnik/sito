/** Session cost (USD) at the moment a quota window was first used up. */
export type Overage = { baseUsd: number }

declare module 'claude-code' {
  interface PluginState {
    'quota-line': { overage: Overage | null }
  }
}
