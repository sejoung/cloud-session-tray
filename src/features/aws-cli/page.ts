import { t } from "../../shared/i18n";
import type { Dashboard } from "../../shared/domain";
import { header } from "../../shared/html";
import { localProfilesCard, type LocalProfilesOptions } from "../profiles/local-profiles";
import { installerCard, type InstallerOptions } from "./installer";
import { installationsCard } from "./installations";
import { terminalPathCard } from "./terminal-path";

export type { InstallerState } from "./installer";

type CliViewOptions = InstallerOptions & LocalProfilesOptions;

export function cliPage(state: Dashboard, options: CliViewOptions): string {
  return header("AWS CLI", t("cli.description"))
    + installerCard(state, options) + installationsCard(state, options.cliBusy)
    + terminalPathCard(state, options.cliBusy) + localProfilesCard(state, options);
}
