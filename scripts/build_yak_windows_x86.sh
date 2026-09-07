#!/usr/bin/env bash
# 在 macOS/Linux 上交叉编译 Windows 386 (x86) 的 yak。
# Windows 目标需要 CGO（sqlite 等），请先安装：
#   macOS: brew install mingw-w64
# 用法：在仓库根目录执行 ./scripts/build_yak_windows_x86.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

MINGW_GCC="${MINGW_GCC:-i686-w64-mingw32-gcc}"
MINGW_GXX="${MINGW_GXX:-i686-w64-mingw32-g++}"
if ! command -v "$MINGW_GCC" &>/dev/null; then
  echo "错误：未找到 $MINGW_GCC。macOS 请先执行: brew install mingw-w64" >&2
  exit 1
fi

mkdir -p "$REPO_ROOT/build"
GZIP_EMBED="$REPO_ROOT/build/gzip-embed"
OUT="${OUT:-$REPO_ROOT/build/yak_windows_386.exe}"

echo "[win-x86-build] repos-tag..."
go run common/yak/cmd/yak.go repos-tag -o tags.txt
YAK_TAG="$(cat tags.txt)"
echo "[win-x86-build] YAK_TAG=$YAK_TAG"

echo "[win-x86-build] generate yakdoc / codec embed blobs..."
go run -gcflags=all="-N -l" common/yak/yakdoc/generate_doc/generate_doc.go common/yak/yakdoc/doc/doc.gob.gzip
go run -gcflags=all="-N -l" common/yak/yaklib/codec/codegrpc/doc/codec_doc.go common/yak/yaklib/codec/codegrpc/codec.gob.gzip

echo "[win-x86-build] embed-fs-hash..."
go run common/yak/cmd/yak.go embed-fs-hash --override --all

echo "[win-x86-build] gzip-embed tool..."
go build -o "$GZIP_EMBED" ./common/utils/gzip_embed/gzip-embed

echo "[win-x86-build] gzip embed resources..."
"$GZIP_EMBED" -cache --source ./common/ai/aid/aitool/buildinaitools/yakscripttools/yakscriptforai --gz ./common/ai/aid/aitool/buildinaitools/yakscripttools/yakscriptforai.tar.gz --no-embed
"$GZIP_EMBED" -cache --source ./common/ai/aid/aireact/skills --gz ./common/ai/aid/aireact/skills.tar.gz --root-path --no-embed
"$GZIP_EMBED" -cache --source ./common/coreplugin/base-yak-plugin --gz ./common/coreplugin/base-yak-plugin.tar.gz --root-path --no-embed
"$GZIP_EMBED" -cache --source ./common/syntaxflow/sfbuildin/buildin --gz ./common/syntaxflow/sfbuildin/buildin.tar.gz --root-path --no-embed
"$GZIP_EMBED" -cache --source ./common/aiforge/buildinforge --gz ./common/aiforge/buildinforge.tar.gz --root-path --no-embed

echo "[win-x86-build] GOOS=windows GOARCH=386 CGO + mingw..."
export CGO_ENABLED=1
export GOOS=windows
export GOARCH=386
export CC="$MINGW_GCC"
export CXX="$MINGW_GXX"

go build -tags gzip_embed \
  -ldflags "-s -w -X 'main.goVersion=$(go version)' -X 'main.gitHash=$(git show -s --format=%H)' -X 'main.buildTime=$(git show -s --format=%cd)' -X 'main.yakVersion=${YAK_TAG}'" \
  -o "$OUT" -v common/yak/cmd/yak.go

echo "[win-x86-build] done: $OUT"
ls -lh "$OUT"
