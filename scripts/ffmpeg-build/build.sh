#!/usr/bin/env bash
# Builds the FFmpeg that Sub20 ships, for one target, from the source
# archives pinned in sources.json. Only what Sub20 uses goes in: the CPU
# encoders x264, x265 and SVT-AV1, Opus, dav1d for AV1 decoding, zimg for
# HDR tone mapping, zlib, and each system's GPU encoders (NVENC, AMF and
# Quick Sync on Windows and Linux, Media Foundation on Windows, VAAPI on
# Linux, VideoToolbox on macOS).
#
#   scripts/ffmpeg-build/build.sh TARGET OUT
#
# TARGET is a Rust target triple:
#   x86_64-pc-windows-msvc    cross-compiled on Linux with mingw-w64
#   x86_64-unknown-linux-gnu  on Linux (third-party libraries static, glibc shared)
#   aarch64-apple-darwin      on an Apple Silicon Mac (Intel Macs aren't supported)
#
# OUT receives sub20-ffmpeg-<version>-<revision>-<target>.zip (Windows) or
# .tar.xz: ffmpeg, ffprobe, the GPL and BUILD.txt. WORK (default
# ./ffmpeg-work) holds the sources and the build.
#
# This script and sources.json are part of the corresponding source the
# GPL asks for: they are how the binaries were made.

set -euo pipefail

TARGET=${1:?target}
mkdir -p "${2:?out folder}"
OUT=$(cd "$2" && pwd)
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
WORK=${WORK:-$PWD/ffmpeg-work}
mkdir -p "$WORK"
WORK=$(cd "$WORK" && pwd)
SRC=$WORK/sources
BUILD=$WORK/build
PREFIX=$WORK/prefix
rm -rf "$BUILD" "$PREFIX"
mkdir -p "$SRC" "$BUILD" "$PREFIX/lib/pkgconfig" "$PREFIX/include"

JOBS=$(getconf _NPROCESSORS_ONLN 2>/dev/null || sysctl -n hw.ncpu)
VERSION=$(node -p "require('$HERE/sources.json').ffmpeg")
REVISION=$(node -p "require('$HERE/sources.json').revision")

# Only our own libraries are visible to pkg-config, never the system's.
export PKG_CONFIG_LIBDIR="$PREFIX/lib/pkgconfig"
export PKG_CONFIG_PATH=""

case "$TARGET" in
  x86_64-pc-windows-msvc)
    OS=windows
    HOST=x86_64-w64-mingw32
    EXE=.exe
    ;;
  x86_64-unknown-linux-gnu)
    OS=linux
    EXE=
    ;;
  aarch64-apple-darwin)
    OS=mac
    EXE=
    ARCH=${TARGET%%-*}
    [ "$ARCH" = aarch64 ] && ARCH=arm64
    if [ "$(uname -m)" != "$ARCH" ]; then
      echo "Build $TARGET on a Mac of that kind: this one is $(uname -m)." >&2
      exit 1
    fi
    # Matches bundle.macOS.minimumSystemVersion in tauri.release.conf.json.
    export MACOSX_DEPLOYMENT_TARGET=11.0
    ;;
  *)
    echo "Unknown target $TARGET" >&2
    exit 1
    ;;
esac

export CFLAGS="-O2 -I$PREFIX/include"
export CXXFLAGS="-O2 -I$PREFIX/include"
export LDFLAGS="-L$PREFIX/lib"
if [ "$OS" = linux ]; then
  CFLAGS="$CFLAGS -fPIC"
  CXXFLAGS="$CXXFLAGS -fPIC"
  # x265 and zimg are C++. Put the static libstdc++ where -lstdc++ finds it
  # first, so the binaries don't need a libstdc++ as new as the build
  # machine's.
  ln -sf "$(g++ -print-file-name=libstdc++.a)" "$PREFIX/lib/libstdc++.a"
fi

# How each build system is told about the target.
AUTOCONF_HOST=()
CMAKE=(-G Ninja -DCMAKE_BUILD_TYPE=Release "-DCMAKE_INSTALL_PREFIX=$PREFIX" -DCMAKE_INSTALL_LIBDIR=lib
  -DBUILD_SHARED_LIBS=OFF -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DCMAKE_POLICY_VERSION_MINIMUM=3.5)
