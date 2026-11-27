import type { Login, Profile, ProfilePreset, Session } from "../../shared/domain";

export type EditorSelection = {
  session?: Session;
  defaultKind?: Login["kind"];
  preferredSource?: string;
  preset?: ProfilePreset;
};

export function suggestTarget(source: string, items: Profile[], managed: ReadonlySet<string>): string {
  if (!source) return "";
  const linked = items.find((profile) => profile.name === source)?.linkedTarget;
  if (linked && !managed.has(linked)) return linked;

  const occupied = new Set(managed);
  occupied.add(source);
  for (const profile of items) {
    if (!profile.targetAllowed || profile.kind !== "unconfigured") occupied.add(profile.name);
  }
  const stem = Array.from(source).slice(0, 56).join("");
  const base = `${stem}-session`;
  let candidate = base;
  for (let suffix = 2; occupied.has(candidate); suffix++) {
    candidate = `${base}-${suffix}`;
  }
  return candidate;
}

function suggestedProfile(items: Profile[], name?: string | null): Profile | undefined {
  return items.find((profile) => profile.name === name)
    ?? items.find((profile) => profile.preferred)
    ?? (items.length === 1 ? items[0] : undefined);
}

function iamFields(
  selection: EditorSelection, items: Profile[], managed: ReadonlySet<string>, sources: Profile[],
) {
  const login = selection.session?.login;
  const preset = selection.preset?.kind === "iamMfa" ? selection.preset : undefined;
  if (login?.kind === "iamMfa") {
    return preset ? { ...login, duration: preset.duration ?? login.duration, region: preset.region ?? login.region } : login;
  }
  const hint = selection.preferredSource ?? preset?.sourceHint;
  const suggested = suggestedProfile(sources, hint);
  return initialIamFields(items, managed, suggested, preset);
}

function initialIamFields(
  items: Profile[], managed: ReadonlySet<string>, suggested: Profile | undefined,
  preset: Extract<ProfilePreset, { kind: "iamMfa" }> | undefined,
) {
  const source = suggested?.name ?? "";
  return {
    source,
    target: preset?.target ?? suggestTarget(source, items, managed),
    device: suggested?.mfaDevice ?? "",
    duration: preset?.duration ?? null,
    region: preset?.region ?? null,
  };
}

export function editorDefaults(items: Profile[], managed: ReadonlySet<string>, selection: EditorSelection) {
  const login = selection.session?.login;
  const existingSso = login?.kind === "sso" ? login.profile : undefined;
  const choice = login ?? selection.preset;
  const ssoName = choice?.kind === "sso" ? choice.profile : undefined;
  const sources = items.filter((profile) => profile.kind === "iamSource");
  const sso = items.filter((profile) => profile.kind === "sso"
    && (!managed.has(profile.name) || existingSso === profile.name));
  const suggestedSso = suggestedProfile(sso, ssoName);
  const preferSso = suggestedSso?.preferred || (!sources.length && sso.length > 0);
  const kind = choice?.kind ?? selection.defaultKind ?? (preferSso ? "sso" : "iamMfa");
  return {
    ...iamFields(selection, items, managed, sources),
    kind, sources, sso,
    ssoProfile: existingSso ?? suggestedSso?.name,
  };
}
