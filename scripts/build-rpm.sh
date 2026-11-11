#!/usr/bin/env bash
# Package a built Flutter Linux bundle as an RPM for Fedora and friends.
# Installs to /opt/self26 with a /usr/bin/self26 symlink, matching the
# layout used by the .deb package built in CI.
#
# Usage: scripts/build-rpm.sh <bundle-dir> <version> <rpm-arch> <output-file> [release]
#   rpm-arch: x86_64 or aarch64
#   release: rpm release number, defaults to 1 (CI passes the build number so
#            nightly rpms stay upgradeable via dnf)
set -euo pipefail

BUNDLE_DIR="${1:?usage: build-rpm.sh <bundle-dir> <version> <rpm-arch> <output-file> [release]}"
VERSION="${2:?usage: build-rpm.sh <bundle-dir> <version> <rpm-arch> <output-file> [release]}"
ARCH="${3:?usage: build-rpm.sh <bundle-dir> <version> <rpm-arch> <output-file> [release]}"
OUT="${4:?usage: build-rpm.sh <bundle-dir> <version> <rpm-arch> <output-file> [release]}"
RELEASE="${5:-1}"

if ! command -v rpmbuild >/dev/null 2>&1; then
  echo "build-rpm: rpmbuild not found (install the rpm/rpm-build package)" >&2
  exit 1
fi
if [ ! -d "$BUNDLE_DIR" ]; then
  echo "build-rpm: bundle dir not found: $BUNDLE_DIR" >&2
  exit 1
fi
if [ "$ARCH" != "x86_64" ] && [ "$ARCH" != "aarch64" ]; then
  echo "build-rpm: unsupported arch: $ARCH" >&2
  exit 1
fi

# Desktop integration assets shared across Linux packages live in
# packaging/linux/ (the AUR packages pull the same files via aur-publish.sh).
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." 2>/dev/null && pwd)"
LINUX_ASSETS="$REPO_ROOT/packaging/linux"
for f in "$LINUX_ASSETS/self26.desktop" "$LINUX_ASSETS/self26.svg" "$REPO_ROOT/LICENSE"; do
  if [ ! -f "$f" ]; then
    echo "build-rpm: missing packaging asset: $f" >&2
    exit 1
  fi
done

# rpm forbids '-' in Version; '~' is the rpm convention for prereleases and
# sorts before the final release.
RPM_VERSION="$(printf '%s' "$VERSION" | tr '-' '~')"

TOPDIR="$(mktemp -d)"
trap 'rm -rf "$TOPDIR"' EXIT
mkdir -p "$TOPDIR"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# rpmbuild turns every DT_NEEDED of the bundled binaries into a Requires, but
# the plugin and flutter libs we ship are excluded from Provides below. Filter
# out Requires on any soname that exists inside the bundle, otherwise installs
# fail unless some unrelated package happens to provide the same sonames.
BUNDLED_SOS="$(find "$BUNDLE_DIR" -type f -name '*.so*' -printf '%f\n' | sort -u | sed 's/\./[.]/g' | paste -sd'|')"
REQUIRES_EXCLUDE="^(${BUNDLED_SOS})([(].*[)])*$"

cat > "$TOPDIR/SPECS/self26.spec" <<'EOF'
Name: self26
Version: %{pkg_version}
Release: %{pkg_release}%{?dist}
Summary: Self26 desktop client
License: AGPL-3.0-only
URL: https://gitlab.com/HttpAnimations/self26

# The bundle is already compiled; skip debug packages and buildroot policy
# scripts (strip, rpath checks) that would mangle or reject the prebuilt libs.
%global debug_package %{nil}
%global __os_install_post %{nil}
# Bundled libs must not leak into the rpm provides namespace.
%global __provides_exclude_from ^/opt/self26/.*
%global __requires_exclude %{requires_exclude}

%description
A secure, self-hostable Material 3 client for managing Devin work.

%install
mkdir -p %{buildroot}/opt/self26 %{buildroot}/usr/bin
cp -a "%{bundle_dir}/." %{buildroot}/opt/self26/
ln -sf /opt/self26/self26_frontend %{buildroot}/usr/bin/self26
# The desktop file is named after the app id so Wayland/GNOME can match the
# running window to the launcher without relying on StartupWMClass alone.
install -Dm644 "%{linux_assets}/self26.desktop" %{buildroot}/usr/share/applications/gitlab.httpanimations.self26.desktop
install -Dm644 "%{linux_assets}/self26.svg" %{buildroot}/usr/share/icons/hicolor/scalable/apps/self26.svg
install -Dm644 "%{license_file}" %{buildroot}/usr/share/licenses/self26/LICENSE

%files
/opt/self26
/usr/bin/self26
/usr/share/applications/gitlab.httpanimations.self26.desktop
/usr/share/icons/hicolor/scalable/apps/self26.svg
%license /usr/share/licenses/self26/LICENSE
EOF

rpmbuild -bb \
  --target "$ARCH" \
  --define "_topdir $TOPDIR" \
  --define "pkg_version $RPM_VERSION" \
  --define "pkg_release $RELEASE" \
  --define "bundle_dir $(realpath "$BUNDLE_DIR")" \
  --define "linux_assets $LINUX_ASSETS" \
  --define "license_file $REPO_ROOT/LICENSE" \
  --define "requires_exclude $REQUIRES_EXCLUDE" \
  "$TOPDIR/SPECS/self26.spec"

RPM_PATH="$(find "$TOPDIR/RPMS" -name '*.rpm' -print -quit)"
if [ -z "$RPM_PATH" ]; then
  echo "build-rpm: rpmbuild produced no rpm" >&2
  exit 1
fi
cp "$RPM_PATH" "$OUT"
echo "build-rpm: $RPM_PATH -> $OUT"
