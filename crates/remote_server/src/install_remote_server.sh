#!/usr/bin/env bash
# 在远端主机安装 Zap CLI 二进制,用于 remote-server-proxy。
#
# setup.rs 会在运行时替换这些占位符:
#   {release_tag}           - the release tag the client was built from, such as v1.2; empty for a
#                             build without one, which installs the newest release's
#   {latest_release_url}    - the page that redirects to the newest release, .../releases/latest
#   {download_url}          - the release's tarball for this system, with shell variables:
#                             .../releases/download/v$release_version/zap-$release_version-$os_name-$arch_name.tar.gz
#   {install_dir}           - 例如 ~/.zap/remote-server
#   {binary_name}           - 例如 zap-oss
#   {version_suffix}        - 例如 -v0.2026...,没有 release tag 时为空
#   {staging_tarball_path}  - SCP fallback 预上传 tarball 路径,常规下载路径为空
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
  mv "$staging_tarball_path" "$tmpdir/zap.tar.gz"
else
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
      echo "error: could not find the newest release of Zap: {latest_release_url} did not redirect to a release" >&2
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
  if [ "$downloader" = curl ]; then
    curl -fSL --connect-timeout 15 "$url" -o "$tmpdir/zap.tar.gz" || download_status=$?
  else
    wget -q -O "$tmpdir/zap.tar.gz" "$url" || download_status=$?
  fi
  if [ -n "${download_status:-}" ]; then
    echo "error: could not download $url" >&2
    exit "$download_status"
  fi
fi

tar -xzf "$tmpdir/zap.tar.gz" -C "$tmpdir"

bin="$tmpdir/{binary_name}"
if [ ! -f "$bin" ]; then
  bin=$(find "$tmpdir" -type f \( -name 'zap-oss' -o -name 'warp-oss' -o -name 'oz*' \) ! -path "$tmpdir/resources/*" ! -name '*.tar.gz' | head -n1)
fi
if [ -z "$bin" ]; then echo "no binary found in tarball" >&2; exit 1; fi
chmod +x "$bin"
mv "$bin" "$install_dir/{binary_name}{version_suffix}"
