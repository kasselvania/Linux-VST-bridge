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
target_link_libraries(wf0-factory-probe PRIVATE sdk_hosting base pluginterfaces advapi32 ws2_32 user32 ole32)
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
target_link_libraries(wf0-loader-adapter-tests PRIVATE sdk_hosting base pluginterfaces advapi32 ws2_32 user32 ole32)
set_target_properties(wf0-loader-adapter-tests PROPERTIES
    OUTPUT_NAME "wf0-loader-adapter-tests"
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")

# Deterministic SDK producer/consumer instrumentation for the AP10 return seam.
add_library(ap10-return-fixture SHARED
    ../windows-fixtures/ap10/return_fixture.cpp
    "${WF0_VST3_SDK_ROOT}/public.sdk/source/main/dllmain.cpp")
target_include_directories(ap10-return-fixture PRIVATE "${WF0_VST3_SDK_ROOT}")
target_compile_definitions(ap10-return-fixture PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_link_libraries(ap10-return-fixture PRIVATE sdk)
set_target_properties(ap10-return-fixture PROPERTIES PREFIX "" SUFFIX ".vst3" RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")
add_executable(ap10-results-tests ../tools/ap10-tests/results.cpp)
target_include_directories(ap10-results-tests PRIVATE "${CMAKE_SOURCE_DIR}" "${CMAKE_SOURCE_DIR}/native-vst3-proxy/include" "${WF0_VST3_SDK_ROOT}")
target_compile_options(ap10-results-tests PRIVATE /UNDEBUG)
target_link_libraries(ap10-results-tests PRIVATE sdk)
set_target_properties(ap10-results-tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")

add_executable(ap11-editor-tests ../tools/ap11-tests/windows.cpp)
target_include_directories(ap11-editor-tests PRIVATE source "${WF0_VST3_SDK_ROOT}")
target_compile_definitions(ap11-editor-tests PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(ap11-editor-tests PRIVATE /W4 /permissive- /EHsc)
target_link_libraries(ap11-editor-tests PRIVATE sdk sdk_hosting user32 ole32)
set_target_properties(ap11-editor-tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")

add_executable(ap11-state-tests ../tools/ap11-tests/state.cpp)
target_include_directories(ap11-state-tests PRIVATE "${CMAKE_SOURCE_DIR}" "${WF0_VST3_SDK_ROOT}")
target_link_libraries(ap11-state-tests PRIVATE sdk sdk_hosting)
set_target_properties(ap11-state-tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")

add_executable(ap12-path-tests ../tools/ap12-tests/paths.cpp)
target_include_directories(ap12-path-tests PRIVATE source)
target_compile_definitions(ap12-path-tests PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(ap12-path-tests PRIVATE /UNDEBUG)
set_target_properties(ap12-path-tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")

add_executable(ap12-fault-tests ../tools/ap12-tests/fault_status.cpp)
target_include_directories(ap12-fault-tests PRIVATE source "${WF0_VST3_SDK_ROOT}")
target_compile_definitions(ap12-fault-tests PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(ap12-fault-tests PRIVATE /UNDEBUG /EHsc)
target_link_libraries(ap12-fault-tests PRIVATE sdk)
set_target_properties(ap12-fault-tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")

add_executable(ap13-state-delivery-tests ../tools/ap13-tests/state_delivery.cpp source/mapped_processing.cpp)
target_include_directories(ap13-state-delivery-tests PRIVATE source include "${WF0_VST3_SDK_ROOT}")
target_compile_definitions(ap13-state-delivery-tests PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(ap13-state-delivery-tests PRIVATE /W4 /permissive- /EHsc)
target_link_libraries(ap13-state-delivery-tests PRIVATE sdk sdk_hosting ws2_32 user32 ole32)
set_target_properties(ap13-state-delivery-tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")

add_executable(ap14-environment-tests ../tools/ap14-tests/environment_desktop.cpp)
target_include_directories(ap14-environment-tests PRIVATE source)
target_compile_definitions(ap14-environment-tests PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(ap14-environment-tests PRIVATE /UNDEBUG /W4 /permissive- /EHsc)
target_link_libraries(ap14-environment-tests PRIVATE user32)
set_target_properties(ap14-environment-tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")

add_executable(ap18-result-custody-tests ../tools/ap18-tests/result_custody.cpp source/offline_processing.cpp source/component_instance_session.cpp)
target_include_directories(ap18-result-custody-tests PRIVATE source include "${WF0_VST3_SDK_ROOT}")
target_compile_definitions(ap18-result-custody-tests PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(ap18-result-custody-tests PRIVATE /UNDEBUG /W4 /permissive- /EHsc)
target_link_libraries(ap18-result-custody-tests PRIVATE sdk sdk_hosting user32 ole32)
set_target_properties(ap18-result-custody-tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")

add_executable(ap18-bus-census-tests ../tools/ap18-tests/bus_census.cpp)
target_include_directories(ap18-bus-census-tests PRIVATE source "${WF0_VST3_SDK_ROOT}")
target_compile_definitions(ap18-bus-census-tests PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
target_compile_options(ap18-bus-census-tests PRIVATE /UNDEBUG /W4 /permissive- /EHsc)
target_link_libraries(ap18-bus-census-tests PRIVATE sdk sdk_hosting)
set_target_properties(ap18-bus-census-tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")
