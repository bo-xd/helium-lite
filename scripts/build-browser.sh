#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

CHROMIUM_VER="158.0.8127.36"
UC_TAG="158.0.8127.36-1"
CRUBIT_REV="d757594f6f7ac27c0cf9c8ea63d9136f8e1f77ae"

BUILD_DIR="/home/bo/Projects/kestrel-build"
JOBS=10
CHECK_DEPS=0
FETCH_ONLY=0

RUNTIME_DIR="${HOME}/.local/share/kestrel/runtime"

REQUIRED_CMDS=(
  ninja gn clang lld llvm-ar node tsc esbuild bindgen java gperf bison flex quilt git curl tar xz
)

REQUIRED_ARCH_PKGS=(
  compiler-rt rust-src
)

PACMAN_PKGS=(
  ninja gn clang lld llvm compiler-rt nodejs gperf bison flex quilt git curl tar xz
  esbuild typescript rust rust-bindgen rust-src jdk-headless go
)

usage() {
  cat <<'EOF'
Usage: build-browser.sh [OPTIONS]

Build Kestrel's custom Ungoogled Chromium (158.0.8127.36-1) from source on Arch Linux.

Options:
  --check-deps      Check required build tools and exit (prints pacman command if missing)
  --fetch-only      Download, unpack, and patch source without compiling
  --jobs N          Parallel ninja jobs (default: 10)
  --build-dir DIR   Build root under /home (default: /home/bo/Projects/kestrel-build)
  -h, --help        Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check-deps) CHECK_DEPS=1; shift ;;
    --fetch-only) FETCH_ONLY=1; shift ;;
    --jobs) JOBS="$2"; shift 2 ;;
    --build-dir) BUILD_DIR="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

