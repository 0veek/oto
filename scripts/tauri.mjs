#!/usr/bin/env node
/**
 * Wrapper around `@tauri-apps/cli` that sets NO_STRIP=1 and fails early with
 * install instructions when Linux packaging deps are missing.
 *
 * Tauri panics with "Can't detect any appindicator library" during bundling
 * when tray-icon is enabled and neither ayatana-appindicator3 nor
 * appindicator3 is visible to pkg-config. That abort happens after a full
 * release compile, so catch it before cargo/tauri spend minutes rebuilding.
 */
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

const args = process.argv.slice(2);
const isBuild = args[0] === "build";
const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function resolveTauriBin() {
  const binDir = join(root, "node_modules", ".bin");
  const candidates =
    process.platform === "win32"
      ? [join(binDir, "tauri.cmd"), join(binDir, "tauri.ps1"), join(binDir, "tauri")]
      : [join(binDir, "tauri")];
  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }
  // Fall back to PATH (e.g. global install or npm-provided PATH).
  return "tauri";
}

/**
 * Mirror tauri-cli's detector: pkg-config --libs-only-L with
 * PKG_CONFIG_ALLOW_SYSTEM_LIBS=1. On Arch/CachyOS, system libs live in
 * /usr/lib so --libs-only-L is empty unless that env var is set.
 */
function pkgConfigLibraryPath(module) {
  const result = spawnSync(
    "pkg-config",
    ["--libs-only-L", module],
    {
      encoding: "utf8",
      env: { ...process.env, PKG_CONFIG_ALLOW_SYSTEM_LIBS: "1" },
    },
  );
  if (result.status !== 0 || !result.stdout?.trim()) {
    return null;
  }
  // stdout is like "-L/usr/lib\n"
  return result.stdout.trim().replace(/^-L/, "");
}

function checkLinuxAppindicator() {
  if (process.platform !== "linux" || !isBuild) {
    return;
  }

  const hasAyatana = pkgConfigLibraryPath("ayatana-appindicator3-0.1");
  const hasLegacy = pkgConfigLibraryPath("appindicator3-0.1");
  if (hasAyatana || hasLegacy) {
    return;
  }

  console.error(`
error: Can't detect any appindicator library (required for Tauri tray packaging)

Oto enables tauri's tray-icon feature. After the binary compiles, the bundler
looks up the system tray library with pkg-config and panics if it is missing,
so deb / AppImage / rpm packages are never written.

Install the development package for your distribution, then re-run the build:

  # Arch / CachyOS
  sudo pacman -S --needed libayatana-appindicator

  # Debian / Ubuntu
  sudo apt install libayatana-appindicator3-dev

  # Fedora
  sudo dnf install libayatana-appindicator-gtk3-devel

Verify:
  pkg-config --exists ayatana-appindicator3-0.1 && echo ok
`);
  process.exit(1);
}

checkLinuxAppindicator();

process.env.NO_STRIP = "1";

const tauriBin = resolveTauriBin();
const result = spawnSync(tauriBin, args, {
  stdio: "inherit",
  env: process.env,
  shell: process.platform === "win32",
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 1);
