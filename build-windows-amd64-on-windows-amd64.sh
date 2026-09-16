#!/usr/bin/env bash
# 在 Windows amd64 主机上原生编译 Windows amd64 的 yak.exe
# 用法：在仓库根目录执行 ./build-windows-amd64-on-windows-amd64.sh
#
# 产物：./yak.exe
# 依赖：Go 工具链 + MinGW-w64 gcc（CGO 需要，PCRE2/SQLite 是 C 库）
set -e

# 切到仓库根目录（脚本可能被从任意位置调用）
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_ROOT"

# ---------- 主机/目标平台校验 ----------
# 本脚本仅用于 Windows amd64 主机原生编译（Go 对 GOOS=windows 可原生自编译，
# 但 uname -m 只能区分 CPU 架构，OS 是否为 Windows 由 SHELL/OS 判断兜底）
HOST_ARCH="$(uname -m)"
if [ "$HOST_ARCH" != "x86_64" ] && [ "$HOST_ARCH" != "amd64" ]; then
  echo "[build] ERROR: 本脚本仅支持 amd64 主机（当前: $HOST_ARCH），请改用对应构建脚本" >&2
  exit 1
fi

# ---------- 目标平台 ----------
# 注意：yaklang 依赖 go-pcre2-lite / go-sqlite3，需要 CGO（PCRE2、SQLite 都是 C 库），
#       不能设 CGO_ENABLED=0。本机 MinGW gcc 即是"原生编译"，无需交叉工具链。
export CGO_ENABLED=1
export GOOS=windows
export GOARCH=amd64
OUTPUT="yak.exe"

# ---------- CGO 编译器检查 ----------
# 优先使用 mingw-w64 专用前缀的 gcc，回落到普通 gcc（WinLibs/MSYS2 等发行版均为 x86_64 目标）
if command -v x86_64-w64-mingw32-gcc &>/dev/null; then
  export CC=x86_64-w64-mingw32-gcc
  export CXX=x86_64-w64-mingw32-g++
elif command -v gcc &>/dev/null; then
  export CC=gcc
  export CXX=g++
else
  echo "[build] ERROR: 未找到 gcc，请安装 MinGW-w64（如 winget install BrechtSanders.WinLibs.POSIX.UCRT）" >&2
  exit 1
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
# 版本串必须形如 dev-<短hash>（对齐 build-linux-arm.sh / CI push-arm-build.yml）：
# YakVersionAtLeast 版本门槛只放行空串 / dev 前缀，注入 unknown 或纯 semver
# 会进 VersionCompare 翻车（panic→「引擎版本过低」）
GIT_HASH="$(git show -s --format=%H 2>/dev/null || echo unknown)"
GIT_TIME="$(git show -s --format=%cd 2>/dev/null || echo unknown)"
YAK_TAG="$(git describe --tag 2>/dev/null || true)"
case "$YAK_TAG" in
  dev-*) ;;  # 已是 dev 前缀则原样使用
  *) YAK_TAG="dev-$(git rev-parse --short HEAD 2>/dev/null || echo unknown)" ;;
esac
GO_VER="$(go version)"

echo "[build] building $OUTPUT (windows/amd64, host: $HOST_ARCH, CC: $CC)..."
go build -tags gzip_embed \
  -ldflags "-s -w \
    -X 'main.goVersion=${GO_VER}' \
    -X 'main.gitHash=${GIT_HASH}' \
    -X 'main.buildTime=${GIT_TIME}' \
    -X 'main.yakVersion=${YAK_TAG}'" \
  -o "$OUTPUT" \
  common/yak/cmd/yak.go

echo "[build] done -> $REPO_ROOT/$OUTPUT"
