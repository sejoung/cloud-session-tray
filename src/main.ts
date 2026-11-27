import { mountApplication } from "./app";
import { desktop } from "./backend";
import { element } from "./ui/html";
import { systemLanguage } from "./ui/i18n";
import "./style.css";

mountApplication(element<HTMLElement>(document, "#app"), desktop);
void desktop.setSystemLanguage(systemLanguage()).catch(() => console.warn("Could not initialize the native display language."));
