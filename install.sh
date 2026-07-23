#!/usr/bin/env bash
#
# Build and install Oto from source on the major Linux distribution families.
#
# Default bundle mapping:
#   Debian / Ubuntu       -> Debian package
#   Fedora / RHEL family  -> Flatpak
#   Arch family           -> AppImage
#
# Run this script as your normal desktop user. It asks for sudo only when it
# installs distribution packages or integrates a system-wide bundle.

set -Eeuo pipefail
IFS=$'\n\t'

readonly APP_ID="dev.oto.app"
readonly APP_NAME="Oto"
readonly DEFAULT_REPOSITORY="https://github.com/0veek/oto.git"
readonly NVM_VERSION="v0.40.3"
readonly NODE_LTS="22"
readonly FLATHUB_REPOSITORY="https://dl.flathub.org/repo/flathub.flatpakrepo"

REPOSITORY="${OTO_REPOSITORY:-$DEFAULT_REPOSITORY}"
SOURCE_DIR="${OTO_SOURCE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/oto-installer/source}"
LOCAL_SOURCE=""
GIT_REF="${OTO_REF:-}"
BUNDLE="${OTO_BUNDLE:-auto}"
SKIP_SYSTEM_DEPS=false
INSTALL_BUNDLE=true

DISTRO_FAMILY=""
DISTRO_NAME=""
SESSION_TYPE=""
COMPOSITOR=""
PACKAGE_MANAGER=""
DNF_COMMAND=""
SELECTED_PACKAGE=""
ARTIFACT=""

declare -a PRIVILEGE=()
declare -a REQUIRED_PACKAGE_GROUPS=()
declare -a OPTIONAL_PACKAGE_GROUPS=()
declare -a RESOLVED_REQUIRED_PACKAGES=()
declare -a RESOLVED_OPTIONAL_PACKAGES=()

log() {
  printf '\n==> %s\n' "$*"
}

info() {
  printf '    %s\n' "$*"
}

