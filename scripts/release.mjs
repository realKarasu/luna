// Publishes a new Luna version that every installed copy picks up by itself.
//
//   npm run release -- 0.4.0 "What changed, in one or two lines"
//   npm run release -- 0.4.0 --notes-file notes.md
//   npm run release -- 0.4.0 --resume "notes"     (tag already pushed: wait for CI and publish)
//
// 1. Writes the version into every file that carries it.
// 2. Builds the Windows installers here and signs them with the updater key.
// 3. Commits, tags and pushes. The tag starts GitHub Actions, which builds macOS and Linux.
// 4. Waits for that run, downloads its packages and signs them here, so the key never leaves
//    this machine.
// 5. Writes latest.json, the file installed copies read to find the new version, and creates
//    the GitHub release with every package.
//
// Needs: the GitHub CLI signed in, and the updater private key in TAURI_SIGNING_PRIVATE_KEY
// or at %USERPROFILE%\.tauri\luna-updater.key. The key never goes in the repository or in GitHub.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
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

const resume = process.argv.includes("--resume");
const args = process.argv.slice(2).filter((a) => a !== "--resume");
const version = args[0];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) fail('usage: npm run release -- <x.y.z> "notes"');
let notes = "";
const nf = args.indexOf("--notes-file");
if (nf >= 0) notes = readFileSync(args[nf + 1], "utf8").trim();
else notes = args.slice(1).join(" ").trim();
if (!notes) fail("add a short note about what changed, it is shown in the app and on the release page");

// Compared with the newest published tag: the files may already carry the new version.
const current = out("git", ["tag", "--list", "v*", "--sort=-v:refname"]).split("\n")[0]?.replace(/^v/, "") || "0.0.0";
const newer = (a, b) => {
  const x = a.split(".").map(Number);
  const y = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return x[i] > y[i];
  return false;
};
if (!resume && !newer(version, current)) fail(`${version} must be newer than ${current}, or installed copies will not update`);

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
const tag = `v${version}`;
let tagExists = true;
try {
  out("git", ["rev-parse", "--verify", "--quiet", `refs/tags/${tag}`]);
} catch {
  tagExists = false;
}
if (resume && !tagExists) fail(`--resume needs the tag ${tag}, which does not exist yet`);
if (!resume && tagExists) fail(`the tag ${tag} exists already; add --resume to finish that release`);
const dirty = out("git", ["status", "--porcelain"]);
if (dirty && !resume) console.log(`Uncommitted changes go into the release commit:\n${dirty}\n`);

// ---------------- 1. version everywhere ----------------

function replaceIn(file, pattern, value) {
  const p = at(file);
  const text = readFileSync(p, "utf8");
  if (!pattern.test(text)) fail(`no version found in ${file}`);
  writeFileSync(p, text.replace(pattern, value));
}

