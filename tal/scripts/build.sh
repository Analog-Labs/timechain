#!/usr/bin/env bash
set -e
cd "$(dirname "$0")/.."

echo "=== Configure ==="
cmake -B build -DCMAKE_BUILD_TYPE=Release -Dpybind11_DIR=/home/jules/.pyenv/versions/3.12.12/lib/python3.12/site-packages/pybind11/share/cmake/pybind11

echo "=== Build C++ ==="
cmake --build build -j$(nproc)

echo "=== Install Python package ==="
cd python
pip install -e .
