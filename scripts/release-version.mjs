import { readFile, writeFile } from "node:fs/promises";

const stableVersion = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/u;
const cargoManifestVersion = /(^\[package\]\r?\n(?:^(?!\[)[^\r\n]*\r?\n)*?^version\s*=\s*")([^"\r\n]+)(")/mu;
const cargoLockVersion = /(^\[\[package\]\]\r?\nname = "cloud-session-tray"\r?\nversion = ")([^"\r\n]+)(")/mu;

function file(filename) {
  return new URL(`../${filename}`, import.meta.url);
}

function read(filename) {
  return readFile(file(filename), "utf8");
}

function readRustVersion(contents, pattern, filename) {
  const match = contents.match(pattern);
  if (!match) throw new Error(`Missing application version in ${filename}`);
  return match[2];
}

function checkVersions(expected, versions, tag) {
  if (typeof expected !== "string" || !stableVersion.test(expected)) {
    throw new Error(`Expected a stable version in package.json, got ${expected}`);
  }
  for (const [filename, version] of Object.entries(versions)) {
    if (version !== expected) throw new Error(`${filename} has version ${version}, expected ${expected}`);
  }
  if (tag !== undefined && tag !== `v${expected}`) {
    throw new Error(`Release tag ${tag} must match v${expected}`);
  }
  process.stdout.write(`Version ${expected}\n`);
}

async function main([command, argument, ...extra]) {
  if (extra.length || (command !== "check" && command !== "set")) {
    throw new Error("Use version:check [vMAJOR.MINOR.PATCH] or version:set -- MAJOR.MINOR.PATCH");
  }
  if (command === "set" && (argument === undefined || !stableVersion.test(argument))) {
    throw new Error("Expected a stable version such as 0.1.1");
  }

  const [manifestText, npmLockText, cargoManifest, cargoLock, tauriConfigText] = await Promise.all([
    read("package.json"),
    read("package-lock.json"),
    read("src-tauri/Cargo.toml"),
    read("src-tauri/Cargo.lock"),
    read("src-tauri/tauri.conf.json"),
  ]);
  const manifest = JSON.parse(manifestText);
  const npmLock = JSON.parse(npmLockText);
  const tauriConfig = JSON.parse(tauriConfigText);
  if (tauriConfig.version !== "../package.json") {
    throw new Error("Tauri version must point to ../package.json");
  }
  const npmRoot = npmLock.packages?.[""];
  if (!npmRoot) throw new Error("Missing root package in package-lock.json");

  const versions = {
    "package.json": manifest.version,
    "package-lock.json": npmLock.version,
    "package-lock.json root package": npmRoot.version,
    "src-tauri/Cargo.toml": readRustVersion(cargoManifest, cargoManifestVersion, "Cargo.toml"),
    "src-tauri/Cargo.lock": readRustVersion(cargoLock, cargoLockVersion, "Cargo.lock"),
  };
  if (command === "check") {
    checkVersions(manifest.version, versions, argument);
    return;
  }

  manifest.version = argument;
  npmLock.version = argument;
  npmRoot.version = argument;
  const replaceVersion = (_match, prefix, _version, suffix) => `${prefix}${argument}${suffix}`;
  const updates = [
    ["package.json", manifestText, `${JSON.stringify(manifest, null, 2)}\n`],
    ["package-lock.json", npmLockText, `${JSON.stringify(npmLock, null, 2)}\n`],
    ["src-tauri/Cargo.toml", cargoManifest, cargoManifest.replace(cargoManifestVersion, replaceVersion)],
    ["src-tauri/Cargo.lock", cargoLock, cargoLock.replace(cargoLockVersion, replaceVersion)],
  ];
  for (const [filename, previous, updated] of updates) {
    if (previous !== updated) await writeFile(file(filename), updated);
  }
  process.stdout.write(`Version set to ${argument}\n`);
}

try {
  await main(process.argv.slice(2));
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
}
