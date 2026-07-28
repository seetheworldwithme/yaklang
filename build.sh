#!/usr/bin/env bash
# 编译 yaklang 为 Windows amd64 的 yak.exe
# 用法：在仓库根目录执行 ./build.sh
#
# 产物：./yak.exe
# 依赖：需要 Go 工具链；首次构建若缺包可先 `go mod download`
set -e

# 切到仓库根目录（脚本可能被从任意位置调用）
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_ROOT"

# ---------- 目标平台 ----------
# 注意：yaklang 依赖 go-pcre2-lite，需要 CGO（PCRE2 是 C 库），不能设 CGO_ENABLED=0。
# 本机为 Windows + MinGW 时这是“原生编译”，CGO 用本机 gcc 即可。
# 若以后要从 Linux/macOS 交叉编译到 Windows，需安装 mingw-w64 并设置 CC=x86_64-w64-mingw32-gcc。
export CGO_ENABLED=1
export GOOS=windows
export GOARCH=amd64

OUTPUT="yak.exe"

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
# git describe --tag 在没有 tag 时会失败，做一层兜底
GIT_HASH="$(git show -s --format=%H 2>/dev/null || echo unknown)"
GIT_TIME="$(git show -s --format=%cd 2>/dev/null || echo unknown)"
YAK_TAG="$(git describe --tag 2>/dev/null || echo unknown)"
GO_VER="$(go version)"

echo "[build] building $OUTPUT (windows/amd64)..."
go build -tags gzip_embed \
  -ldflags "-s -w \
    -X 'main.goVersion=${GO_VER}' \
    -X 'main.gitHash=${GIT_HASH}' \
    -X 'main.buildTime=${GIT_TIME}' \
    -X 'main.yakVersion=${YAK_TAG}'" \
  -o "$OUTPUT" \
  common/yak/cmd/yak.go

echo "[build] done -> $REPO_ROOT/$OUTPUT"
