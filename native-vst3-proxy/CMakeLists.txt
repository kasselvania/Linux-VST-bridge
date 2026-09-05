if(NOT EXISTS "${AP2_RUST_LIBRARY}")
 message(FATAL_ERROR "AP2_RUST_LIBRARY must name the separately bound PIC GNU Rust static library")
endif()
smtg_add_vst3plugin(AGainOfflineBridge
 "${VST3_SDK_ROOT}/public.sdk/source/main/linuxmain.cpp"
 source/processor.cpp source/factory.cpp)
target_include_directories(AGainOfflineBridge PRIVATE include)
target_link_libraries(AGainOfflineBridge PRIVATE sdk "${AP2_RUST_LIBRARY}" pthread dl m)
target_compile_features(AGainOfflineBridge PRIVATE cxx_std_20)
target_compile_options(AGainOfflineBridge PRIVATE -Wall -Wextra -Wpedantic -Werror -Wno-error=stringop-truncation)
set_target_properties(AGainOfflineBridge PROPERTIES CXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN ON)
target_link_options(AGainOfflineBridge PRIVATE "-Wl,--exclude-libs,ALL" "-Wl,-z,defs")
add_executable(ap2-offline-host host/main.cpp
 "${VST3_SDK_ROOT}/public.sdk/source/vst/hosting/module_linux.cpp")
target_link_libraries(ap2-offline-host PRIVATE sdk_hosting pthread dl)
target_compile_features(ap2-offline-host PRIVATE cxx_std_20)
target_compile_options(ap2-offline-host PRIVATE -Wall -Wextra -Wpedantic -Werror)
