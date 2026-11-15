import { readFileSync } from 'node:fs';

/** The app's Polish UI strings, so a changed wording fails here, not silently. */
const PL: unknown = JSON.parse(
  readFileSync(new URL('../../ui/src/i18n/pl.json', import.meta.url), 'utf8'),
);

const PLURAL = new Intl.PluralRules('pl-PL');

/** The string at the dotted `key` of pl.json, with `{{name}}` placeholders filled in. */
export function t(key: string, values: Readonly<Record<string, string | number>> = {}): string {
  const found = key
    .split('.')
    .reduce<unknown>(
      (node, part) =>
        typeof node === 'object' && node !== null
          ? (node as Record<string, unknown>)[part]
          : undefined,
      PL,
    );
  if (typeof found !== 'string') throw new Error(`pl.json has no string at ${key}`);
  return found.replace(/\{\{(\w+)(?:, number)?\}\}/g, (_, name: string) => {
    const value = values[name];
    if (value === undefined) throw new Error(`${key} needs a value for ${name}`);
    return String(value);
  });
}

/** The plural form of `key` for `count` (`_one`, `_few`, `_many` or `_other`), filled in. */
export function plural(key: string, count: number): string {
  return t(`${key}_${PLURAL.select(count)}`, { count });
}
