cmake_minimum_required(VERSION 3.25)

project(
    LinuxVstBridgeHP0
    VERSION 0.1.0
    DESCRIPTION "HP0 deterministic native Linux VST3 host probe"
    LANGUAGES C CXX
)

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

include(HP0ToolchainGuard)
hp0_require_freedesktop_sdk()

include(HP0Vst3SdkLock)
hp0_verify_vst3_sdk("${VST3_SDK_ROOT}")

if(NOT CMAKE_SYSTEM_NAME STREQUAL "Linux")
    message(FATAL_ERROR "HP0 requires Linux; observed '${CMAKE_SYSTEM_NAME}'")
endif()
if(NOT CMAKE_SYSTEM_PROCESSOR MATCHES "^(x86_64|amd64|AMD64)$")
    message(FATAL_ERROR "HP0 requires x86_64; observed '${CMAKE_SYSTEM_PROCESSOR}'")
endif()
if(NOT CMAKE_SIZEOF_VOID_P EQUAL 8)
    message(FATAL_ERROR "HP0 requires a 64-bit compiler")
endif()
if(NOT CMAKE_CXX_COMPILER_ID STREQUAL "GNU" OR NOT CMAKE_CXX_COMPILER_VERSION VERSION_EQUAL "15.2.0")
    message(FATAL_ERROR "HP0 requires Freedesktop SDK GCC 15.2.0; observed '${CMAKE_CXX_COMPILER_ID} ${CMAKE_CXX_COMPILER_VERSION}'")
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(FETCHCONTENT_FULLY_DISCONNECTED ON CACHE BOOL "No configure-time downloads" FORCE)

set(SMTG_ENABLE_VST3_PLUGIN_EXAMPLES OFF CACHE BOOL "" FORCE)
set(SMTG_ENABLE_VST3_HOSTING_EXAMPLES OFF CACHE BOOL "" FORCE)
set(SMTG_ENABLE_VSTGUI_SUPPORT OFF CACHE BOOL "" FORCE)
set(SMTG_CREATE_PLUGIN_LINK OFF CACHE BOOL "" FORCE)
set(SMTG_RUN_VST_VALIDATOR OFF CACHE BOOL "" FORCE)

add_subdirectory("${VST3_SDK_ROOT}" "${CMAKE_BINARY_DIR}/vst3sdk")

include(HP0ModernGcc)
hp0_remove_legacy_stdcxxfs()

if(NOT TARGET validator)
    message(FATAL_ERROR "Pinned VST3 SDK did not expose the official validator target")
endif()

add_subdirectory(native-probe)
hp0_remove_legacy_stdcxxfs()

add_custom_target(hp0-probe DEPENDS LabHostProbe validator)