warn() {
  printf 'warning: %s\n' "$*" >&2
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

on_error() {
  local exit_code=$?
  printf 'error: installer stopped at line %s (exit %s)\n' "${BASH_LINENO[0]}" "$exit_code" >&2
  exit "$exit_code"
}
trap on_error ERR

usage() {
  cat <<'EOF'
Usage: bash install.sh [options]

Clone, build, bundle, and install Oto for the current Linux distribution.

Options:
  --bundle FORMAT       auto, deb, flatpak, or appimage (default: auto)
  --ref REF             Git branch, tag, or commit to build
  --repo URL            Git repository to clone
  --source-dir PATH     Clone/update the repository at PATH
  --local [PATH]        Build an existing checkout instead of cloning
  --skip-system-deps    Do not invoke the distribution package manager
  --build-only          Build the bundle without installing it
  -h, --help            Show this help

Environment equivalents:
  OTO_BUNDLE, OTO_REF, OTO_REPOSITORY, OTO_SOURCE_DIR

Examples:
  bash install.sh
  bash install.sh --ref v0.1.0
  bash install.sh --bundle appimage --build-only
  bash install.sh --local .
EOF
}

while (($# > 0)); do
  case "$1" in
    --bundle)
      (($# >= 2)) || die "--bundle requires a value"
      BUNDLE="${2,,}"
      shift 2
      ;;
    --ref)
      (($# >= 2)) || die "--ref requires a value"
      GIT_REF="$2"
      shift 2
      ;;
    --repo)
      (($# >= 2)) || die "--repo requires a value"
      REPOSITORY="$2"
      shift 2
      ;;
    --source-dir)
      (($# >= 2)) || die "--source-dir requires a value"
      SOURCE_DIR="$2"
      shift 2
      ;;
    --local)
      LOCAL_SOURCE="."
      if (($# >= 2)) && [[ "$2" != -* ]]; then
        LOCAL_SOURCE="$2"
        shift
      fi
      shift
      ;;
    --skip-system-deps)
      SKIP_SYSTEM_DEPS=true
      shift
      ;;
    --build-only)
      INSTALL_BUNDLE=false
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown option: $1 (run with --help)"
      ;;
  esac
done

case "$BUNDLE" in
  auto|deb|flatpak|appimage) ;;
  *) die "unsupported bundle '$BUNDLE'; use auto, deb, flatpak, or appimage" ;;
esac

if [[ -n "$LOCAL_SOURCE" && -n "$GIT_REF" ]]; then
  die "--local and --ref cannot be used together; check out the desired ref in the local repository first"
fi

if [[ -n "$GIT_REF" ]] && [[ ! "$GIT_REF" =~ ^[A-Za-z0-9._/-]+$ || "$GIT_REF" == *..* ]]; then
  die "invalid Git ref '$GIT_REF'"
fi

[[ "$(uname -s)" == "Linux" ]] || die "Oto's installer currently supports Linux only"
((EUID != 0)) || die "run this installer as your normal desktop user, not with sudo"

detect_distribution() {
  [[ -r /etc/os-release ]] || die "cannot identify this Linux distribution (/etc/os-release is missing)"

  # shellcheck disable=SC1091
  source /etc/os-release
  DISTRO_NAME="${PRETTY_NAME:-${NAME:-Linux}}"
  local distro_tokens=" ${ID:-} ${ID_LIKE:-} "

  case "$distro_tokens" in
    *" debian "*|*" ubuntu "*)
      DISTRO_FAMILY="debian"
      PACKAGE_MANAGER="apt-get"
      ;;
    *" fedora "*|*" rhel "*|*" centos "*)
      DISTRO_FAMILY="fedora"
      if command -v dnf >/dev/null 2>&1; then
        DNF_COMMAND="dnf"
      elif command -v dnf5 >/dev/null 2>&1; then
        DNF_COMMAND="dnf5"
      else
        die "Fedora-family system detected, but neither dnf nor dnf5 is available"
      fi
      PACKAGE_MANAGER="$DNF_COMMAND"
      ;;
    *" arch "*)
      DISTRO_FAMILY="arch"
      PACKAGE_MANAGER="pacman"
      ;;
    *)
      die "unsupported distribution: $DISTRO_NAME. Supported families: Debian/Ubuntu, Fedora/RHEL, and Arch"
      ;;
  esac

  if [[ "$DISTRO_FAMILY" == "fedora" ]] &&
    [[ ! -e /.toolboxenv && ! -e /run/.containerenv ]] &&
    command -v rpm-ostree >/dev/null 2>&1; then
    if rpm-ostree status >/dev/null 2>&1; then
      die "immutable Fedora variants need a build container/toolbox; run this installer inside a mutable Fedora toolbox"
    fi
  fi
}

detect_desktop() {
  SESSION_TYPE="${XDG_SESSION_TYPE:-}"
  if [[ -z "$SESSION_TYPE" ]]; then
    if [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
      SESSION_TYPE="wayland"
    elif [[ -n "${DISPLAY:-}" ]]; then
      SESSION_TYPE="x11"
    else
      SESSION_TYPE="unknown"
    fi
  fi
  SESSION_TYPE="${SESSION_TYPE,,}"

  local desktop="${XDG_CURRENT_DESKTOP:-}:${XDG_SESSION_DESKTOP:-}:${DESKTOP_SESSION:-}"
  desktop="${desktop,,}"

  if [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" || "$desktop" == *hypr* ]]; then
    COMPOSITOR="hyprland"
  elif [[ -n "${SWAYSOCK:-}" || "$desktop" == *sway* ]]; then
    COMPOSITOR="wlroots"
  elif [[ "$desktop" == *gnome* || "$desktop" == *unity* || "$desktop" == *budgie* ]]; then
    COMPOSITOR="gnome"
  elif [[ "$desktop" == *kde* || "$desktop" == *plasma* || "$desktop" == *lxqt* ]]; then
    COMPOSITOR="kde"
  elif [[ "$desktop" == *cosmic* ]]; then
    COMPOSITOR="cosmic"
  elif [[ "$SESSION_TYPE" == "wayland" ]]; then
    COMPOSITOR="generic-wayland"
  elif [[ "$SESSION_TYPE" == "x11" ]]; then
    COMPOSITOR="x11"
  else
    COMPOSITOR="unknown"
  fi
}

select_default_bundle() {
  if [[ "$BUNDLE" != "auto" ]]; then
    return
  fi

  case "$DISTRO_FAMILY" in
    debian) BUNDLE="deb" ;;
    fedora) BUNDLE="flatpak" ;;
    arch) BUNDLE="appimage" ;;
  esac
}

