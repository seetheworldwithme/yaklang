#!/usr/bin/env bash
# 编译 yaklang 为 Linux arm64 (aarch64) 的可执行文件
# 用法：在仓库根目录执行 ./build-linux-arm.sh
#
# 产物：./yak_linux_arm64
#
# ⚠️ 本脚本在 manylinux2014 容器内构建（glibc 2.17 基线），与 CI push-arm-build.yml 的
#    linux-arm64 job 完全对齐。不要改回宿主机裸编：
#    GLIBC 符号版本在「链接时」绑定构建环境，在宿主机（如 Ubuntu 22.04, glibc 2.35）上
#    链接的产物会引用 GLIBC_2.33+/2.34+ 符号，在麒麟/统信/CentOS 等老 glibc 系统上报
#    "version GLIBC_2.xx not found"；glibc 2.17 容器内链接的产物新老系统通吃（向后兼容）。
#
# 依赖：
#   - docker（镜像 quay.io/pypa/manylinux2014_aarch64，首次自动拉取）
#   - 宿主机 Go 工具链（仅用于 go mod download 与生成 gzip-embed 资源，与目标平台无关）
#   - x86_64 宿主机跑 arm64 镜像需 qemu 用户态模拟（apt install qemu-user-static），很慢，
#     建议直接在 arm64 机器上构建
#
# 可用环境变量覆盖：
#   MANYLINUX_IMAGE  容器镜像（默认 quay.io/pypa/manylinux2014_aarch64）
#   GO_MIRROR        Go 工具链下载镜像（默认阿里云，容器内下载用）
set -euo pipefail

# 切到仓库根目录（脚本可能被从任意位置调用）
REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_ROOT"

MANYLINUX_IMAGE="${MANYLINUX_IMAGE:-quay.io/pypa/manylinux2014_aarch64}"
GO_MIRROR="${GO_MIRROR:-https://mirrors.aliyun.com/golang}"
OUTPUT="yak_linux_arm64"

# ---------- 0. 环境检查 ----------
command -v docker &>/dev/null || { echo "[build] ERROR: 需要 docker" >&2; exit 1; }
docker info &>/dev/null || { echo "[build] ERROR: docker 守护进程不可用（未启动或当前用户无权限）" >&2; exit 1; }

HOST_ARCH="$(uname -m)"
if [ "$HOST_ARCH" != "aarch64" ] && [ "$HOST_ARCH" != "arm64" ]; then
  echo "[build] WARN: 宿主机为 $HOST_ARCH，arm64 镜像将走 qemu 模拟，构建会很慢" >&2
fi

GO_MOD_VER="$(awk '$1 == "go" { print $2; exit }' go.mod)"
[ -n "$GO_MOD_VER" ] || { echo "[build] ERROR: 无法从 go.mod 解析 Go 版本" >&2; exit 1; }

# ---------- 1. 宿主机准备模块缓存 ----------
# 容器内 GOMODCACHE 以只读方式挂载宿主机缓存，必须先在宿主机下载完依赖
echo "[build] go mod download (host)..."
go mod download

# ---------- 2. 生成 gzip_embed 所需的 .tar.gz 资源 ----------
# gzip-embed 按宿主机平台编译，与目标 GOOS/GOARCH 无关；
# 容器内 GOMODCACHE 只读，资源必须先在宿主机生成（清单与 CI push-arm-build.yml 一致）
GZ_BIN="gzip-embed"
if ! command -v "$GZ_BIN" &>/dev/null; then
  echo "[build] gzip-embed not found, installing from repo..."
  go install ./common/utils/gzip_embed/gzip-embed
  GZ_BIN="$(go env GOPATH)/bin/gzip-embed"
  [ -x "$GZ_BIN" ] || { echo "[build] ERROR: gzip-embed 安装失败" >&2; exit 1; }
fi

echo "[build] generating gzip embed resources..."
"$GZ_BIN" -cache --source ./common/ai/aid/aitool/buildinaitools/yakscripttools/yakscriptforai --gz ./common/ai/aid/aitool/buildinaitools/yakscripttools/yakscriptforai.tar.gz --no-embed
"$GZ_BIN" -cache --source ./common/ai/aid/aireact/skills --gz ./common/ai/aid/aireact/skills.tar.gz --root-path --no-embed
"$GZ_BIN" -cache --source ./common/yso/resources/static --gz ./common/yso/resources/static.tar.gz --root-path --no-embed
"$GZ_BIN" -cache --source ./common/coreplugin/base-yak-plugin --gz ./common/coreplugin/base-yak-plugin.tar.gz --root-path --no-embed
"$GZ_BIN" -cache --source ./common/syntaxflow/sfbuildin/buildin --gz ./common/syntaxflow/sfbuildin/buildin.tar.gz --root-path --no-embed
"$GZ_BIN" -cache --source ./common/aiforge/buildinforge --gz ./common/aiforge/buildinforge.tar.gz --root-path --no-embed

