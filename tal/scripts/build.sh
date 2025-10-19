#!/usr/bin/env bash
set -e
cd "$(dirname "$0")/.."

echo "=== Configure ==="
cmake -B build -DCMAKE_BUILD_TYPE=Release -DUILD_PYTHON=ON

echo "=== Build C++ ==="
cmake --build build -j$(nproc)

echo "=== Install Python package ==="
cd python
pip install -e .
