import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { createInterface } from "node:readline/promises";

const root = new URL("../", import.meta.url);
const versionFiles = ["package.json", "package-lock.json", "src-tauri/Cargo.toml", "src-tauri/Cargo.lock"];
const bumps = { major: 0, minor: 1, patch: 2 };
const usage = `npm run release:patch                         Backward-compatible fixes
npm run release:minor                         Backward-compatible features
npm run release:major                         Breaking changes
npm run release:current                       Release the current version
npm run release:resume -- vMAJOR.MINOR.PATCH  Push a prepared release`;

function run(program, args, capture = false) {
  const result = spawnSync(program, args, {
    cwd: root,
    encoding: "utf8",
    stdio: capture ? ["ignore", "pipe", "inherit"] : "inherit",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${program} ${args.join(" ")} failed (${result.signal ?? result.status})`);
  return result.stdout?.trim();
}

function git(...args) {
  return run("git", args, true);
}

function npm(script, ...args) {
  const npmCli = process.env.npm_execpath;
  if (!npmCli) throw new Error(`Start the release through npm.\n\n${usage}`);
  run(process.execPath, [npmCli, "run", script, "--", ...args]);
}

function requireCleanMain() {
  if (git("branch", "--show-current") !== "main" || git("status", "--porcelain")) {
    throw new Error("Run the release command from main with your changes committed.");
  }
}

function releaseVersion(current, mode) {
  if (mode === "current") return current;
  const bump = bumps[mode];
  return current.split(".").map((component, index) => {
    if (index === bump) return String(BigInt(component) + 1n);
    return index < bump ? component : "0";
  }).join(".");
}

function prepareTag(mode) {
  npm("version:check");
  const { version } = JSON.parse(readFileSync(new URL("package.json", root), "utf8"));
  const next = releaseVersion(version, mode);
  const tag = `v${next}`;
  if (git("tag", "--list", tag)) {
    throw new Error(`${tag} already exists. To retry a prepared release, use npm run release:resume -- ${tag}.`);
  }

  if (next !== version) {
    try {
      npm("version:set", next);
      npm("version:check", tag);
    } catch (error) {
      npm("version:set", version);
      throw error;
    }
    run("git", ["commit", "--only", "-m", `chore: release ${tag}`, "--", ...versionFiles]);
  }
  requireCleanMain();
  run("git", ["tag", "-a", tag, "-m", `Cloud Session Tray ${tag}`]);
  return tag;
}

function tagCommit(tag) {
  return git("rev-parse", "--verify", `refs/tags/${tag}^{commit}`);
}

async function confirmPush(tag) {
  const prompt = createInterface({ input: process.stdin, output: process.stdout });
  try {
    return /^y(?:es)?$/i.test((await prompt.question(`Push main and ${tag} to origin and start the release? [y/N] `)).trim());
  } finally {
    prompt.close();
  }
}

async function main([mode, preparedTag, ...extra]) {
  if (mode === undefined || (mode === "--help" && preparedTag === undefined)) {
    process.stdout.write(`${usage}\n`);
    return;
  }
  const resume = mode === "resume";
  if (extra.length || (resume ? !preparedTag : preparedTag || (mode !== "current" && !Object.hasOwn(bumps, mode)))) {
    throw new Error(`Choose a release command.\n\n${usage}`);
  }
  if (!process.stdin.isTTY) throw new Error("Run this command in a terminal to confirm the push.");
  requireCleanMain();
  run("git", ["fetch", "--tags", "origin", "main"]);
  run("git", ["merge-base", "--is-ancestor", "origin/main", "HEAD"]);

  const commitBeforeRelease = git("rev-parse", "HEAD");
  if (resume) {
    npm("version:check", preparedTag);
    if (tagCommit(preparedTag) !== commitBeforeRelease) throw new Error(`${preparedTag} must point to HEAD on main.`);
  }
  const tag = resume ? preparedTag : prepareTag(mode);
  const commit = git("rev-parse", "HEAD");
  process.stdout.write(`${tag} → ${commit}\n`);
  if (!await confirmPush(tag)) {
    process.stdout.write(`Kept ${tag} locally. Resume with npm run release:resume -- ${tag}.\n`);
    return;
  }
  requireCleanMain();
  if (git("rev-parse", "HEAD") !== commit || tagCommit(tag) !== commit) {
    throw new Error("Main or the release tag changed while waiting for confirmation.");
  }
  run("git", ["push", "--atomic", "origin", `${commit}:refs/heads/main`, `refs/tags/${tag}`]);
}

try {
  await main(process.argv.slice(2));
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
}
