#!/usr/bin/env bash
# notes-tui installer for Linux (x86_64 / aarch64).
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/ItsNotArjun/notes-tui/main/install.sh | bash
#   curl -fsSL .../install.sh | bash -s -- --from-source
#   ./install.sh --dir ~/.local/bin --tag v0.1.0
set -euo pipefail

REPO="ItsNotArjun/notes-tui"
BIN_NAME="notes"
DEFAULT_DIR="$HOME/.local/bin"
INSTALL_DIR="${INSTALL_DIR:-$DEFAULT_DIR}"
FROM_SOURCE=0
TAG=""

log() { printf '%s\n' "$*"; }
err() { printf 'error: %s\n' "$*" >&2; }

usage() {
  cat <<EOF
notes installer (Linux)

Options:
  --dir <path>     Install directory (default: ~/.local/bin)
  --from-source    Skip prebuilt binary, build with cargo
  --tag <vX.Y.Z>   Install specific release (default: latest)
  -h, --help       Show this help
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --dir) INSTALL_DIR="${2:-}"; shift 2 ;;
    --from-source) FROM_SOURCE=1; shift ;;
    --tag) TAG="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) err "unknown arg: $1"; usage; exit 1 ;;
  esac
done

need() { command -v "$1" >/dev/null 2>&1 || { err "missing required tool: $1"; return 1; }; }

detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  [ "$os" = "Linux" ] || { err "only Linux is supported (got $os)"; exit 1; }
  case "$arch" in
    x86_64|amd64) echo "x86_64" ;;
    aarch64|arm64) echo "aarch64" ;;
    *) err "unsupported arch: $arch (need x86_64 or aarch64)"; exit 1 ;;
  esac
}

latest_tag() {
  # Try GitHub API, fall back to empty (caller falls back to source).
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
      | grep -m1 '"tag_name"' | cut -d'"' -f4 || true
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
      | grep -m1 '"tag_name"' | cut -d'"' -f4 || true
  fi
}

download() {
  local url="$1" out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$out"
  else
    wget -qO "$out" "$url"
  fi
}

try_prebuilt() {
  local arch="$1" tag="$2" tmp="$3"
  local asset="notes-linux-${arch}.tar.gz"
  local url="https://github.com/${REPO}/releases/download/${tag}/${asset}"
  log "Trying prebuilt $tag ($arch)..."
  if download "$url" "$tmp/$asset"; then
    if download "$url.sha256" "$tmp/$asset.sha256"; then
      (cd "$tmp" && sha256sum -c "$asset.sha256") || {
        err "checksum mismatch, refusing to install"
        return 1
      }
    else
      log "No checksum file, skipping verify."
    fi
    tar -xzf "$tmp/$asset" -C "$tmp"
    mkdir -p "$INSTALL_DIR"
    install -m 755 "$tmp/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"
    return 0
  fi
  return 1
}

build_from_source() {
  local tag="$1" tmp="$2"
  log "Building from source..."
  need git || { err "install git first (dnf/apt: git)"; exit 1; }
  need cc || { err "install a C compiler (dnf groupinstall 'Development Tools' / apt: build-essential)"; exit 1; }
  if ! command -v cargo >/dev/null 2>&1; then
    log "Rust not found, installing via rustup (minimal profile)..."
    need curl || { err "install curl first"; exit 1; }
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
  fi
  local src="$tmp/src"
  if [ -n "$tag" ]; then
    git clone --depth 1 --branch "$tag" "https://github.com/$REPO.git" "$src"
  else
    git clone --depth 1 "https://github.com/$REPO.git" "$src"
  fi
  (cd "$src" && cargo build --release --locked)
  mkdir -p "$INSTALL_DIR"
  install -m 755 "$src/target/release/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"
}

main() {
  local arch tmp
  arch="$(detect_platform)"
  need tar || exit 1
  command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 || {
    err "need curl or wget"
    exit 1
  }
  tmp="$(mktemp -d)"
  # shellcheck disable=SC2064
  trap "rm -rf '$tmp'" EXIT

  if [ "$FROM_SOURCE" -eq 0 ]; then
    local tag="$TAG"
    [ -n "$tag" ] || tag="$(latest_tag || true)"
    if [ -n "$tag" ] && try_prebuilt "$arch" "$tag" "$tmp"; then
      :
    else
      log "Prebuilt not available, falling back to source build."
      build_from_source "$TAG" "$tmp"
    fi
  else
    build_from_source "$TAG" "$tmp"
  fi

  log "Installed to $INSTALL_DIR/$BIN_NAME"
  if ! "$INSTALL_DIR/$BIN_NAME" --version; then
    err "verify failed"
    exit 1
  fi

  case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
      err "$INSTALL_DIR is not in PATH."
      log "Add it with:"
      log "  export PATH=\"\$HOME/.local/bin:\$PATH\""
      log "Then restart your shell or run: hash -r"
      ;;
  esac

  log "Done. Run with: $BIN_NAME"
  log "Clipboard needs x11/wayland backend; if copy fails install: wl-clipboard (Wayland) or xclip/xsel (X11)."
}

main
