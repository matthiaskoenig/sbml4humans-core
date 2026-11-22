#!/usr/bin/env bash
# Builds the libsbml the crate sbml4humans-libsbml links, into third_party/prefix.
#
# A static libsbml 5.21.2 which parses with expat and carries the packages the
# report reads. The prefix holds include/ and lib/, which is what the build
# script of the crate expects; LIBSBML_DIR names another prefix instead.
#
# Needs git, cmake, a C++ compiler and the headers of expat, zlib and bzip2.
set -euo pipefail

VERSION=v5.21.2
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
SOURCE=$ROOT/third_party/libsbml
BUILD=$ROOT/third_party/build
PREFIX=$ROOT/third_party/prefix

if [ ! -d "$SOURCE" ]; then
    git clone --depth 1 --branch "$VERSION" https://github.com/sbmlteam/libsbml.git "$SOURCE"
fi

cmake -S "$SOURCE" -B "$BUILD" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DLIBSBML_SKIP_SHARED_LIBRARY=ON \
    -DWITH_EXPAT=ON \
    -DWITH_LIBXML=OFF \
    -DWITH_XERCES=OFF \
    -DWITH_ZLIB=ON \
    -DWITH_BZIP2=ON \
    -DWITH_SWIG=OFF \
    -DWITH_CHECK=OFF \
    -DWITH_CPP_NAMESPACE=ON \
    -DENABLE_COMP=ON \
    -DENABLE_FBC=ON \
    -DENABLE_QUAL=ON \
    -DENABLE_DISTRIB=ON

cmake --build "$BUILD" --parallel "$(nproc)"
cmake --install "$BUILD"

echo "libsbml $VERSION is installed in $PREFIX"
