#!/usr/bin/env bash
# 编译 yaklang 为 Windows amd64 的 yak 引擎（普通版，Win10+；Win7 兼容请用 build-windows-legacy-amd64.sh）
# 用法：在仓库根目录执行 ./build-windows-amd64.sh
#
# 产物：./yak_windows_amd64.exe（+ .sha256.txt）
#
# 与 build.sh 的差异：
#   1. 支持 Linux/macOS 主机交叉编译（自动设置 mingw-w64 的 CC/CXX，build.sh 需手动 export）
#   2. 版本串强制形如 dev-<短hash>：YakitVersionAtLeast 版本门槛只放行空串 / dev 前缀，
#      build.sh 在无 tag 仓库里会注入 unknown，导致 Yakit 判定引擎版本过低
#   3. 产物按 CI 命名规则 yak_windows_amd64.exe，可直接放入 yakit/bins/
#
# 依赖：go-pcre2-lite / go-sqlite3 需要 CGO，Linux/macOS 主机需 mingw-w64
#   Debian/Ubuntu: sudo apt install gcc-mingw-w64-x86-64
#   macOS:         brew install mingw-w64
set -e

# 切到仓库根目录（脚本可能被从任意位置调用）
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_ROOT"

# ---------- 主机环境 ----------
HOST_OS_RAW="$(uname -s)"
case "$HOST_OS_RAW" in
  Linux*) HOST_KIND=linux ;;
  Darwin*) HOST_KIND=darwin ;;
  MINGW*|MSYS*|CYGWIN*) HOST_KIND=windows ;;
  *) echo "[build] ERROR: 不支持的主机系统：$HOST_OS_RAW" >&2; exit 1 ;;
esac

HOST_ARCH="$(uname -m)"
case "$HOST_ARCH" in
  x86_64|amd64) HOST_ARCH=amd64 ;;
  aarch64|arm64) HOST_ARCH=arm64 ;;
  *) echo "[build] ERROR: 不支持的主机架构：$HOST_ARCH" >&2; exit 1 ;;
esac

# ---------- CGO 编译器 ----------
# 目标固定 windows/amd64。Linux/macOS 为交叉编译，必须用 mingw-w64 专用前缀 gcc；
# Windows 主机为原生编译，回落到普通 gcc（WinLibs/MSYS2 均为 x86_64 目标）
if command -v x86_64-w64-mingw32-gcc &>/dev/null; then
  export CC=x86_64-w64-mingw32-gcc
  export CXX=x86_64-w64-mingw32-g++
elif [ "$HOST_KIND" = "windows" ] && command -v gcc &>/dev/null; then
  export CC=gcc
  export CXX=g++
else
  echo "[build] ERROR: 未找到 x86_64-w64-mingw32-gcc（CGO 交叉编译必需）" >&2
  echo "  Debian/Ubuntu: sudo apt install gcc-mingw-w64-x86-64" >&2
  echo "  macOS:         brew install mingw-w64" >&2
  exit 1
fi

command -v go &>/dev/null || { echo "[build] ERROR: 缺少 go 工具链" >&2; exit 1; }

# ---------- 1. 确保 gzip-embed 工具存在（宿主平台二进制，与目标 GOOS/GOARCH 无关）----------
GOBIN_DIR="${GOBIN_DIR:-$HOME/.cache/yaklang-build/bin}"
mkdir -p "$GOBIN_DIR"
if [ ! -x "$GOBIN_DIR/gzip-embed" ]; then
  echo "[build] gzip-embed not found, installing from repo..."
  # 工具是宿主平台二进制：必须用宿主 gcc 编译。此刻 CC 已被指向 mingw，不能直接用——
  #   mingw 编宿主 linux 代码 → runtime/cgo 找不到 grp.h；
  #   也不能 CGO_ENABLED=0 —— 依赖树里有必须 cgo 的 go-pcre2-lite，关掉后 lib 包退化为空
  GOBIN="$GOBIN_DIR" CC="${HOST_CC:-gcc}" CXX="${HOST_CXX:-g++}" go install ./common/utils/gzip_embed/gzip-embed
fi

# ---------- 2. 生成 gzip_embed 所需的 .tar.gz 资源 ----------
echo "[build] generating gzip embed resources..."
"$GOBIN_DIR/gzip-embed" -cache --source ./common/ai/aid/aitool/buildinaitools/yakscripttools/yakscriptforai --gz ./common/ai/aid/aitool/buildinaitools/yakscriptforai.tar.gz --no-embed
"$GOBIN_DIR/gzip-embed" -cache --source ./common/ai/aid/aireact/skills --gz ./common/ai/aid/aireact/skills.tar.gz --root-path --no-embed
"$GOBIN_DIR/gzip-embed" -cache --source ./common/coreplugin/base-yak-plugin --gz ./common/coreplugin/base-yak-plugin.tar.gz --root-path --no-embed
"$GOBIN_DIR/gzip-embed" -cache --source ./common/syntaxflow/sfbuildin/buildin --gz ./common/syntaxflow/sfbuildin/buildin.tar.gz --no-embed
"$GOBIN_DIR/gzip-embed" -cache --source ./common/aiforge/buildinforge --gz ./common/aiforge/buildinforge.tar.gz --root-path --no-embed

# ---------- 3. 编译 ----------
# 版本串必须形如 dev-<短hash>（对齐 build-windows-legacy-amd64.sh）：
# YakVersionAtLeast 版本门槛只放行空串 / dev 前缀
GIT_HASH="$(git show -s --format=%H 2>/dev/null || echo unknown)"
GIT_TIME="$(git show -s --format=%cd 2>/dev/null || echo unknown)"
YAK_TAG="$(git describe --tag 2>/dev/null || true)"
case "$YAK_TAG" in
  dev-*) ;;
  *) YAK_TAG="dev-$(git rev-parse --short HEAD 2>/dev/null || echo unknown)" ;;
esac
GO_VER="$(go version)"

export CGO_ENABLED=1
export GOOS=windows
export GOARCH=amd64
OUTPUT="yak_windows_amd64.exe"

echo "[build] building $OUTPUT (windows/amd64, host: ${HOST_KIND}/${HOST_ARCH}, go: ${GO_VER}, CC: $CC)..."
go build -tags gzip_embed \
  -ldflags "-s -w \
    -X 'main.goVersion=${GO_VER}' \
    -X 'main.gitHash=${GIT_HASH}' \
    -X 'main.buildTime=${GIT_TIME}' \
    -X 'main.yakVersion=${YAK_TAG}'" \
  -o "$OUTPUT" \
  common/yak/cmd/yak.go

# 校验和文件与 CI 发布产物保持同名
sha256sum "$OUTPUT" > "${OUTPUT}.sha256.txt"

echo "[build] done -> $REPO_ROOT/$OUTPUT"
ls -lh "$OUTPUT" "${OUTPUT}.sha256.txt"
