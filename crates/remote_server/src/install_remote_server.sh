#!/usr/bin/env bash
# 在远端主机安装 ZapNG CLI 二进制,用于 remote-server-proxy。
#
# setup.rs 会在运行时替换这些占位符:
#   {release_tag}           - the release tag the client was built from, such as v1.2; empty for a
#                             build without one, which installs the newest release's
#   {latest_release_url}    - the page that redirects to the newest release, .../releases/latest
#   {download_url}          - the release's tarball for this system, with shell variables:
#                             .../releases/download/v$release_version/zap-ng-$release_version-$os_name-$arch_name.tar.gz
#   {checksum_url}          - the tarball's SHA-256 file that the release publishes beside it, {download_url}.sha256
#   {expected_sha256}       - the tarball's SHA-256, which a client built from a release read from that file
#                             after checking the release's signature of it; empty for a build without a
#                             release tag, and the script then downloads {checksum_url} itself
#   {install_dir}           - 例如 ~/.zap-ng/remote-server
#   {binary_name}           - 例如 zap-ng
#   {version_suffix}        - 例如 -v0.2026...,没有 release tag 时为空
#   {staging_tarball_path}  - SCP fallback 预上传 tarball 路径,常规下载路径为空
#
# A downloaded tarball is extracted only once its SHA-256 matches the one its release publishes.
# The exit codes for that, besides 1 (any other error), 2 (unsupported system), 3 (no downloader)
# and the downloader's own when the tarball can't be downloaded:
#   4 - neither sha256sum nor shasum is available to verify the tarball
#   5 - the release's SHA-256 file can't be downloaded or holds no SHA-256: releases made before
#       checksums were published have none, and are not installed
#   6 - the tarball's SHA-256 differs from the published one
# None of them stops the client's SCP fallback, which downloads the tarball and verifies it on
# the client: a tarball it uploads is extracted without a check here.
set -e

arch=$(uname -m)
case "$arch" in
  x86_64|amd64)  arch_name=x86_64 ;;
  aarch64|arm64) arch_name=aarch64 ;;
  *) echo "unsupported arch: $arch" >&2; exit 2 ;;
esac

os_kernel=$(uname -s)
case "$os_kernel" in
  Darwin) os_name=macos ;;
  Linux)  os_name=linux ;;
  *) echo "unsupported OS: $os_kernel" >&2; exit 2 ;;
esac

install_dir="{install_dir}"
case "$install_dir" in
  "~"|"~/"*) install_dir="${HOME}${install_dir#\~}" ;;
esac
mkdir -p "$install_dir"

tmpdir=$(mktemp -d "$install_dir/.install.XXXXXX")
# 尽力清理 staging 目录。这里失败不能覆盖真正的安装结果:
# trap 触发时二进制要么已经移动到最终路径,要么脚本已经因为
# 其他原因失败,后者的错误更值得暴露给调用方。
cleanup() {
  rm -rf "$tmpdir" 2>/dev/null || true
}
trap cleanup EXIT

staging_tarball_path="{staging_tarball_path}"
if [ -n "$staging_tarball_path" ]; then
  case "$staging_tarball_path" in
    "~"|"~/"*) staging_tarball_path="${HOME}${staging_tarball_path#\~}" ;;
  esac
  mv "$staging_tarball_path" "$tmpdir/zap-ng.tar.gz"
