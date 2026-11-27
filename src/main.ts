import { mountApplication } from "./app/application";
import { desktop } from "./desktop/tauri";
import { element } from "./shared/html";
import { systemLanguage } from "./shared/i18n";
import "./app/styles.css";

mountApplication(element<HTMLElement>(document, "#app"), desktop);
void desktop.setSystemLanguage(systemLanguage()).catch(() => console.warn("Could not initialize the native display language."));
