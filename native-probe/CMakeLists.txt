smtg_add_vst3plugin(LabHostProbe
    "${VST3_SDK_ROOT}/public.sdk/source/main/linuxmain.cpp"
    source/controller.cpp
    source/controller.h
    source/factory.cpp
    source/processor.cpp
    source/processor.h
    source/version.h
    include/lab_host_probe/ids.h
    include/lab_host_probe/state.h
)

target_include_directories(LabHostProbe PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/include")
target_link_libraries(LabHostProbe PRIVATE sdk)
target_compile_features(LabHostProbe PRIVATE cxx_std_20)
target_compile_options(LabHostProbe PRIVATE
    -Wall -Wextra -Wpedantic -Werror
    # GCC 15 diagnoses the pinned SDK's constexpr factory strncpy calls.
    -Wno-error=stringop-truncation
)
set_target_properties(LabHostProbe PROPERTIES
    CXX_EXTENSIONS OFF
    CXX_VISIBILITY_PRESET hidden
    VISIBILITY_INLINES_HIDDEN ON
)