else
  # sha256sum on Linux (also BusyBox's), shasum on macOS.
  if command -v sha256sum >/dev/null 2>&1; then
    sha256_tool=sha256sum
  elif command -v shasum >/dev/null 2>&1; then
    sha256_tool=shasum
  else
    echo "error: neither sha256sum nor shasum is available to verify the download of ZapNG's remote server" >&2
    exit 4
  fi

  if command -v curl >/dev/null 2>&1; then
    downloader=curl
  elif command -v wget >/dev/null 2>&1; then
    downloader=wget
  else
    echo "error: neither curl nor wget is available" >&2
    exit 3
  fi

  release_tag="{release_tag}"
  if [ -z "$release_tag" ]; then
    # The newest release's tag ends the URL its page redirects to, .../releases/tag/<tag>, which
    # both programs print in the response's Location header.
    if [ "$downloader" = curl ]; then
      headers=$(curl -sSI --connect-timeout 15 "{latest_release_url}") || true
    else
      headers=$(wget -S --spider "{latest_release_url}" 2>&1) || true
    fi
    release_tag=$(printf '%s\n' "$headers" | sed -n 's#^.*[Ll]ocation:.*/releases/tag/\([^[:space:]]*\).*$#\1#p' | head -n 1)
    if [ -z "$release_tag" ]; then
      echo "error: could not find the newest release of ZapNG: {latest_release_url} did not redirect to a release" >&2
      exit 1
    fi
  fi
  # A release's files carry its version, the tag without its v.
  case "$release_tag" in
    v?*) release_version="${release_tag#v}" ;;
    *) echo "error: the release tag $release_tag is not a version tag (v<version>)" >&2; exit 1 ;;
  esac

  # The URL names the release with $release_version, and this system with $os_name and $arch_name.
  url="{download_url}"

  checksum_url="{checksum_url}"
  # From the client when it has one: it checked the release's signature, which this host can't.
  expected_sha256="{expected_sha256}"
  if [ -z "$expected_sha256" ]; then
    # The SHA-256 first, so that a release without one isn't downloaded for nothing.
    if [ "$downloader" = curl ]; then
      curl -fsSL --connect-timeout 15 "$checksum_url" -o "$tmpdir/zap-ng.tar.gz.sha256" || checksum_status=$?
    else
      wget -q -O "$tmpdir/zap-ng.tar.gz.sha256" "$checksum_url" || checksum_status=$?
    fi
    if [ -n "${checksum_status:-}" ]; then
      echo "error: could not download the SHA-256 of the remote server, $checksum_url (exit $checksum_status); a release made before checksums were published has none, and its tarball is not installed unverified" >&2
      exit 5
    fi
    # The file is sha256sum's output: the SHA-256 in hex, then the file's name.
    read -r expected_sha256 _ < "$tmpdir/zap-ng.tar.gz.sha256" || true
  fi
  expected_sha256=$(printf '%s' "${expected_sha256:-}" | tr 'A-F' 'a-f')
  case "$expected_sha256" in
    *[!0-9a-f]*|"") expected_sha256= ;;
  esac
  if [ "${#expected_sha256}" -ne 64 ]; then
    echo "error: $checksum_url holds no SHA-256" >&2
    exit 5
  fi

  if [ "$downloader" = curl ]; then
    curl -fSL --connect-timeout 15 "$url" -o "$tmpdir/zap-ng.tar.gz" || download_status=$?
  else
    wget -q -O "$tmpdir/zap-ng.tar.gz" "$url" || download_status=$?
  fi
  if [ -n "${download_status:-}" ]; then
    echo "error: could not download $url" >&2
    exit "$download_status"
  fi

  # From standard input, as the output then holds no path that the program could escape.
  if [ "$sha256_tool" = sha256sum ]; then
    actual_sha256=$(sha256sum < "$tmpdir/zap-ng.tar.gz")
  else
    actual_sha256=$(shasum -a 256 < "$tmpdir/zap-ng.tar.gz")
  fi
  actual_sha256=$(printf '%s' "${actual_sha256%% *}" | tr 'A-F' 'a-f')
  if [ "$actual_sha256" != "$expected_sha256" ]; then
    echo "error: SHA-256 mismatch for $url: expected $expected_sha256, got $actual_sha256; the tarball is not installed" >&2
    exit 6
  fi
fi

tar -xzf "$tmpdir/zap-ng.tar.gz" -C "$tmpdir"

bin="$tmpdir/{binary_name}"
if [ ! -f "$bin" ]; then
  bin=$(find "$tmpdir" -type f \( -name 'zap-ng' -o -name 'warp-oss' -o -name 'oz*' \) ! -path "$tmpdir/resources/*" ! -name '*.tar.gz' | head -n1)
fi
if [ -z "$bin" ]; then echo "no binary found in tarball" >&2; exit 1; fi
chmod +x "$bin"
mv "$bin" "$install_dir/{binary_name}{version_suffix}"