MESON=(--prefix "$PREFIX" --libdir lib --buildtype release --default-library static)
if [ "$OS" = windows ]; then
  AUTOCONF_HOST=(--host=$HOST)
  CMAKE+=(-DCMAKE_SYSTEM_NAME=Windows -DCMAKE_SYSTEM_PROCESSOR=AMD64
    -DCMAKE_C_COMPILER=$HOST-gcc -DCMAKE_CXX_COMPILER=$HOST-g++ -DCMAKE_RC_COMPILER=$HOST-windres
    -DCMAKE_ASM_NASM_COMPILER=nasm "-DCMAKE_FIND_ROOT_PATH=$PREFIX"
    -DCMAKE_FIND_ROOT_PATH_MODE_PROGRAM=NEVER -DCMAKE_FIND_ROOT_PATH_MODE_LIBRARY=ONLY -DCMAKE_FIND_ROOT_PATH_MODE_INCLUDE=ONLY)
  cat > "$WORK/mingw.meson" <<EOF
[binaries]
c = '$HOST-gcc'
cpp = '$HOST-g++'
ar = '$HOST-ar'
strip = '$HOST-strip'
windres = '$HOST-windres'
pkg-config = 'pkg-config'

[host_machine]
system = 'windows'
cpu_family = 'x86_64'
cpu = 'x86_64'
endian = 'little'
EOF
  MESON+=(--cross-file "$WORK/mingw.meson")
elif [ "$OS" = mac ]; then
  CMAKE+=(-DCMAKE_OSX_ARCHITECTURES=$ARCH -DCMAKE_OSX_DEPLOYMENT_TARGET=$MACOSX_DEPLOYMENT_TARGET)
fi

node "$HERE/sources.mjs" fetch "$SRC" --target "$TARGET"

# Unpacks a pinned source into a fresh folder and enters it.
enter() {
  local file
  file=$(node "$HERE/sources.mjs" list --target "$TARGET" | awk -F'\t' -v n="$1" '$1 == n { print $2 }')
  [ -n "$file" ] || { echo "No source named $1 for $TARGET" >&2; exit 1; }
  rm -rf "${BUILD:?}/$1"
  mkdir -p "$BUILD/$1"
  tar -xf "$SRC/$file" -C "$BUILD/$1" --strip-components=1
  cd "$BUILD/$1"
  echo "::group::$1"
}

done_with() {
  echo "::endgroup::"
  cd "$WORK"
}

# Edits a file in place the same way on GNU and BSD systems.
edit() {
  perl -pi -e "$1" "$2"
}

# --- zlib (the macOS SDK has its own) ---------------------------------------
if [ "$OS" != mac ]; then
  enter zlib
  cmake -S . -B out "${CMAKE[@]}" -DZLIB_BUILD_SHARED=OFF -DZLIB_BUILD_TESTING=OFF
  cmake --build out
  cmake --install out
  # On Windows the static library is called libzs.a; FFmpeg asks for -lz.
  [ -f "$PREFIX/lib/libzs.a" ] && cp "$PREFIX/lib/libzs.a" "$PREFIX/lib/libz.a"
  done_with
fi

# --- x264 ----------------------------------------------------------------------
enter x264
X264=(--prefix="$PREFIX" --enable-static --enable-pic --disable-cli --disable-lavf --disable-swscale
  --disable-ffms --disable-gpac --disable-lsmash)
[ "$OS" = windows ] && X264+=(--host=$HOST --cross-prefix=$HOST-)
./configure "${X264[@]}"
make -j"$JOBS"
make install
done_with

# --- x265 (8-bit: Sub20 always encodes 8-bit) ---------------------------------
enter x265
cmake -S source -B out "${CMAKE[@]}" -DENABLE_SHARED=OFF -DENABLE_CLI=OFF -DENABLE_LIBNUMA=OFF
cmake --build out
cmake --install out
# A static link has no libgcc_s to find.
edit 's/-lgcc_s//g' "$PREFIX/lib/pkgconfig/x265.pc"
done_with

# --- SVT-AV1 -------------------------------------------------------------------
enter svt-av1
cmake -S . -B out "${CMAKE[@]}" -DBUILD_APPS=OFF -DBUILD_DEC=OFF -DBUILD_TESTING=OFF
cmake --build out
cmake --install out
done_with