if (!resume) {
  replaceIn("package.json", /("version":\s*")[^"]+(")/, `$1${version}$2`);
  const lock = JSON.parse(readFileSync(at("package-lock.json"), "utf8"));
  lock.version = version;
  if (lock.packages?.[""]) lock.packages[""].version = version;
  writeFileSync(at("package-lock.json"), JSON.stringify(lock, null, 2) + "\n");
  replaceIn("src-tauri/tauri.conf.json", /("version":\s*")[^"]+(")/, `$1${version}$2`);
  replaceIn("src-tauri/Cargo.toml", /^(version\s*=\s*")[^"]+(")/m, `$1${version}$2`);
  replaceIn("src/lib/backend.ts", /(version: ")[^"]+(")/, `$1${version}$2`);
  replaceIn("src/store.tsx", /(version: ")[^"]+(")/, `$1${version}$2`);
}

// ---------------- 2. Windows build, signed here ----------------

const target = process.env.CARGO_TARGET_DIR || (existsSync("D:\\") ? "D:\\cargo-target\\luna" : at("src-tauri", "target"));
const env = {
  ...process.env,
  CARGO_TARGET_DIR: target,
  TAURI_SIGNING_PRIVATE_KEY: key,
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ?? "",
};
const bundle = join(target, "release", "bundle");
const setup = join(bundle, "nsis", `Luna_${version}_x64-setup.exe`);
const msi = join(bundle, "msi", `Luna_${version}_x64_en-US.msi`);

if (!resume) {
  try {
    execFileSync("taskkill", ["/IM", "luna.exe", "/F"], { stdio: "ignore", shell: true });
  } catch {
    // Not running.
  }
  // npm is a .cmd file on Windows, so this one call goes through the shell.
  run("npm", ["run", "tauri", "build"], { env, shell: true });
}
for (const f of [setup, `${setup}.sig`, msi]) if (!existsSync(f)) fail(`missing after the build: ${f}`);

// ---------------- 3. commit, tag, push (starts the macOS and Linux build) ----------------

if (!resume) {
  run("git", ["add", "-A"]);
  run("git", ["commit", "-m", `Luna ${version}: ${notes.split("\n")[0]}`]);
  run("git", ["tag", "-a", tag, "-m", `Luna ${version}`]);
  run("git", ["push"]);
  run("git", ["push", "origin", tag]);
}

// ---------------- 4. macOS and Linux packages from GitHub Actions ----------------

const commit = out("git", ["rev-list", "-n", "1", tag]);
const sleep = (ms) => Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
let runId = "";
for (let i = 0; i < 60 && !runId; i++) {
  const runs = JSON.parse(
    out("gh", ["run", "list", "--repo", REPO, "--workflow", "build.yml", "--commit", commit, "--limit", "5", "--json", "databaseId,event"]),
  );
  runId = String(runs.find((r) => r.event === "push")?.databaseId ?? runs[0]?.databaseId ?? "");
  if (!runId) sleep(5000);
}
if (!runId) fail(`no GitHub Actions run found for ${tag}; start "Build macOS and Linux" by hand, then rerun with --resume`);
console.log(`Waiting for the macOS and Linux build (run ${runId})...`);
try {
  run("gh", ["run", "watch", runId, "--repo", REPO, "--exit-status", "--interval", "30"]);
} catch {
  fail(`the macOS and Linux build failed: gh run view ${runId} --repo ${REPO} --log-failed. Fix it, then rerun with --resume`);
}

const ci = join(bundle, "ci");
rmSync(ci, { recursive: true, force: true });
mkdirSync(ci, { recursive: true });
run("gh", ["run", "download", runId, "--repo", REPO, "--dir", ci]);

const files = (dir) =>
  readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? files(p) : [p];
  });
const found = files(ci);
const pick = (suffix) => found.find((f) => f.endsWith(suffix) && f.includes(version)) ?? fail(`the CI build has no *${suffix}`);
const macTar = pick(".app.tar.gz");
const dmg = pick(".dmg");
const appImage = pick(".AppImage");
const deb = pick(".deb");

// Same key and signature format as the Windows build.
for (const f of [macTar, appImage, deb]) run("npx", ["tauri", "signer", "sign", `"${f}"`], { env, shell: true });
const sig = (f) => readFileSync(`${f}.sig`, "utf8").trim();

// ---------------- 5. latest.json, notes and the release ----------------

const name = (f) => f.split(/[\\/]/).pop();
const url = (f) => `https://github.com/${REPO}/releases/download/${tag}/${name(f)}`;
const mac = { signature: sig(macTar), url: url(macTar) };
const latest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": { signature: sig(setup), url: url(setup) },
    "darwin-aarch64": mac,
    "darwin-x86_64": mac,
    "linux-x86_64": { signature: sig(appImage), url: url(appImage) },
    "linux-x86_64-deb": { signature: sig(deb), url: url(deb) },
  },
};
const latestPath = join(bundle, "latest.json");
writeFileSync(latestPath, JSON.stringify(latest, null, 2) + "\n");

const hash = (f) => createHash("sha256").update(readFileSync(f)).digest("hex").toUpperCase();
const assets = [setup, msi, dmg, macTar, appImage, deb];
const body = `${notes}

| System | Download |
| --- | --- |
| Windows 10 or 11 | \`${name(setup)}\` (or the \`.msi\`) |
| macOS 10.15 or newer | \`${name(dmg)}\` |
| Linux | \`${name(appImage)}\`, or \`${name(deb)}\` on Debian and Ubuntu |

Installed copies of Luna 0.3.0 and newer update by themselves. First time on a Mac or on Linux? See the [README](https://github.com/${REPO}#first-launch-system-by-system).

SHA-256 fingerprints (\`Get-FileHash\` on Windows, \`shasum -a 256\` on macOS, \`sha256sum\` on Linux):
${assets.map((f) => `- \`${name(f)}\`: \`${hash(f)}\``).join("\n")}
`;
const bodyPath = join(bundle, "release-notes.md");
writeFileSync(bodyPath, body);

run("gh", ["release", "create", tag, ...assets, latestPath, "--repo", REPO, "--title", `Luna ${version}`, "--notes-file", bodyPath]);

console.log(`\nLuna ${version} is out. Installed copies pick it up within a few hours, or at their next start.`);
