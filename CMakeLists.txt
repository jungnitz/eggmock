cmake_minimum_required(VERSION 3.19)
project(eggmock)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

include(FetchContent)

FetchContent_Declare(
        Corrosion
        GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
        GIT_TAG v0.5 # Optionally specify a commit hash, version tag or branch here
)
FetchContent_MakeAvailable(Corrosion)

add_subdirectory(mockturtle)
add_subdirectory(examples)

add_library(eggmock INTERFACE)
target_link_libraries(eggmock INTERFACE mockturtle)
target_include_directories(eggmock INTERFACE include)