# ---------- 3. 版本元数据 ----------
# 版本串必须形如 dev-<短hash>（对齐 CI push-arm-build.yml）：YakVersionAtLeast 版本门槛
# 只放行空串 / dev 前缀，注入 unknown 或纯 semver 都会进 VersionCompare 翻车（panic→「引擎版本过低」）
GIT_HASH="$(git show -s --format=%H 2>/dev/null || echo unknown)"
GIT_TIME="$(git show -s --format=%cd 2>/dev/null || echo unknown)"
YAK_TAG="$(git describe --tag 2>/dev/null || true)"
case "$YAK_TAG" in
  dev-*) ;;  # 已是 dev 前缀则原样使用
  *) YAK_TAG="dev-$(git rev-parse --short HEAD 2>/dev/null || echo unknown)" ;;
esac

# ---------- 4. manylinux2014 容器内构建 ----------
# 容器配置与 .github/scripts/build-linux-manylinux.sh 一致（devtoolset-10 + go.mod 版本的 Go），
# 额外用持久化 named volume 缓存，避免每次重下 Go / 全量重编：
#   goroot 卷  -> /usr/local/go            （go.mod 版本变化时自动重下）
#   gocache 卷 -> /root/.cache/go-build    （gcc10/cgo 编译缓存）
GOROOT_VOL="yaklang-manylinux-goroot-arm64"
GOCACHE_VOL="yaklang-manylinux-gocache-arm64"
GOMODCACHE_HOST="$(go env GOMODCACHE)"

echo "[build] building $OUTPUT (linux/arm64, manylinux2014 glibc-2.17 baseline, host: $HOST_ARCH)..."
docker run --rm -i --platform linux/arm64 \
  -v "$REPO_ROOT:/work" -w /work \
  -v "$GOMODCACHE_HOST:/gomodcache:ro" \
  -v "$GOROOT_VOL:/usr/local/go" \
  -v "$GOCACHE_VOL:/root/.cache/go-build" \
  -e GOMODCACHE=/gomodcache \
  -e GOPATH=/tmp/gopath \
  -e CGO_ENABLED=1 \
  -e BUILD_TAGS=gzip_embed \
  -e OUTPUT_BINARY="$OUTPUT" \
  -e YAK_TAG="$YAK_TAG" \
  -e GIT_HASH="$GIT_HASH" \
  -e GIT_TIME="$GIT_TIME" \
  -e GO_MOD_VER="$GO_MOD_VER" \
  -e GO_MIRROR="$GO_MIRROR" \
  -e HOST_UID="$(id -u)" \
  -e HOST_GID="$(id -g)" \
  "$MANYLINUX_IMAGE" \
  bash -leo pipefail -s <<'EOS'
source /opt/rh/devtoolset-10/enable
export PATH=/usr/local/go/bin:$PATH
export GOMODCACHE=/gomodcache
export GOPATH=/tmp/gopath
export CGO_ENABLED=1

# go.mod 指定版本的 Go 装进挂载卷；版本一致时跳过下载（卷持久化，仅首次/升版时下载）
# 注意 /usr/local/go 是卷挂载点，不能整目录 rm，只能清空内容
if [ ! -x /usr/local/go/bin/go ] || [ "$(go version 2>/dev/null | awk '{print $3}')" != "go${GO_MOD_VER}" ]; then
  echo "[container] downloading go${GO_MOD_VER} from ${GO_MIRROR}..."
  find /usr/local/go -mindepth 1 -delete 2>/dev/null || true
  curl -fsSL "${GO_MIRROR}/go${GO_MOD_VER}.linux-arm64.tar.gz" | tar -C /usr/local -xz
fi

GO_VER_STR="$(go version)"
go build -trimpath -tags "${BUILD_TAGS}" \
  -ldflags "-s -w -X main.goVersion=${GO_VER_STR} -X main.gitHash=${GIT_HASH} -X main.buildTime=${GIT_TIME} -X main.yakVersion=${YAK_TAG}" \
  -o "./${OUTPUT_BINARY}" -v common/yak/cmd/yak.go
ls -lh "./${OUTPUT_BINARY}"

# 容器内是 root，产物归还给宿主机调用者，避免 root 属主文件
chown "${HOST_UID}:${HOST_GID}" "./${OUTPUT_BINARY}" 2>/dev/null || true
EOS

# ---------- 5. 校验 glibc 符号基线 ----------
# 正常应 ≤ GLIBC_2.17；若超出说明没走 manylinux 基线（比如换了镜像），产物在老系统会报
# "version GLIBC_2.xx not found"，直接失败避免带病出厂
if command -v readelf &>/dev/null; then
  MAX_GLIBC="$(readelf -V "$OUTPUT" 2>/dev/null | grep -o 'GLIBC_[0-9.]*' | sort -Vu | tail -1 || true)"
  if [ -n "$MAX_GLIBC" ] && [ "$(printf '%s\nGLIBC_2.17\n' "$MAX_GLIBC" | sort -V | tail -1)" != "GLIBC_2.17" ]; then
    echo "[build] ERROR: 产物要求 ${MAX_GLIBC}，超出 manylinux2014 基线（GLIBC_2.17），老系统将无法运行" >&2
    exit 1
  fi
  [ -n "$MAX_GLIBC" ] && echo "[build] glibc 符号基线: $MAX_GLIBC (OK, ≤ 2.17)"
fi

echo "[build] done -> $REPO_ROOT/$OUTPUT"
