# Separate first-party instrumentation identities; the delivered reference
# modules and their existing baseline contract remain unchanged.
foreach(role IN ITEMS instrument effect)
    if(role STREQUAL "instrument")
        set(completion_is_instrument 1)
    else()
        set(completion_is_instrument 0)
    endif()
    add_library(lvb-completion-${role} SHARED
        "${CMAKE_CURRENT_LIST_DIR}/stateful.cpp"
        "${WF0_VST3_SDK_ROOT}/public.sdk/source/main/dllmain.cpp")
    target_include_directories(lvb-completion-${role} PRIVATE "${WF0_VST3_SDK_ROOT}")
    target_compile_definitions(lvb-completion-${role} PRIVATE LVB_BETA_COMPLETION=1
        LVB_BETA_INSTRUMENT=${completion_is_instrument} UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
    target_compile_options(lvb-completion-${role} PRIVATE /W4 /permissive- /EHsc /Brepro)
    target_link_options(lvb-completion-${role} PRIVATE /Brepro /INCREMENTAL:NO)
    target_link_libraries(lvb-completion-${role} PRIVATE sdk)
    set_target_properties(lvb-completion-${role} PROPERTIES PREFIX "" SUFFIX ".vst3"
        RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")
    add_executable(lvb-completion-${role}-tests "${CMAKE_CURRENT_LIST_DIR}/completion_contract_tests.cpp")
    target_include_directories(lvb-completion-${role}-tests PRIVATE "${WF0_VST3_SDK_ROOT}" "${CMAKE_SOURCE_DIR}")
    target_compile_definitions(lvb-completion-${role}-tests PRIVATE
        LVB_BETA_INSTRUMENT=${completion_is_instrument} UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
    target_compile_options(lvb-completion-${role}-tests PRIVATE /W4 /permissive- /EHsc)
    target_link_libraries(lvb-completion-${role}-tests PRIVATE sdk sdk_hosting ws2_32)
    set_target_properties(lvb-completion-${role}-tests PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/bin")
endforeach()
