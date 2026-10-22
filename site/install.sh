#!/bin/sh
# Installs Shape for this user on macOS or Linux (no sudo needed):
#
#   curl -fsSL https://dobrinov.github.io/shape/install.sh | sh
#
# shape goes in ~/.local/bin (SHAPE_INSTALL_DIR to change), with `diagram`
# next to it, and .shape files open in it with a double-click. Run it again to
# upgrade. On macOS, Homebrew (brew install dobrinov/tap/shape) works too.
set -eu

main() {
  repo=dobrinov/shape
  case "$(uname -s)" in
    Darwin) os=macos ;;
    Linux) os=linux ;;
    *) die "this installer is for macOS and Linux; on Windows run in PowerShell: irm https://dobrinov.github.io/shape/install.ps1 | iex" ;;
  esac
  case "$(uname -m)" in
    arm64 | aarch64) arch=arm64 ;;
    x86_64 | amd64) arch=x86_64 ;;
    *) die "no Shape build for $(uname -m) yet; see https://github.com/$repo#install" ;;
  esac
  # Apple silicon running this shell under Rosetta still wants the arm64 build
  if [ "$os" = macos ] && [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" = 1 ]; then arch=arm64; fi

  command -v curl >/dev/null || die "curl is needed"
  tag=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1)
  [ -n "$tag" ] || die "could not find the latest release (GitHub rate limit?); try again later"
  version=${tag#v}
  name="shape-$version-$os-$arch.tar.gz"

  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  echo "Downloading Shape $version..."
  curl -fsSL "https://github.com/$repo/releases/download/$tag/$name" -o "$tmp/$name" ||
    die "Shape $version for $os-$arch is not out yet (the release is still being built?); try again in a few minutes"
  curl -fsSL "https://github.com/$repo/releases/download/$tag/SHA256SUMS" -o "$tmp/SHA256SUMS"
  want=$(grep " $name\$" "$tmp/SHA256SUMS" | cut -d' ' -f1)
  if command -v sha256sum >/dev/null; then got=$(sha256sum "$tmp/$name" | cut -d' ' -f1); else got=$(shasum -a 256 "$tmp/$name" | cut -d' ' -f1); fi
  [ -n "$want" ] && [ "$got" = "$want" ] || die "the download is damaged (its checksum does not match); try again"
  tar -xzf "$tmp/$name" -C "$tmp"

  dir=${SHAPE_INSTALL_DIR:-$HOME/.local/bin}
  mkdir -p "$dir"
  # a new file, not overwritten in place: a running Shape keeps its old one
  cp "$tmp/shape" "$dir/.shape.new" && chmod +x "$dir/.shape.new" && mv -f "$dir/.shape.new" "$dir/shape"
  ln -sf shape "$dir/diagram"

  "$dir/shape" install-app || echo "(the double-click on .shape files did not work out: shape install-app tries again)"
  echo
  echo "Shape $version is installed in $dir."
  case ":$PATH:" in
    *":$dir:"*) echo "Open a board with: shape" ;;
    *)
      echo "$dir is not on your PATH yet. Add this line to your shell's profile (~/.zshrc, ~/.bashrc):"
      echo "  export PATH=\"$dir:\$PATH\""
      echo "Until then: $dir/shape"
      ;;
  esac
}

die() {
  echo "shape install: $*" >&2
  exit 1
}

main "$@"
