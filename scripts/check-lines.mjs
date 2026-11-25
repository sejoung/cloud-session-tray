import { readFile, readdir } from "node:fs/promises";
import { extname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const limit = 400;
const textFiles = new Set([".rs", ".ts", ".tsx", ".js", ".mjs", ".cjs", ".css", ".html", ".json",
  ".toml", ".yaml", ".yml", ".md"]);
const ignored = new Set([".git", ".idea", ".agents", ".codex", ".aws", "node_modules", "dist", "target",
  "src-tauri/gen", "package-lock.json", "src-tauri/Cargo.lock"]);

async function check(directory) {
  const violations = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    const label = relative(root, path).replaceAll("\\", "/");
    if (ignored.has(label) || ignored.has(entry.name)) continue;
    if (entry.isDirectory()) {
      violations.push(...await check(path));
    } else if (entry.isFile() && textFiles.has(extname(entry.name))) {
      const lines = (await readFile(path, "utf8")).split(/\r\n|\n|\r/);
      if (lines.at(-1) === "") lines.pop();
      if (lines.length > limit) violations.push(`${label}: ${lines.length} lines (maximum ${limit})`);
    }
  }
  return violations;
}

const violations = await check(root);
if (violations.length) {
  process.stderr.write(`${violations.join("\n")}\n`);
  process.exitCode = 1;
}