configure_privilege_command() {
  if ((EUID == 0)); then
    PRIVILEGE=()
  elif command -v sudo >/dev/null 2>&1; then
    PRIVILEGE=(sudo)
  elif command -v doas >/dev/null 2>&1; then
    PRIVILEGE=(doas)
  else
    die "sudo or doas is required to install system dependencies"
  fi
}

package_is_available() {
  local package="$1"

  case "$DISTRO_FAMILY" in
    debian)
      dpkg-query -W -f='${Status}' "$package" 2>/dev/null | grep -q 'ok installed' ||
        apt-cache show "$package" >/dev/null 2>&1
      ;;
    fedora)
      rpm -q "$package" >/dev/null 2>&1 ||
        "$DNF_COMMAND" -q list --available "$package" >/dev/null 2>&1
      ;;
    arch)
      pacman -Q "$package" >/dev/null 2>&1 ||
        pacman -Si "$package" >/dev/null 2>&1
      ;;
  esac
}

select_package_from_group() {
  local group="$1"
  local candidate
  local -a candidates=()
  IFS='|' read -r -a candidates <<<"$group"

  for candidate in "${candidates[@]}"; do
    if package_is_available "$candidate"; then
      SELECTED_PACKAGE="$candidate"
      return 0
    fi
  done

  SELECTED_PACKAGE=""
  return 1
}

resolve_package_groups() {
  local group
  RESOLVED_REQUIRED_PACKAGES=()
  RESOLVED_OPTIONAL_PACKAGES=()

  for group in "${REQUIRED_PACKAGE_GROUPS[@]}"; do
    if ! select_package_from_group "$group"; then
      die "none of the required packages are available: ${group//|/ or }"
    fi
    RESOLVED_REQUIRED_PACKAGES+=("$SELECTED_PACKAGE")
  done

  for group in "${OPTIONAL_PACKAGE_GROUPS[@]}"; do
    if select_package_from_group "$group"; then
      RESOLVED_OPTIONAL_PACKAGES+=("$SELECTED_PACKAGE")
    else
      warn "optional desktop helper is unavailable: ${group//|/ or }"
    fi
  done
}

configure_package_groups() {
  case "$DISTRO_FAMILY" in
    debian)
      REQUIRED_PACKAGE_GROUPS=(
        build-essential git curl wget file pkg-config
        libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libssl-dev
        "libayatana-appindicator3-dev|libappindicator3-dev"
        librsvg2-dev libasound2-dev libsecret-1-dev
        clang cmake patchelf at-spi2-core
      )
      OPTIONAL_PACKAGE_GROUPS=(gnome-keyring)
      ;;
    fedora)
      REQUIRED_PACKAGE_GROUPS=(
        gcc gcc-c++ make binutils git curl wget file pkgconf-pkg-config
        webkit2gtk4.1-devel gtk3-devel openssl-devel
        "libayatana-appindicator-gtk3-devel|libappindicator-gtk3-devel"
        librsvg2-devel alsa-lib-devel libsecret-devel
        clang cmake patchelf at-spi2-core
      )
      OPTIONAL_PACKAGE_GROUPS=(gnome-keyring)
      ;;
    arch)
      REQUIRED_PACKAGE_GROUPS=(
        base-devel git curl wget file pkgconf
        webkit2gtk-4.1 gtk3 openssl
        "libayatana-appindicator|libappindicator-gtk3"
        librsvg alsa-lib libsecret
        clang cmake patchelf at-spi2-core
      )
      OPTIONAL_PACKAGE_GROUPS=(gnome-keyring appmenu-gtk-module)
      ;;
  esac

  if [[ "$BUNDLE" == "flatpak" ]]; then
    REQUIRED_PACKAGE_GROUPS+=(flatpak flatpak-builder binutils)
  elif [[ "$BUNDLE" == "appimage" ]]; then
    case "$DISTRO_FAMILY" in
      debian) REQUIRED_PACKAGE_GROUPS+=("libfuse2|libfuse2t64") ;;
      fedora) REQUIRED_PACKAGE_GROUPS+=(fuse-libs) ;;
      arch) REQUIRED_PACKAGE_GROUPS+=(fuse2) ;;
    esac
  fi

  if [[ "$SESSION_TYPE" == "wayland" ]]; then
    REQUIRED_PACKAGE_GROUPS+=(xdg-desktop-portal)
    OPTIONAL_PACKAGE_GROUPS+=(ydotool wtype wl-clipboard xdg-desktop-portal-gtk)
    case "$COMPOSITOR" in
      gnome) OPTIONAL_PACKAGE_GROUPS+=(xdg-desktop-portal-gnome) ;;
      kde) OPTIONAL_PACKAGE_GROUPS+=(xdg-desktop-portal-kde) ;;
      hyprland) OPTIONAL_PACKAGE_GROUPS+=(xdg-desktop-portal-hyprland) ;;
      wlroots) OPTIONAL_PACKAGE_GROUPS+=(xdg-desktop-portal-wlr) ;;
      cosmic) OPTIONAL_PACKAGE_GROUPS+=(xdg-desktop-portal-cosmic) ;;
    esac
  elif [[ "$SESSION_TYPE" == "x11" ]]; then
    OPTIONAL_PACKAGE_GROUPS+=(xdotool)
  else
    OPTIONAL_PACKAGE_GROUPS+=(xdotool ydotool wtype wl-clipboard xdg-desktop-portal)
  fi
}

