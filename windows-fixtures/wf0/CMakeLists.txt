set(WF0_FAULT_TARGETS
    wf0-missing-factory
    wf0-null-factory
    wf0-no-entry
    wf0-factory1-only
    wf0-factory2-only
    wf0-factory3-fallback
    wf0-init-false
    wf0-factory-info-false
    wf0-count-negative
    wf0-count-excessive
    wf0-class-info-false
    wf0-duplicate-class-id
    wf0-hang-entry
    wf0-hang-factory
    wf0-hang-class
    wf0-crash-entry
    wf0-crash-factory
    wf0-crash-class
    wf0-hang-release
    wf0-crash-release
    wf0-exit-false
    wf0-create-instance-tripwire)

set(fault_kind 0)
foreach(target IN LISTS WF0_FAULT_TARGETS)
    add_library(${target} SHARED source/fault_fixture.cpp)
    target_include_directories(${target} PRIVATE "${WF0_VST3_SDK_ROOT}")
    target_compile_definitions(${target} PRIVATE
        WF0_FAULT_KIND=${fault_kind} UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
    target_compile_options(${target} PRIVATE /W4 /permissive- /Brepro)
    target_link_options(${target} PRIVATE /Brepro /INCREMENTAL:NO)
    target_link_libraries(${target} PRIVATE pluginterfaces)
    set_target_properties(${target} PROPERTIES
        PREFIX ""
        OUTPUT_NAME "${target}"
        RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/fixtures"
        LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/fixtures")
    math(EXPR fault_kind "${fault_kind} + 1")
endforeach()

add_custom_target(wf0-fault-fixtures DEPENDS ${WF0_FAULT_TARGETS})

set(WC0_FAULT_TARGETS
    wc0-create-failure-null
    wc0-create-success-null
    wc0-create-failure-nonnull
    wc0-controller-id-failure
    wc0-controller-id-mismatch
    wc0-initialize-failure
    wc0-initialize-hang
    wc0-initialize-crash
    wc0-terminate-failure
    wc0-terminate-hang
    wc0-terminate-crash
    wc0-release-nonzero
    wc0-release-hang
    wc0-release-crash
    wc0-host-object-request
    wc0-host-reference-leak)

set(wc0_fault_kind 0)
foreach(target IN LISTS WC0_FAULT_TARGETS)
    add_library(${target} SHARED source/fault_fixture.cpp)
    target_include_directories(${target} PRIVATE "${WF0_VST3_SDK_ROOT}")
    target_compile_definitions(${target} PRIVATE
        WF0_FAULT_KIND=255 WC0_FAULT_KIND=${wc0_fault_kind}
        UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
    target_compile_options(${target} PRIVATE /W4 /permissive- /Brepro)
    target_link_options(${target} PRIVATE /Brepro /INCREMENTAL:NO)
    target_link_libraries(${target} PRIVATE pluginterfaces)
    set_target_properties(${target} PROPERTIES
        PREFIX ""
        OUTPUT_NAME "${target}"
        RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/fixtures"
        LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/wf0/fixtures")
    math(EXPR wc0_fault_kind "${wc0_fault_kind} + 1")
endforeach()

add_custom_target(wc0-fault-fixtures DEPENDS ${WC0_FAULT_TARGETS})