# --- dav1d ---------------------------------------------------------------------
enter dav1d
meson setup out "${MESON[@]}" -Denable_tools=false -Denable_tests=false
ninja -C out install
done_with

# --- Opus ----------------------------------------------------------------------
enter opus
./configure --prefix="$PREFIX" "${AUTOCONF_HOST[@]+"${AUTOCONF_HOST[@]}"}" --disable-shared --enable-static --with-pic \
  --disable-doc --disable-extra-programs
make -j"$JOBS"
make install
done_with

# --- zimg (zscale, for HDR tone mapping) ---------------------------------------
enter zimg
./autogen.sh
./configure --prefix="$PREFIX" "${AUTOCONF_HOST[@]+"${AUTOCONF_HOST[@]}"}" --disable-shared --enable-static --with-pic
make -j"$JOBS"
make install
done_with

# --- GPU encoder headers and dispatchers (Windows and Linux) ------------------
if [ "$OS" != mac ]; then
  enter nv-codec-headers
  make PREFIX="$PREFIX" install
  done_with

  enter amf
  mkdir -p "$PREFIX/include/AMF"
  cp -R AMF/. "$PREFIX/include/AMF/"
  done_with

  enter libvpl
  # mingw-w64 has no _MSC_VER; the test below breaks the build there.
  edit 's/^#if _MSC_VER < 1400$/#if defined(_MSC_VER) && _MSC_VER < 1400/' libvpl/src/windows/mfx_dispatcher_defs.h
  cmake -S . -B out "${CMAKE[@]}" -DBUILD_DISPATCHER=ON -DBUILD_DEV=ON -DBUILD_PREVIEW=OFF -DBUILD_TOOLS=OFF \
    -DBUILD_TOOLS_ONEVPL_EXPERIMENTAL=OFF -DBUILD_EXAMPLES=OFF -DINSTALL_EXAMPLE_CODE=OFF -DBUILD_TESTS=OFF
  cmake --build out
  cmake --install out
  rm -rf "$PREFIX/etc" "$PREFIX/share/vpl"
  done_with
fi

# --- VAAPI (Linux) -------------------------------------------------------------
if [ "$OS" = linux ]; then
  enter libdrm
  meson setup out "${MESON[@]}" -Dintel=disabled -Dradeon=disabled -Damdgpu=disabled -Dnouveau=disabled \
    -Dvmwgfx=disabled -Domap=disabled -Dexynos=disabled -Dfreedreno=disabled -Dtegra=disabled -Dvc4=disabled \
    -Detnaviv=disabled -Dcairo-tests=disabled -Dman-pages=disabled -Dvalgrind=disabled -Dudev=false -Dtests=false
  ninja -C out install
  done_with

  enter libva
  # libva only offers shared libraries; let meson's default_library decide.
  edit 's/\bshared_library\(/library(/g' va/meson.build
  # Static libva looks for the GPU's driver itself: search where Debian,
  # Ubuntu, Fedora and Arch keep them.
  meson setup out "${MESON[@]}" -Ddriverdir=/usr/lib/x86_64-linux-gnu/dri:/usr/lib64/dri:/usr/lib/dri \
    -Ddisable_drm=false -Dwith_x11=no -Dwith_glx=no -Dwith_wayland=no -Denable_docs=false
  ninja -C out install
  done_with
fi

# --- FFmpeg --------------------------------------------------------------------
enter ffmpeg
CONFIGURE=(--prefix="$PREFIX/ffmpeg" --pkg-config-flags=--static
  --enable-gpl --enable-version3
  --disable-autodetect --disable-debug --disable-doc --disable-ffplay --disable-network
  --disable-shared --enable-static
  --enable-libx264 --enable-libx265 --enable-libsvtav1 --enable-libdav1d --enable-libopus --enable-libzimg
  --enable-zlib
  "--extra-cflags=-I$PREFIX/include" "--extra-ldflags=-L$PREFIX/lib")
