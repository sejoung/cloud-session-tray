import { expect, it } from "vitest";
import en from "../locales/en.json";
import ko from "../locales/ko.json";
import { localizeMessage, useLanguage } from "../src/ui/i18n";

it("renders stored messages in the current language", () => {
  useLanguage("en");
  expect(localizeMessage(ko["errors.awsAuth"])).toBe(en["errors.awsAuth"]);
  expect(localizeMessage("errors.mfaRejected")).toBe(en["errors.mfaRejected"]);
  useLanguage("ko");
  expect(localizeMessage("errors.mfaRejected")).toBe(ko["errors.mfaRejected"]);
  useLanguage();
});