install_system_dependencies() {
  if [[ "$SKIP_SYSTEM_DEPS" == true ]]; then
    warn "skipping system package installation; requirement checks will still run"
    return
  fi

  configure_privilege_command
  log "Refreshing $PACKAGE_MANAGER package metadata"
  case "$DISTRO_FAMILY" in
    debian) "${PRIVILEGE[@]}" apt-get update ;;
    fedora) "${PRIVILEGE[@]}" "$DNF_COMMAND" -q makecache ;;
    arch) info "Using the current pacman sync database (run 'sudo pacman -Syu' first if it is stale)." ;;
  esac

  configure_package_groups
  resolve_package_groups

  log "Installing native build and desktop integration packages"
  case "$DISTRO_FAMILY" in
    debian)
      "${PRIVILEGE[@]}" apt-get install -y \
        "${RESOLVED_REQUIRED_PACKAGES[@]}" "${RESOLVED_OPTIONAL_PACKAGES[@]}"
      ;;
    fedora)
      "${PRIVILEGE[@]}" "$DNF_COMMAND" install -y \
        "${RESOLVED_REQUIRED_PACKAGES[@]}" "${RESOLVED_OPTIONAL_PACKAGES[@]}"
      ;;
    arch)
      "${PRIVILEGE[@]}" pacman -S --needed --noconfirm \
        "${RESOLVED_REQUIRED_PACKAGES[@]}" "${RESOLVED_OPTIONAL_PACKAGES[@]}"
      ;;
  esac
}

version_at_least() {
  local actual="$1"
  local minimum="$2"
  [[ "$(printf '%s\n%s\n' "$minimum" "$actual" | sort -V | head -n1)" == "$minimum" ]]
}

node_is_compatible() {
  command -v node >/dev/null 2>&1 || return 1
  local version major minor
  version="$(node --version | sed 's/^v//')"
  IFS=. read -r major minor _ <<<"$version"

  if ((major == 20 && minor >= 19)); then
    return 0
  fi
  if ((major == 22 && minor >= 12)); then
    return 0
  fi
  ((major > 22))
}

ensure_node() {
  if node_is_compatible && command -v npm >/dev/null 2>&1; then
    info "Node.js $(node --version) and npm $(npm --version)"
    return
  fi

  log "Installing Node.js $NODE_LTS LTS with nvm"
  export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
  if [[ ! -s "$NVM_DIR/nvm.sh" ]]; then
    [[ ! -e "$NVM_DIR" ]] || die "$NVM_DIR exists but does not contain nvm.sh; repair it or set NVM_DIR"
    git clone --depth 1 --branch "$NVM_VERSION" https://github.com/nvm-sh/nvm.git "$NVM_DIR"
  fi

  # shellcheck disable=SC1091
  source "$NVM_DIR/nvm.sh"
  nvm install "$NODE_LTS" --latest-npm
  nvm alias default "$NODE_LTS"
  nvm use "$NODE_LTS"
  node_is_compatible || die "Node.js installation did not provide a Vite-compatible version"
}

