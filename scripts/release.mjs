// Publishes a new Luna version that every installed copy picks up by itself.
//
//   npm run release -- 0.4.0 "What changed, in one or two lines"
//   npm run release -- 0.4.0 --notes-file notes.md
//
// 1. Writes the version into every file that carries it.
// 2. Builds the installers and signs them with the updater key.
// 3. Writes latest.json, the file installed copies read to find the new version.
// 4. Commits, tags, pushes and creates the GitHub release with the installers and latest.json.
//
// Needs: the GitHub CLI signed in, and the updater private key in TAURI_SIGNING_PRIVATE_KEY
// or at %USERPROFILE%\.tauri\luna-updater.key. The key never goes in the repository.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = "realKarasu/luna";
const root = fileURLToPath(new URL("..", import.meta.url));
const at = (...p) => join(root, ...p);

function fail(msg) {
  console.error(`\nrelease: ${msg}`);
  process.exit(1);
}

// Arguments go to git and gh as they are (no shell), so notes with spaces stay whole.
function run(cmd, args, opts = {}) {
  console.log(`> ${cmd} ${args.join(" ")}`);
  return execFileSync(cmd, args, { cwd: root, stdio: "inherit", ...opts });
}

function out(cmd, args) {
  return execFileSync(cmd, args, { cwd: root, encoding: "utf8" }).trim();
}

// ---------------- arguments ----------------

const args = process.argv.slice(2);
const version = args[0];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) fail('usage: npm run release -- <x.y.z> "notes"');
let notes = "";
const nf = args.indexOf("--notes-file");
if (nf >= 0) notes = readFileSync(args[nf + 1], "utf8").trim();
else notes = args.slice(1).join(" ").trim();
if (!notes) fail("add a short note about what changed, it is shown in the app and on the release page");

const current = JSON.parse(readFileSync(at("package.json"), "utf8")).version;
const newer = (a, b) => {
  const x = a.split(".").map(Number);
  const y = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return x[i] > y[i];
  return false;
};
if (!newer(version, current)) fail(`${version} must be newer than ${current}, or installed copies will not update`);

// ---------------- checks ----------------

const keyPath = join(homedir(), ".tauri", "luna-updater.key");
let key = process.env.TAURI_SIGNING_PRIVATE_KEY;
if (!key) {
  if (!existsSync(keyPath)) fail(`no signing key: set TAURI_SIGNING_PRIVATE_KEY or put it at ${keyPath}`);
  key = readFileSync(keyPath, "utf8").trim();
}
try {
  out("gh", ["auth", "status"]);
} catch {
  fail("the GitHub CLI is not signed in (gh auth login)");
}
const dirty = out("git", ["status", "--porcelain"]);
if (dirty) console.log(`Uncommitted changes go into the release commit:\n${dirty}\n`);

// ---------------- 1. version everywhere ----------------

function replaceIn(file, pattern, value) {
  const p = at(file);
  const text = readFileSync(p, "utf8");
  if (!pattern.test(text)) fail(`no version found in ${file}`);
  writeFileSync(p, text.replace(pattern, value));
}

replaceIn("package.json", /("version":\s*")[^"]+(")/, `$1${version}$2`);
const lock = JSON.parse(readFileSync(at("package-lock.json"), "utf8"));
lock.version = version;
if (lock.packages?.[""]) lock.packages[""].version = version;
writeFileSync(at("package-lock.json"), JSON.stringify(lock, null, 2) + "\n");
replaceIn("src-tauri/tauri.conf.json", /("version":\s*")[^"]+(")/, `$1${version}$2`);
replaceIn("src-tauri/Cargo.toml", /^(version\s*=\s*")[^"]+(")/m, `$1${version}$2`);
replaceIn("src/lib/backend.ts", /(version: ")[^"]+(")/, `$1${version}$2`);
replaceIn("src/store.tsx", /(version: ")[^"]+(")/, `$1${version}$2`);

// ---------------- 2. build and sign ----------------

const target = process.env.CARGO_TARGET_DIR || (existsSync("D:\\") ? "D:\\cargo-target\\luna" : at("src-tauri", "target"));
const env = {
  ...process.env,
  CARGO_TARGET_DIR: target,
  TAURI_SIGNING_PRIVATE_KEY: key,
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ?? "",
};
try {
  execFileSync("taskkill", ["/IM", "luna.exe", "/F"], { stdio: "ignore", shell: true });
} catch {
  // Not running.
}
// npm is a .cmd file on Windows, so this one call goes through the shell.
run("npm", ["run", "tauri", "build"], { env, shell: true });

const bundle = join(target, "release", "bundle");
const setup = join(bundle, "nsis", `Luna_${version}_x64-setup.exe`);
const msi = join(bundle, "msi", `Luna_${version}_x64_en-US.msi`);
for (const f of [setup, `${setup}.sig`, msi]) if (!existsSync(f)) fail(`missing after the build: ${f}`);

// ---------------- 3. latest.json ----------------

const tag = `v${version}`;
const latest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature: readFileSync(`${setup}.sig`, "utf8").trim(),
      url: `https://github.com/${REPO}/releases/download/${tag}/Luna_${version}_x64-setup.exe`,
    },
  },
};
const latestPath = join(bundle, "latest.json");
writeFileSync(latestPath, JSON.stringify(latest, null, 2) + "\n");

const sha = (f) => createHash("sha256").update(readFileSync(f)).digest("hex").toUpperCase();
const body = `${notes}

Installed copies of Luna 0.3.0 and newer update by themselves. Older copies: install this version once.

SHA-256 fingerprints (compare with \`Get-FileHash\`):
- \`Luna_${version}_x64-setup.exe\`: \`${sha(setup)}\`
- \`Luna_${version}_x64_en-US.msi\`: \`${sha(msi)}\`
`;
const bodyPath = join(bundle, "release-notes.md");
writeFileSync(bodyPath, body);

// ---------------- 4. commit, tag, push, release ----------------

run("git", ["add", "-A"]);
run("git", ["commit", "-m", `Luna ${version}: ${notes.split("\n")[0]}`]);
run("git", ["tag", "-a", tag, "-m", `Luna ${version}`]);
run("git", ["push"]);
run("git", ["push", "origin", tag]);
run("gh", ["release", "create", tag, setup, msi, latestPath, "--repo", REPO, "--title", `Luna ${version}`, "--notes-file", bodyPath]);

console.log(`\nLuna ${version} is out. Installed copies pick it up within a few hours, or at their next start.`);