check_dependencies() {
  local missing=()
  for cmd in "${REQUIRED_CMDS[@]}"; do
    if ! command -v "${cmd}" >/dev/null 2>&1; then
      missing+=("${cmd}")
    fi
  done

  if command -v pacman >/dev/null 2>&1; then
    for pkg in "${REQUIRED_ARCH_PKGS[@]}"; do
      if ! pacman -Q "${pkg}" >/dev/null 2>&1; then
        missing+=("${pkg}")
      fi
    done
  fi

  if [[ ${#missing[@]} -eq 0 ]]; then
    echo "All required build tools are installed."
    return 0
  fi

  echo "Missing build tools: ${missing[*]}"
  echo
  echo "Install required Arch Linux build dependencies with:"
  echo "  sudo pacman -S --needed ${PACMAN_PKGS[*]}"
  return 1
}

if [[ "${CHECK_DEPS}" -eq 1 ]]; then
  check_dependencies
  exit $?
fi

if ! check_dependencies; then
  exit 1
fi

if [[ "${BUILD_DIR}" == /tmp* ]]; then
  echo "Error: --build-dir must not be under /tmp (Chromium build needs ~100GB+)." >&2
  exit 1
fi

mkdir -p "${BUILD_DIR}/tmp"
export TMPDIR="${BUILD_DIR}/tmp"

UC_DIR="${BUILD_DIR}/ungoogled-chromium-${UC_TAG}"
ARCH_DIR="${BUILD_DIR}/ungoogled-chromium-archlinux"
CRUBIT_DIR="${BUILD_DIR}/crubit"
SRC_TARBALL="${BUILD_DIR}/chromium-${CHROMIUM_VER}-lite.tar.xz"
SRC_DIR="${BUILD_DIR}/chromium-${CHROMIUM_VER}"

if [[ ! -d "${UC_DIR}" ]]; then
  echo "Cloning ungoogled-chromium (${UC_TAG})..."
  git clone --depth 1 --branch "${UC_TAG}" \
    https://github.com/ungoogled-software/ungoogled-chromium.git "${UC_DIR}"
fi

if [[ ! -d "${ARCH_DIR}" ]]; then
  echo "Cloning ungoogled-chromium-archlinux..."
  git clone --depth 1 \
    https://github.com/ungoogled-software/ungoogled-chromium-archlinux.git "${ARCH_DIR}"
fi

if [[ ! -d "${CRUBIT_DIR}" ]]; then
  echo "Cloning crubit (${CRUBIT_REV})..."
  git clone https://chromium.googlesource.com/external/github.com/google/crubit.git "${CRUBIT_DIR}"
  git -C "${CRUBIT_DIR}" checkout "${CRUBIT_REV}"
fi

if [[ ! -f "${SRC_TARBALL}" ]]; then
  echo "Downloading Chromium ${CHROMIUM_VER}-lite source tarball..."
  curl -fL --progress-bar -o "${SRC_TARBALL}" \
    "https://commondatastorage.googleapis.com/chromium-browser-official/chromium-${CHROMIUM_VER}-lite.tar.xz"
fi

if [[ ! -d "${SRC_DIR}" ]]; then
  echo "Unpacking Chromium ${CHROMIUM_VER} to ${SRC_DIR}..."
  tar -xJf "${SRC_TARBALL}" -C "${BUILD_DIR}"
fi

if [[ ! -f "${SRC_DIR}/.kestrel_patched" ]]; then
  echo "Applying Arch Linux & ungoogled-chromium patches..."
  pushd "${SRC_DIR}" >/dev/null

  sed -i 's/OFFICIAL_BUILD/GOOGLE_CHROME_BUILD/' \
    tools/generate_shim_headers/generate_shim_headers.py

  sed -i -e 's/\<xmlMalloc\>/malloc/' -e 's/\<xmlFree\>/free/' \
         -e '1i #include <cstdlib>' \
    third_party/blink/renderer/core/xml/*.cc \
    third_party/blink/renderer/core/xml/parser/xml_document_parser.cc \
    third_party/libxml/chromium/*.cc

  for p in \
    use-oauth2-client-switches-as-default.patch \
    chromium-138-nodejs-version-check.patch \
    compiler-rt-adjust-paths.patch \
    increase-fortify-level.patch \
    chromium-145-fix-SYS_SECCOMP.patch \
    chromium-147-rust-1.95-bytemuck.patch \
    enable-widevine-arm64.patch \
    glibc-2.42-baud-rate-fix.patch \
    chromium-150-revert-avx-flag-change.patch \
    chromium-147-simdutf-8.0-base-char.patch \
    chromium-152-unbundle-minizip-undo-unicode.patch \
    chromium-152-unbundle-opus-devtools.patch \
    chromium-153-hermetic-python.patch \
    chromium-153-iamf-tools-unbundled-opus.patch \
    chromium-155-use-system-esbuild.patch \
    chromium-158-isolatedDeclarations.patch \
    chromium-157-typescript.patch \
    chromium-158-revert-gn-check-includes.patch; do
    if [[ -f "${ARCH_DIR}/${p}" ]]; then
      patch -Np1 -i "${ARCH_DIR}/${p}"
    fi
  done

  sed -i '/brotli:headers/ d' third_party/pdfium/core/fxcodec/BUILD.gn
  rm -rf third_party/devtools-frontend/src/node_modules/esbuild/
  ln -sfn /usr/lib/node_modules/esbuild/ third_party/devtools-frontend/src/node_modules/esbuild

  python3 "${UC_DIR}/utils/prune_binaries.py" ./ "${UC_DIR}/pruning.list" || true
  python3 "${UC_DIR}/utils/patches.py" apply ./ "${UC_DIR}/patches"
  python3 "${UC_DIR}/utils/domain_substitution.py" apply \
    -r "${UC_DIR}/domain_regex.list" \
    -f "${UC_DIR}/domain_substitution.list" \
    -c domainsubcache.tar.gz ./

  shopt -s nullglob
  for kp in "${REPO_ROOT}/browser/patches/"*.patch; do
    echo "Applying custom Kestrel patch: $(basename "${kp}")"
    patch -Np1 -i "${kp}"
  done
  shopt -u nullglob

  mkdir -p third_party/node/linux/node-linux-x64/bin \
           third_party/rust-toolchain/bin \
           third_party/jdk/current/bin \
           third_party/gperf/cipd/bin \
           third_party/dawn/tools/golang/linux-amd64/bin \
           third_party/rust-toolchain/crubit/lib/third_party/

  ln -sfn /usr/bin/node third_party/node/linux/node-linux-x64/bin/node
  ln -sfn /usr/bin/java third_party/jdk/current/bin/java
  ln -sfn /usr/bin/rustc third_party/rust-toolchain/bin/rustc
  ln -sfn /usr/bin/gperf third_party/gperf/cipd/bin/gperf
  ln -sfn /usr/bin/go third_party/dawn/tools/golang/linux-amd64/bin/go
  ln -sfn "${CRUBIT_DIR}" third_party/rust-toolchain/crubit/lib/third_party/crubit

  touch .kestrel_patched
  popd >/dev/null
fi

if [[ "${FETCH_ONLY}" -eq 1 ]]; then
  echo "Fetch and patch complete at ${SRC_DIR}."
  exit 0
fi

echo "Configuring out/Kestrel with browser/args.gn..."
pushd "${SRC_DIR}" >/dev/null

export CC=clang
export CXX=clang++
export AR=llvm-ar
export NM=nm
export RUSTC_BOOTSTRAP=1

CLANG_VER="$(clang --version | grep -m1 version | sed 's/.* \([0-9]\+\).*/\1/')"
RUSTC_VER="$(rustc --version | awk '{print $2}')"
SITE_PKGS="$(python3 -c 'import site; print(site.getsitepackages()[0])')"

mkdir -p out/Kestrel
cp "${REPO_ROOT}/browser/args.gn" out/Kestrel/args.gn
cat >> out/Kestrel/args.gn <<EOF
clang_version="${CLANG_VER}"
v8_metagen_libclang_so="/usr/lib/libclang.so"
v8_metagen_libclang_bindings_dir="${SITE_PKGS}"
rust_sysroot_absolute="/usr"
rust_bindgen_root="/usr"
rustc_version="${RUSTC_VER}"
rust_crubit_root="//third_party/rust-toolchain/crubit/"
added_rust_stdlib_libs=["addr2line","adler2","gimli","miniz_oxide"]
EOF

gn gen out/Kestrel

echo "Compiling Kestrel Chromium runtime (-j ${JOBS})..."
ninja -j "${JOBS}" -C out/Kestrel chrome chrome_sandbox

echo "Installing custom Kestrel runtime to ${RUNTIME_DIR}..."
STAGING="${RUNTIME_DIR}.custom_staging"
rm -rf "${STAGING}"
mkdir -p "${STAGING}"

cp -a out/Kestrel/chrome "${STAGING}/chrome"
cp -a out/Kestrel/chrome_sandbox "${STAGING}/chrome-sandbox"
cp -a out/Kestrel/*.pak out/Kestrel/*.bin out/Kestrel/icudtl.dat "${STAGING}/" 2>/dev/null || true
cp -a out/Kestrel/locales "${STAGING}/" 2>/dev/null || true
cp -a out/Kestrel/lib*.so "${STAGING}/" 2>/dev/null || true
chmod 755 "${STAGING}/chrome" "${STAGING}/chrome-sandbox"

rm -rf "${RUNTIME_DIR}"
mv "${STAGING}" "${RUNTIME_DIR}"

popd >/dev/null
echo "Custom Kestrel Chromium runtime installed at ${RUNTIME_DIR}/chrome"
