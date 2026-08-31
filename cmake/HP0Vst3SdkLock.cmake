set(HP0_EXPECTED_VST3_SDK_COMMIT "3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96")
set(HP0_EXPECTED_VST3_SUBMODULES
    "base|fcf9da0bd27a16f7f03773a3a39822f28f5c8477"
    "cmake|054c9143cbb8d47fc4694e473f2ee3b4d951a8f5"
    "doc|8bfca19d3b76a61d093951ba9297047f544caea1"
    "pluginterfaces|4f547e8e102b47de4a8b8aaf343c73b700786372"
    "public.sdk|586dc5e6c8012c3e4b01c79389375cbe96bdb1da"
    "tutorials|33b73dfbb87f3fde3bce8c0a10cae934dc66ad34"
    "vstgui4|5db272256172557818b6158cf0bb2c4410bddb25"
)

function(hp0_git_output output_var sdk_root)
    execute_process(
        COMMAND git -C "${sdk_root}" ${ARGN}
        RESULT_VARIABLE hp0_git_rc
        OUTPUT_VARIABLE hp0_git_stdout
        ERROR_VARIABLE hp0_git_stderr
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    if(NOT hp0_git_rc EQUAL 0)
        message(FATAL_ERROR "Git dependency verification failed (${ARGN}): ${hp0_git_stderr}")
    endif()
    set("${output_var}" "${hp0_git_stdout}" PARENT_SCOPE)
endfunction()

function(hp0_verify_vst3_sdk sdk_root)
    if(NOT DEFINED VST3_SDK_ROOT OR "${sdk_root}" STREQUAL "")
        message(FATAL_ERROR "VST3_SDK_ROOT is required and must name the pinned untracked SDK checkout")
    endif()
    if(NOT IS_ABSOLUTE "${sdk_root}" OR NOT EXISTS "${sdk_root}/CMakeLists.txt")
        message(FATAL_ERROR "VST3_SDK_ROOT must be an existing absolute SDK checkout")
    endif()
    if(NOT EXISTS "${sdk_root}/.git" OR NOT EXISTS "${sdk_root}/LICENSE.txt")
        message(FATAL_ERROR "VST3_SDK_ROOT is not a complete Git checkout with license metadata")
    endif()

    hp0_git_output(hp0_root_commit "${sdk_root}" rev-parse HEAD)
    if(NOT hp0_root_commit STREQUAL HP0_EXPECTED_VST3_SDK_COMMIT)
        message(FATAL_ERROR "Wrong VST3 SDK commit: expected ${HP0_EXPECTED_VST3_SDK_COMMIT}, observed ${hp0_root_commit}")
    endif()

    hp0_git_output(hp0_root_status "${sdk_root}" status --porcelain=v1 --untracked-files=all --ignore-submodules=none)
    if(NOT hp0_root_status STREQUAL "")
        message(FATAL_ERROR "VST3 SDK checkout is dirty or has incomplete submodules")
    endif()

    foreach(hp0_entry IN LISTS HP0_EXPECTED_VST3_SUBMODULES)
        string(REPLACE "|" ";" hp0_pair "${hp0_entry}")
        list(GET hp0_pair 0 hp0_path)
        list(GET hp0_pair 1 hp0_expected_commit)
        if(NOT EXISTS "${sdk_root}/${hp0_path}/.git")
            message(FATAL_ERROR "Required VST3 SDK submodule is not initialized: ${hp0_path}")
        endif()
        hp0_git_output(hp0_submodule_commit "${sdk_root}/${hp0_path}" rev-parse HEAD)
        if(NOT hp0_submodule_commit STREQUAL hp0_expected_commit)
            message(FATAL_ERROR "Wrong VST3 SDK submodule ${hp0_path}: expected ${hp0_expected_commit}, observed ${hp0_submodule_commit}")
        endif()
        hp0_git_output(hp0_submodule_status "${sdk_root}/${hp0_path}" status --porcelain=v1 --untracked-files=all --ignore-submodules=none)
        if(NOT hp0_submodule_status STREQUAL "")
            message(FATAL_ERROR "VST3 SDK submodule is dirty: ${hp0_path}")
        endif()
    endforeach()

    foreach(hp0_required_file IN ITEMS
        "cmake/modules/SMTG_AddVST3Library.cmake"
        "public.sdk/samples/vst-hosting/validator/CMakeLists.txt"
        "public.sdk/source/vst/vstaudioeffect.h"
        "public.sdk/source/vst/vsteditcontroller.h"
    )
        if(NOT EXISTS "${sdk_root}/${hp0_required_file}")
            message(FATAL_ERROR "Pinned VST3 SDK interface missing: ${hp0_required_file}")
        endif()
    endforeach()
endfunction()
