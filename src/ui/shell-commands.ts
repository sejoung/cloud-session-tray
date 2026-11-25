export type Shell = "posix" | "powershell";

export function shellQuote(value: string, shell: Shell): string {
  return shell === "powershell"
    ? `'${value.replaceAll("'", "''")}'`
    : `'${value.replaceAll("'", "'\"'\"'")}'`;
}
