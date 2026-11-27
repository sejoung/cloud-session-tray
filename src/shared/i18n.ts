import en from "../../locales/en.json";
import ko from "../../locales/ko.json";
import type { Language } from "./domain";

export type MessageKey = keyof typeof en;
export type Values = Readonly<Record<string, string | number>>;
const messages: Record<Language, Record<MessageKey, string>> = { en, ko };
const legacy = new Map(Object.entries(ko).map(([key, value]) => [value, key as MessageKey]));

export function systemLanguage(): Language {
  return /^ko(?:-|$)/i.test(navigator.language) ? "ko" : "en";
}

let language = systemLanguage();

export function useLanguage(preferred?: Language | null): boolean {
  const next = preferred ?? systemLanguage();
  const changed = language !== next;
  language = next;
  if (document.documentElement.lang !== language) document.documentElement.lang = language;
  return changed;
}

export function currentLanguage(): Language {
  return language;
}

export function t(key: MessageKey, values: Values = {}): string {
  return messages[language][key].replace(/\{(\w+)\}/g, (token, name: string) =>
    Object.hasOwn(values, name) ? String(values[name]) : token,
  );
}

export function localizeMessage(message: string, fallback: MessageKey = "errors.unexpected"): string {
  const key = Object.hasOwn(en, message) ? message as MessageKey : legacy.get(message);
  return t(key ?? fallback);
}