rust_is_compatible() {
  command -v rustc >/dev/null 2>&1 || return 1
  command -v cargo >/dev/null 2>&1 || return 1
  local version
  version="$(rustc --version | awk '{print $2}')"
  version_at_least "$version" "1.85.0"
}

ensure_rust() {
  if rust_is_compatible; then
    info "$(rustc --version)"
    return
  fi

  log "Installing the stable Rust toolchain with rustup"
  local rustup_installer
  rustup_installer="$(mktemp "${TMPDIR:-/tmp}/oto-rustup.XXXXXX")"
  curl --proto '=https' --tlsv1.2 -fsS https://sh.rustup.rs -o "$rustup_installer"
  sh "$rustup_installer" -y --profile minimal --default-toolchain stable
  rm -f -- "$rustup_installer"

  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
  rust_is_compatible || die "Rust installation did not provide rustc 1.85+ and cargo"
}

prepare_source() {
  if [[ -n "$LOCAL_SOURCE" ]]; then
    [[ -d "$LOCAL_SOURCE" ]] || die "local source directory does not exist: $LOCAL_SOURCE"
    SOURCE_DIR="$(cd "$LOCAL_SOURCE" && pwd)"
    info "Using local checkout: $SOURCE_DIR"
  elif [[ ! -e "$SOURCE_DIR" ]]; then
    log "Cloning $REPOSITORY"
    mkdir -p "$(dirname "$SOURCE_DIR")"
    git clone "$REPOSITORY" "$SOURCE_DIR"
  else
    [[ -d "$SOURCE_DIR/.git" ]] || die "source path exists but is not a Git checkout: $SOURCE_DIR"
    local existing_remote
    existing_remote="$(git -C "$SOURCE_DIR" remote get-url origin)"
    [[ "$existing_remote" == "$REPOSITORY" ]] ||
      die "$SOURCE_DIR tracks $existing_remote, not $REPOSITORY; choose another --source-dir"
  fi

  [[ -f "$SOURCE_DIR/package.json" && -f "$SOURCE_DIR/src-tauri/Cargo.toml" ]] ||
    die "$SOURCE_DIR is not an Oto source checkout"

  if [[ -n "$LOCAL_SOURCE" ]]; then
    return
  fi

  [[ -z "$(git -C "$SOURCE_DIR" status --porcelain --untracked-files=no)" ]] ||
    die "installer checkout has local changes: $SOURCE_DIR"

  log "Updating the Oto source checkout"
  if [[ "$(git -C "$SOURCE_DIR" rev-parse --is-shallow-repository)" == "true" ]]; then
    git -C "$SOURCE_DIR" fetch --unshallow origin
  fi
  if [[ -n "$GIT_REF" ]]; then
    git -C "$SOURCE_DIR" fetch origin "$GIT_REF"
    git -C "$SOURCE_DIR" switch --detach FETCH_HEAD
  else
    git -C "$SOURCE_DIR" remote set-head origin -a >/dev/null
    local default_branch
    default_branch="$(git -C "$SOURCE_DIR" symbolic-ref --short refs/remotes/origin/HEAD)"
    default_branch="${default_branch#origin/}"
    git -C "$SOURCE_DIR" fetch --depth 1 origin "$default_branch"
    if git -C "$SOURCE_DIR" show-ref --verify --quiet "refs/heads/$default_branch"; then
      git -C "$SOURCE_DIR" switch "$default_branch"
    else
      git -C "$SOURCE_DIR" switch --track -c "$default_branch" "origin/$default_branch"
    fi
    git -C "$SOURCE_DIR" merge --ff-only "origin/$default_branch"
  fi
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "required command is missing after dependency installation: $1"
}

require_pkg_config_module() {
  pkg-config --exists "$1" ||
    die "required development library is missing from pkg-config: $1"
}

