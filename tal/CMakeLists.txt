cmake_minimum_required(VERSION 3.24)
project(tal VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
add_compile_options(-fvisibility=hidden)

# ---------- vcpkg helper (header-only libs) ----------
include(FetchContent)
FetchContent_Declare(
  vcpkg-tool
  GIT_REPOSITORY https://github.com/Microsoft/vcpkg.git
  GIT_TAG        2024.06.15
)
FetchContent_MakeAvailable(vcpkg-tool)
set(CMAKE_TOOLCHAIN_FILE "${vcpkg-tool_SOURCE_DIR}/scripts/buildsystems/vcpkg.cmake"
    CACHE STRING "Vcpkg toolchain")

find_package(pybind11 CONFIG REQUIRED)
find_package(Threads REQUIRED)
find_package(Python3 REQUIRED COMPONENTS Interpreter Development)

# ---------- Timechain (PoT + VDF) ----------
add_library(timechain STATIC
    src/timechain/vdf.cpp
    src/timechain/pot_consensus.cpp
    src/timechain/omni_bridge.cpp)
target_include_directories(timechain PUBLIC src/timechain)
target_link_libraries(timechain PRIVATE Threads::Threads)

# ---------- AurumGrid (Tesselate) ----------
add_library(aurumgrid STATIC
    src/aurumgrid/tesselate.cpp
    src/aurumgrid/tss_handler.cpp)
target_include_directories(aurumgrid PUBLIC src/aurumgrid)
target_link_libraries(aurumgrid PRIVATE timechain)

# ---------- LunCoSim wrapper ----------
add_library(lunco STATIC
    src/lunco/lunco_wrapper.cpp)
target_include_directories(lunco PUBLIC src/lunco)
target_link_libraries(lunco PRIVATE pybind11::pybind11)
# link against the real LunCoSim sources (see §3)

# ---------- Python module ----------
pybind11_add_module(_tal python/tal.cpp)
target_link_libraries(_tal PRIVATE timechain aurumgrid lunco Python3::Python)

# ---------- install rules ----------
install(TARGETS _tal
        LIBRARY DESTINATION tal)
