add_executable(wf0-factory-probe
    source/main.cpp
    source/component_instance_session.cpp
    source/offline_processing.cpp
    source/mapped_processing.cpp
    source/win32_module.cpp
    source/factory_census.cpp)
target_sources(wf0-factory-probe PRIVATE source/inspect_module.cpp)

target_include_directories(wf0-factory-probe PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/include"
    "${CMAKE_CURRENT_SOURCE_DIR}/source"
    "${WF0_VST3_SDK_ROOT}")
target_compile_definitions(wf0-factory-probe PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(wf0-factory-probe PRIVATE /W4 /permissive- /EHsc /Brepro)
target_link_options(wf0-factory-probe PRIVATE /Brepro /INCREMENTAL:NO)
target_link_libraries(wf0-factory-probe PRIVATE sdk_hosting base pluginterfaces advapi32 ws2_32)
set_target_properties(wf0-factory-probe PROPERTIES
    OUTPUT_NAME "wf0-factory-probe"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")

add_executable(wf0-loader-adapter-tests
    source/main.cpp
    source/component_instance_session.cpp
    source/offline_processing.cpp
    source/mapped_processing.cpp
    source/win32_module.cpp)
target_include_directories(wf0-loader-adapter-tests PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/include"
    "${CMAKE_CURRENT_SOURCE_DIR}/source"
    "${WF0_VST3_SDK_ROOT}")
target_compile_definitions(wf0-loader-adapter-tests PRIVATE
    WF0_LOADER_ADAPTER_TEST UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(wf0-loader-adapter-tests PRIVATE /W4 /permissive- /EHsc /Brepro)
target_link_options(wf0-loader-adapter-tests PRIVATE /Brepro /INCREMENTAL:NO)
target_link_libraries(wf0-loader-adapter-tests PRIVATE sdk_hosting base pluginterfaces advapi32 ws2_32)
set_target_properties(wf0-loader-adapter-tests PROPERTIES
    OUTPUT_NAME "wf0-loader-adapter-tests"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")