verify_requirements() {
  log "Checking build requirements"
  local command
  for command in git curl wget file pkg-config clang cmake patchelf node npm rustc cargo; do
    require_command "$command"
  done
  if [[ "$BUNDLE" == "flatpak" ]]; then
    require_command flatpak
    require_command flatpak-builder
    require_command ar
  fi

  local module
  for module in gtk+-3.0 webkit2gtk-4.1 alsa libsecret-1 librsvg-2.0; do
    require_pkg_config_module "$module"
  done

  if ! pkg-config --exists ayatana-appindicator3-0.1 &&
    ! pkg-config --exists appindicator3-0.1; then
    die "AppIndicator development files are missing (required by Oto's system tray)"
  fi

  node_is_compatible || die "Oto requires Node.js 20.19+ or 22.12+"
  rust_is_compatible || die "Oto requires rustc 1.85+ and cargo"

  info "All compiler, toolchain, WebKitGTK, GTK, audio, keyring, and tray checks passed."
}

read_app_version() {
  node -e '
    const fs = require("node:fs");
    const config = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    process.stdout.write(config.version);
  ' "$SOURCE_DIR/src-tauri/tauri.conf.json"
}

resolve_artifact() {
  local directory="$1"
  local exact_pattern="$2"
  local fallback_pattern="$3"
  local label="$4"
  local -a matches=()

  shopt -s nullglob
  matches=("$directory"/$exact_pattern)
  if ((${#matches[@]} == 0)); then
    matches=("$directory"/$fallback_pattern)
  fi
  shopt -u nullglob

  ((${#matches[@]} > 0)) || die "$label build completed without producing an artifact in $directory"
  if ((${#matches[@]} > 1)); then
    die "multiple $label artifacts found in $directory; remove stale bundles and retry"
  fi
  ARTIFACT="${matches[0]}"
}

build_bundle() {
  local version="$1"
  log "Installing locked JavaScript dependencies"
  (
    cd "$SOURCE_DIR"
    # The frontend and Tauri CLIs are devDependencies. Explicitly include them
    # even when the user's shell exports NODE_ENV=production.
    npm ci --include=dev
  )

  log "Checking the Svelte frontend"
  (
    cd "$SOURCE_DIR"
    npm run check
  )

  case "$BUNDLE" in
    deb)
      log "Building the Debian bundle"
      (
        cd "$SOURCE_DIR"
        npm run tauri build -- --bundles deb
      )
      resolve_artifact \
        "$SOURCE_DIR/src-tauri/target/release/bundle/deb" \
        "${APP_NAME}_${version}_*.deb" "*.deb" "Debian"
      ;;
    appimage)
      log "Building the AppImage bundle"
      (
        cd "$SOURCE_DIR"
        npm run tauri build -- --bundles appimage
      )
      resolve_artifact \
        "$SOURCE_DIR/src-tauri/target/release/bundle/appimage" \
        "${APP_NAME}_${version}_*.AppImage" "*.AppImage" "AppImage"
      ;;
    flatpak)
      log "Building the Debian payload used by the Flatpak manifest"
      (
        cd "$SOURCE_DIR"
        npm run tauri build -- --bundles deb
      )
      resolve_artifact \
        "$SOURCE_DIR/src-tauri/target/release/bundle/deb" \
        "${APP_NAME}_${version}_*.deb" "*.deb" "Debian payload"
      build_flatpak "$version"
      ;;
  esac
}

build_flatpak() {
  local version="$1"
  local manifest="$SOURCE_DIR/packaging/dev.oto.app.yml"
  local payload="$SOURCE_DIR/packaging/oto.deb"
  local build_dir="$SOURCE_DIR/.flatpak-build"
  local repository_dir="$SOURCE_DIR/.flatpak-repo"
  local output_dir="$SOURCE_DIR/dist"
  local output_bundle="$output_dir/${APP_NAME}_${version}_$(uname -m).flatpak"
  local temporary_bundle="${output_bundle}.tmp.$$"

  [[ -f "$manifest" ]] || die "Flatpak manifest is missing: $manifest"
  install -m 0644 "$ARTIFACT" "$payload"
  mkdir -p "$output_dir"

  log "Building the Flatpak"
  flatpak remote-add --user --if-not-exists flathub "$FLATHUB_REPOSITORY"
  flatpak-builder \
    --force-clean \
    --user \
    --install-deps-from=flathub \
    --repo="$repository_dir" \
    "$build_dir" \
    "$manifest"

  flatpak build-bundle \
    "$repository_dir" \
    "$temporary_bundle" \
    "$APP_ID" \
    --runtime-repo="$FLATHUB_REPOSITORY"
  mv -f -- "$temporary_bundle" "$output_bundle"
  ARTIFACT="$output_bundle"
}

install_deb() {
  configure_privilege_command
  log "Installing $ARTIFACT"
  "${PRIVILEGE[@]}" apt-get install -y "$ARTIFACT"
}

install_flatpak() {
  log "Installing $ARTIFACT for the current user"
  flatpak install --user -y --reinstall "$ARTIFACT"
}

install_appimage() {
  configure_privilege_command
  local appimage_path="/opt/oto/Oto.AppImage"

  log "Installing the AppImage under /opt/oto"
  "${PRIVILEGE[@]}" install -Dm755 "$ARTIFACT" "$appimage_path"
  "${PRIVILEGE[@]}" ln -sfn "$appimage_path" /usr/local/bin/oto
  "${PRIVILEGE[@]}" install -Dm644 \
    "$SOURCE_DIR/packaging/dev.oto.app.desktop" \
    /usr/share/applications/dev.oto.app.desktop

  local size
  for size in 32 128 256 512; do
    "${PRIVILEGE[@]}" install -Dm644 \
      "$SOURCE_DIR/src-tauri/icons/${size}x${size}.png" \
      "/usr/share/icons/hicolor/${size}x${size}/apps/dev.oto.app.png"
  done

  if command -v update-desktop-database >/dev/null 2>&1; then
    "${PRIVILEGE[@]}" update-desktop-database /usr/share/applications
  fi
}

install_built_bundle() {
  if [[ "$INSTALL_BUNDLE" != true ]]; then
    warn "build-only mode selected; the bundle was not installed"
    return
  fi

  case "$BUNDLE" in
    deb)
      [[ "$DISTRO_FAMILY" == "debian" ]] ||
        die "automatic .deb installation is supported only on Debian-family systems"
      install_deb
      ;;
    flatpak) install_flatpak ;;
    appimage) install_appimage ;;
  esac
}

print_desktop_notes() {
  if [[ "$SESSION_TYPE" == "wayland" ]]; then
    if [[ "$BUNDLE" == "flatpak" ]]; then
      warn "Flatpak cannot execute host ydotool/wtype binaries; use AT-SPI or clipboard-only insertion if synthetic paste is blocked."
    elif command -v ydotool >/dev/null 2>&1; then
      info "Wayland: enable ydotool with 'systemctl --user enable --now ydotool.service'."
      info "If /dev/uinput is denied, add your user to the input group and fully log out/in."
    elif command -v wtype >/dev/null 2>&1; then
      info "Wayland: wtype is installed; ydotool is recommended when the compositor blocks virtual-keyboard input."
    else
      warn "no Wayland typing helper was available; Oto will use AT-SPI and clipboard fallbacks"
    fi
  elif [[ "$SESSION_TYPE" == "x11" ]] && ! command -v xdotool >/dev/null 2>&1; then
    warn "xdotool is unavailable; Oto will use AT-SPI and clipboard fallbacks on X11"
  fi
}

main() {
  detect_distribution
  detect_desktop
  select_default_bundle
  if [[ "$BUNDLE" == "deb" && "$DISTRO_FAMILY" != "debian" && "$INSTALL_BUNDLE" == true ]]; then
    die "automatic .deb installation is supported only on Debian-family systems (use --build-only elsewhere)"
  fi

  log "Oto Linux installer"
  info "Distribution: $DISTRO_NAME ($DISTRO_FAMILY family)"
  info "Session: $SESSION_TYPE; desktop/compositor: $COMPOSITOR"
  info "Bundle: $BUNDLE"

  install_system_dependencies
  prepare_source
  ensure_node
  ensure_rust
  verify_requirements

  local version
  version="$(read_app_version)"
  build_bundle "$version"
  install_built_bundle
  print_desktop_notes

  log "Oto installation complete"
  info "Bundle: $ARTIFACT"
  if [[ "$INSTALL_BUNDLE" == true ]]; then
    case "$BUNDLE" in
      flatpak) info "Launch with: flatpak run $APP_ID" ;;
      deb|appimage) info "Launch with: oto" ;;
    esac
  fi
}

main
