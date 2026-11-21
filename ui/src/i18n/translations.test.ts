import { describe, expect, it } from 'vitest';

import { LANGUAGES, type Language } from './api';
import de from './de.json';
import en from './en.json';
import pl from './pl.json';

const FILES: Readonly<Record<Language, object>> = { pl, en, de };

const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/u;

/** Every text of a translation file by its dotted key. */
function texts(tree: object, prefix = ''): Map<string, string> {
  const found = new Map<string, string>();
  for (const [key, value] of Object.entries(tree)) {
    const path = prefix + key;
    if (typeof value === 'string') found.set(path, value);
    else if (typeof value === 'object' && value !== null) {
      for (const entry of texts(value as object, `${path}.`)) found.set(...entry);
    } else throw new Error(`${path} is neither a text nor a group`);
  }
  return found;
}

/** The keys without their plural suffix, each with the plural forms it has (empty if none). */
function keysWithForms(file: object): Map<string, Set<string>> {
  const keys = new Map<string, Set<string>>();
  for (const key of texts(file).keys()) {
    const form = PLURAL_SUFFIX.exec(key)?.[1];
    const base = form === undefined ? key : key.slice(0, -form.length - 1);
    const forms = keys.get(base) ?? new Set<string>();
    if (form !== undefined) forms.add(form);
    keys.set(base, forms);
  }
  return keys;
}

/** The `{{placeholder}}` names of every text with `base` as key or plural base. */
function placeholders(file: object, base: string): string[] {
  const names = new Set<string>();
  for (const [key, text] of texts(file)) {
    if (key !== base && key.replace(PLURAL_SUFFIX, '') !== base) continue;
    for (const match of text.matchAll(/\{\{\s*(\w+)/gu)) names.add(match[1] ?? '');
  }
  return [...names].sort();
}

const polishKeys = keysWithForms(pl);

describe.each(LANGUAGES)('the %s translation', (language) => {
  const file = FILES[language];
  const keys = keysWithForms(file);
  const pluralForms = new Intl.PluralRules(language).resolvedOptions().pluralCategories;

  it('has every key of the Polish one and no other', () => {
    expect([...keys.keys()].sort()).toEqual([...polishKeys.keys()].sort());
  });

  it('has exactly the plural forms its language needs', () => {
    for (const [base, polishForms] of polishKeys) {
      if (polishForms.size === 0) continue;
      expect([...(keys.get(base) ?? [])].sort(), base).toEqual([...pluralForms].sort());
    }
  });

  it('uses the placeholders of the Polish texts', () => {
    for (const base of polishKeys.keys()) {
      expect(placeholders(file, base), base).toEqual(placeholders(pl, base));
    }
  });

  it('has no empty text', () => {
    for (const [key, text] of texts(file)) expect(text.trim(), key).not.toBe('');
  });
});

describe('quotation marks', () => {
  it('are the language’s own', () => {
    // Polish „…”, English “…”, German „…“.
    const wrong: Readonly<Record<Language, RegExp>> = { pl: /[“"]/u, en: /[„"]/u, de: /[”"]/u };
    for (const language of LANGUAGES) {
      for (const [key, text] of texts(FILES[language])) {
        expect(text, `${language}: ${key}`).not.toMatch(wrong[language]);
      }
    }
  });
});
