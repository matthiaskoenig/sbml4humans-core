#!/usr/bin/env bash
# Builds the libsbml the crate sbml4humans-libsbml links, into third_party/prefix.
#
# A static libsbml 5.21.2 which parses with expat and carries exactly the
# packages python-libsbml 5.21.2 registers, because the report of a document
# lists the packages the library knows and the oracle gate compares that list
# with the one the python implementation writes. Every ENABLE_ option of the
# release is named below, on for that set and off for the three packages python
# does not carry, so the set does not follow the defaults of cmake. What a built
# prefix actually carries is recorded in its
# include/sbml/common/libsbml-config-packages.h.
#
# The prefix holds include/ and lib/, which is what the build script of the
# crate expects; LIBSBML_DIR names another prefix instead.
#
# Needs git, cmake, a C++ compiler and the headers of expat, zlib and bzip2.
set -euo pipefail

VERSION=v5.21.2
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
BUILD_ROOT=${LIBSBML_BUILD_ROOT:-$ROOT/third_party}
SOURCE=$BUILD_ROOT/libsbml
BUILD=$BUILD_ROOT/build
PREFIX=$BUILD_ROOT/prefix
mkdir -p "$BUILD_ROOT"

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
    -DENABLE_DISTRIB=ON \
    -DENABLE_FBC=ON \
    -DENABLE_GROUPS=ON \
    -DENABLE_L3V2EXTENDEDMATH=ON \
    -DENABLE_LAYOUT=ON \
    -DENABLE_MULTI=ON \
    -DENABLE_QUAL=ON \
    -DENABLE_RENDER=ON \
    -DENABLE_SPATIAL=ON \
    -DENABLE_ARRAYS=OFF \
    -DENABLE_DYN=OFF \
    -DENABLE_REQUIREDELEMENTS=OFF

# A caller may allocate more jobs explicitly; never expand to all host CPUs.
cmake --build "$BUILD" --parallel "${CMAKE_BUILD_PARALLEL_LEVEL:-2}"
cmake --install "$BUILD"

echo "libsbml $VERSION is installed in $PREFIX"
