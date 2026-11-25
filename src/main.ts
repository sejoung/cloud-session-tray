import { mountApplication } from "./app";
import { desktop } from "./backend";
import { element } from "./ui/html";
import "./style.css";

mountApplication(element<HTMLElement>(document, "#app"), desktop);