case "$OS" in
  windows)
    # Plain pkg-config: the cross prefix would make it look for
    # x86_64-w64-mingw32-pkg-config, which isn't there.
    CONFIGURE+=(--target-os=mingw32 --arch=x86_64 --cross-prefix=$HOST- --pkg-config=pkg-config
      --enable-ffnvcodec --enable-cuda --enable-nvenc --enable-nvdec --enable-amf --enable-libvpl
      --enable-mediafoundation --enable-d3d11va --enable-dxva2
      --extra-ldflags=-static "--extra-libs=-lstdc++")
    ;;
  linux)
    CONFIGURE+=(--enable-ffnvcodec --enable-cuda --enable-nvenc --enable-nvdec --enable-amf --enable-libvpl
      --enable-vaapi --enable-libdrm
      "--extra-libs=-lstdc++ -lm -ldl -lpthread")
    ;;
  mac)
    CONFIGURE+=(--enable-videotoolbox --enable-audiotoolbox
      "--extra-cflags=-mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET"
      "--extra-ldflags=-mmacosx-version-min=$MACOSX_DEPLOYMENT_TARGET" "--extra-libs=-lc++")
    ;;
esac
./configure "${CONFIGURE[@]}" || { tail -40 ffbuild/config.log; exit 1; }
make -j"$JOBS"
make install
done_with

# --- Package -------------------------------------------------------------------
DIST=$WORK/dist
rm -rf "$DIST"
mkdir -p "$DIST"
for tool in ffmpeg ffprobe; do
  cp "$PREFIX/ffmpeg/bin/$tool$EXE" "$DIST/"
done
if [ "$OS" = mac ]; then
  # Apple Silicon runs nothing unsigned; an ad-hoc signature is enough.
  for tool in ffmpeg ffprobe; do codesign --force --sign - "$DIST/$tool"; done
fi

# The binaries may only need what every such system has.
echo "Linked against:"
case "$OS" in
  windows)
    deps=$($HOST-objdump -p "$DIST/ffmpeg.exe" | awk '/DLL Name/ { print $3 }')
    # mingw's runtimes (libwinpthread, libstdc++, libgcc_s) must be linked in.
    bad=$(echo "$deps" | grep -i '^lib' || true)
    ;;
  linux)
    deps=$(ldd "$DIST/ffmpeg" | awk '{ print $1 }')
    bad=$(echo "$deps" | grep -v -E '^(linux-vdso|/lib64/ld-linux-x86-64|libc|libm|libdl|libpthread|librt)\.so' || true)
    ;;
  mac)
    deps=$(otool -L "$DIST/ffmpeg" | tail -n +2 | awk '{ print $1 }')
    bad=$(echo "$deps" | grep -v -E '^(/usr/lib/|/System/Library/)' || true)
    ;;
esac
echo "$deps" | sed 's/^/  /'
if [ -n "$bad" ]; then
  echo "ffmpeg needs libraries a user's system may not have:" >&2
  echo "$bad" >&2
  exit 1
fi
cp "$ROOT/licenses/FFmpeg-GPL-3.0.txt" "$DIST/LICENSE-GPL-3.0.txt"

NAME="sub20-ffmpeg-$VERSION-$REVISION-$TARGET"
{
  echo "FFmpeg $VERSION, built by Sub20 for $TARGET (revision $REVISION)."
  echo
  echo "FFmpeg is free software. This build is licensed under the GNU General"
  echo "Public License version 3 (LICENSE-GPL-3.0.txt). The source code of"
  echo "FFmpeg and of every library built into it, and the scripts that built"
  echo "them, are published with these binaries:"
  echo "https://github.com/Y4dd/Sub20/releases/tag/ffmpeg-$VERSION-$REVISION"
  echo
  echo "Built from:"
  node -e "for (const s of require('$HERE/sources.json').sources) if (!s.targets || s.targets.includes('$TARGET')) console.log('  ' + s.name + ' ' + s.version + ' (' + s.licence + ')')"
  echo
  echo "Configured with:"
  printf '  %s\n' "${CONFIGURE[@]}" | sed "s|$WORK|<work>|g"
} > "$DIST/BUILD.txt"

rm -f "$OUT/$NAME.zip" "$OUT/$NAME.tar.xz"
if [ "$OS" = windows ]; then
  (cd "$DIST" && zip -9 -q "$OUT/$NAME.zip" ./*)
  echo "$OUT/$NAME.zip"
else
  tar -cJf "$OUT/$NAME.tar.xz" -C "$DIST" .
  echo "$OUT/$NAME.tar.xz"
fi
