#!/usr/bin/env bash
# 编译 yaklang 为 Linux arm64 (aarch64) 的可执行文件
# 用法：在仓库根目录执行 ./build-linux-arm.sh
#
# 产物：./yak_linux_arm64
# 依赖：需要 Go 工具链；首次构建若缺包可先 `go mod download`
#
# 说明：
#   - 本机为 aarch64 时是“原生编译”，CGO 用本机 gcc 即可；
#   - 若在 x86_64 主机上交叉编译，需要安装 aarch64 交叉工具链
#     （Debian/Ubuntu: sudo apt install gcc-aarch64-linux-gnu）。
set -e

# 切到仓库根目录（脚本可能被从任意位置调用）
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_ROOT"

# ---------- 目标平台 ----------
# 注意：yaklang 依赖 go-pcre2-lite / go-sqlite3，需要 CGO（PCRE2、SQLite 都是 C 库），
#       不能设 CGO_ENABLED=0。
export CGO_ENABLED=1
export GOOS=linux
export GOARCH=arm64
OUTPUT="yak_linux_arm64"

# ---------- CGO 交叉编译器 ----------
# 原生 arm64 主机直接用本机 gcc；x86_64 主机则切换到 aarch64 交叉编译器
HOST_ARCH="$(uname -m)"
if [ "$HOST_ARCH" != "aarch64" ] && [ "$HOST_ARCH" != "arm64" ]; then
  if command -v aarch64-linux-gnu-gcc &>/dev/null; then
    export CC=aarch64-linux-gnu-gcc
    export CXX=aarch64-linux-gnu-g++
  else
    echo "[build] ERROR: 交叉编译需要 aarch64-linux-gnu-gcc（sudo apt install gcc-aarch64-linux-gnu）" >&2
    exit 1
  fi
fi

# ---------- 1. 确保 gzip-embed 工具存在 ----------
# 带 gzip_embed 的构建需要先把资源打成 .tar.gz，由本项目的 gzip-embed 工具生成
# gzip-embed 是按当前主机平台编译的工具，与目标 GOOS/GOARCH 无关
SAVED_GOOS="$GOOS"
SAVED_GOARCH="$GOARCH"
if ! command -v gzip-embed &>/dev/null; then
  echo "[build] gzip-embed not found, installing from repo..."
  unset GOOS GOARCH
  go install ./common/utils/gzip_embed/gzip-embed
fi
export GOOS="$SAVED_GOOS"
export GOARCH="$SAVED_GOARCH"

# ---------- 2. 生成 gzip_embed 所需的 .tar.gz 资源 ----------
echo "[build] generating gzip embed resources..."
gzip-embed -cache --source ./common/ai/aid/aitool/buildinaitools/yakscripttools/yakscriptforai --gz ./common/ai/aid/aitool/buildinaitools/yakscripttools/yakscriptforai.tar.gz --no-embed
gzip-embed -cache --source ./common/ai/aid/aireact/skills --gz ./common/ai/aid/aireact/skills.tar.gz --root-path --no-embed
gzip-embed -cache --source ./common/coreplugin/base-yak-plugin --gz ./common/coreplugin/base-yak-plugin.tar.gz --root-path --no-embed
gzip-embed -cache --source ./common/syntaxflow/sfbuildin/buildin --gz ./common/syntaxflow/sfbuildin/buildin.tar.gz --no-embed
gzip-embed -cache --source ./common/aiforge/buildinforge --gz ./common/aiforge/buildinforge.tar.gz --no-embed

# ---------- 3. 编译 ----------
# -tags gzip_embed        ：编入 //go:build gzip_embed 的内置资源
# -ldflags "-s -w ..."     ：裁剪调试信息、注入版本元数据
# 版本串必须形如 dev-<短hash>（对齐 CI push-arm-build.yml）：YakVersionAtLeast 版本门槛
# 只放行空串 / dev 前缀，注入 unknown 或纯 semver 都会进 VersionCompare 翻车（panic→「引擎版本过低」）
GIT_HASH="$(git show -s --format=%H 2>/dev/null || echo unknown)"
GIT_TIME="$(git show -s --format=%cd 2>/dev/null || echo unknown)"
YAK_TAG="$(git describe --tag 2>/dev/null || true)"
case "$YAK_TAG" in
  dev-*) ;;  # 已是 dev 前缀则原样使用
  *) YAK_TAG="dev-$(git rev-parse --short HEAD 2>/dev/null || echo unknown)" ;;
esac
GO_VER="$(go version)"

echo "[build] building $OUTPUT (linux/arm64, host: $HOST_ARCH)..."
go build -tags gzip_embed \
  -ldflags "-s -w \
    -X 'main.goVersion=${GO_VER}' \
    -X 'main.gitHash=${GIT_HASH}' \
    -X 'main.buildTime=${GIT_TIME}' \
    -X 'main.yakVersion=${YAK_TAG}'" \
  -o "$OUTPUT" \
  common/yak/cmd/yak.go

echo "[build] done -> $REPO_ROOT/$OUTPUT"
