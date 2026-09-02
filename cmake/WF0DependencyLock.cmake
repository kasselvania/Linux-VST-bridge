set(WF0_VST3_SDK_COMMIT "3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96")
set(WF0_VST3_SDK_TREE "38343890fd1a0cedd48b7ec80ef17da15231b6c8")

set(WF0_VST3_SOURCE_LOCKS
    "CMakeLists.txt|f006a70c9b5d115928cd14d0580ca541d72717b0|5f3d95f1037d501e5c0018e282cff2c5e4a0f35e4c7f0c20ee6cf5d3357d6497"
    "cmake/modules/SMTG_VstGuiSupport.cmake|0c63c1c2322c9e1374dd45b297e2508a0f93e8bc|4c99377c14b253aa7f7da699e7376db5239a129823516a6eaa89ec78c2ea25b5"
    "vstgui4/CMakeLists.txt|8fa41c406756788e44c7c301c3bb9d9b1ede5335|f8c6bacb19cbc7d309326ac407db68049028aa5579489121d331b541843a730a"
    "vstgui4/vstgui/standalone/CMakeLists.txt|a1bbc822ea153d1ef1036241da79b437c82ddf40|70269facc46bb4094a93feccf0947e9efe92bfa76c2293612fbee857a06a0942"
    "public.sdk/samples/vst/again/CMakeLists.txt|f2616195f4f0b92b55ade45ac2fa448a4674ec79|b3b6865609cfe50338f210cc90b7f6d137204b145e05e01402baac19f95abc89"
    "public.sdk/samples/vst/again/source/againentry.cpp|13b920b4b7a74137301bf213cf048e96e82861d4|1cf23e867418578b4676a6298386d8eedaf463846d5ef8128389635731f72708"
    "public.sdk/samples/vst/again/source/againcids.h|d32d1640ea187e718baba9cbb039d3a436d53cce|5b6bd8bd5a714ababddbf490b55abd0d5d232b03b294882bf783e4421ee4715e")

set(WF0_VST3_SUBMODULE_LOCKS
    "base|fcf9da0bd27a16f7f03773a3a39822f28f5c8477"
    "cmake|054c9143cbb8d47fc4694e473f2ee3b4d951a8f5"
    "doc|8bfca19d3b76a61d093951ba9297047f544caea1"
    "pluginterfaces|4f547e8e102b47de4a8b8aaf343c73b700786372"
    "public.sdk|586dc5e6c8012c3e4b01c79389375cbe96bdb1da"
    "tutorials|33b73dfbb87f3fde3bce8c0a10cae934dc66ad34"
    "vstgui4|5db272256172557818b6158cf0bb2c4410bddb25")

function(wf0_git_read output root)
    find_program(WF0_GIT_EXECUTABLE NAMES git REQUIRED NO_CACHE)
    execute_process(
        COMMAND "${WF0_GIT_EXECUTABLE}" -C "${root}" ${ARGN}
        RESULT_VARIABLE result
        OUTPUT_VARIABLE value
        ERROR_VARIABLE error
        OUTPUT_STRIP_TRAILING_WHITESPACE)
    if(NOT result EQUAL 0)
        message(FATAL_ERROR "WF0 dependency readback failed: ${error}")
    endif()
    set(${output} "${value}" PARENT_SCOPE)
endfunction()

function(wf0_verify_git_checkout_configuration root label)
    wf0_git_read(autocrlf "${root}" config --local --get-all core.autocrlf)
    wf0_git_read(eol "${root}" config --local --get-all core.eol)
    if(NOT autocrlf STREQUAL "false" OR NOT eol STREQUAL "lf")
        message(FATAL_ERROR
            "WF0 ${label} checkout configuration differs: "
            "core.autocrlf=${autocrlf} core.eol=${eol}")
    endif()
endfunction()

function(wf0_verify_vst3_sdk root)
    if(NOT IS_ABSOLUTE "${root}" OR NOT EXISTS "${root}/.git")
        message(FATAL_ERROR "WF0_VST3_SDK_ROOT must be the exact absolute pinned checkout")
    endif()
    get_filename_component(real_root "${root}" REALPATH)
    if(NOT real_root STREQUAL root)
        message(FATAL_ERROR "WF0 SDK root may not be a symlink or lexical alias")
    endif()

    wf0_git_read(head "${root}" rev-parse HEAD)
    wf0_git_read(tree "${root}" rev-parse "HEAD^{tree}")
    wf0_git_read(status "${root}" status --porcelain=v1 --untracked-files=all)
    if(NOT head STREQUAL WF0_VST3_SDK_COMMIT OR NOT tree STREQUAL WF0_VST3_SDK_TREE)
        message(FATAL_ERROR "WF0 pinned SDK identity mismatch: ${head} ${tree}")
    endif()
    if(NOT status STREQUAL "")
        message(FATAL_ERROR "WF0 pinned SDK is dirty")
    endif()
    wf0_verify_git_checkout_configuration("${root}" "SDK root")

    foreach(lock IN LISTS WF0_VST3_SUBMODULE_LOCKS)
        string(REPLACE "|" ";" fields "${lock}")
        list(GET fields 0 path)
        list(GET fields 1 expected)
        wf0_git_read(observed "${root}/${path}" rev-parse HEAD)
        wf0_git_read(gitlink "${root}" rev-parse "HEAD:${path}")
        wf0_git_read(sub_status "${root}/${path}" status --porcelain=v1 --untracked-files=all)
        if(NOT observed STREQUAL expected OR NOT gitlink STREQUAL expected OR
           NOT sub_status STREQUAL "")
            message(FATAL_ERROR "WF0 SDK submodule mismatch: ${path} ${observed}")
        endif()
        wf0_verify_git_checkout_configuration("${root}/${path}" "SDK submodule ${path}")
    endforeach()

    foreach(lock IN LISTS WF0_VST3_SOURCE_LOCKS)
        string(REPLACE "|" ";" fields "${lock}")
        list(GET fields 0 path)
        list(GET fields 1 expected_blob)
        list(GET fields 2 expected_sha256)
        set(repository_root "${root}")
        set(repository_path "${path}")
        foreach(submodule_lock IN LISTS WF0_VST3_SUBMODULE_LOCKS)
            string(REPLACE "|" ";" submodule_fields "${submodule_lock}")
            list(GET submodule_fields 0 submodule_path)
            string(FIND "${path}" "${submodule_path}/" submodule_prefix)
            if(submodule_prefix EQUAL 0)
                string(LENGTH "${submodule_path}/" submodule_prefix_length)
                string(SUBSTRING "${path}" ${submodule_prefix_length} -1 repository_path)
                set(repository_root "${root}/${submodule_path}")
            endif()
        endforeach()
        wf0_git_read(locked_blob "${repository_root}"
            rev-parse "HEAD:${repository_path}")
        wf0_git_read(unfiltered_worktree_blob "${repository_root}"
            hash-object --no-filters -- "${root}/${path}")
        file(SHA256 "${root}/${path}" observed_sha256)
        if(NOT locked_blob STREQUAL expected_blob OR
           NOT unfiltered_worktree_blob STREQUAL expected_blob OR
           NOT observed_sha256 STREQUAL expected_sha256)
            message(FATAL_ERROR "WF0 SDK locked source differs: ${path}")
        endif()
    endforeach()
endfunction()
