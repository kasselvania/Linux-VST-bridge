set(WF0_VST3_SDK_COMMIT "3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96")
set(WF0_VST3_SDK_TREE "38343890fd1a0cedd48b7ec80ef17da15231b6c8")

set(WF0_VST3_SOURCE_LOCKS
    "CMakeLists.txt|f006a70c9b5d115928cd14d0580ca541d72717b0|5f3d95f1037d501e5c0018e282cff2c5e4a0f35e4c7f0c20ee6cf5d3357d6497"
    "cmake/modules/SMTG_VstGuiSupport.cmake|0c63c1c2322c9e1374dd45b297e2508a0f93e8bc|4c99377c14b253aa7f7da699e7376db5239a129823516a6eaa89ec78c2ea25b5"
    "vstgui4/CMakeLists.txt|8fa41c406756788e44c7c301c3bb9d9b1ede5335|f8c6bacb19cbc7d309326ac407db68049028aa5579489121d331b541843a730a"
    "vstgui4/vstgui/standalone/CMakeLists.txt|a1bbc822ea153d1ef1036241da79b437c82ddf40|70269facc46bb4094a93feccf0947e9efe92bfa76c2293612fbee857a06a0942"
    "public.sdk/samples/vst/again/CMakeLists.txt|f2616195f4f0b92b55ade45ac2fa448a4674ec79|b3b6865609cfe50338f210cc90b7f6d137204b145e05e01402baac19f95abc89"
    "public.sdk/samples/vst/again/source/againentry.cpp|13b920b4b7a74137301bf213cf048e96e82861d4|1cf23e867418578b4676a6298386d8eedaf463846d5ef8128389635731f72708"
    "public.sdk/samples/vst/again/source/againcids.h|d32d1640ea187e718baba9cbb039d3a436d53cce|5b6bd8bd5a714ababddbf490b55abd0d5d232b03b294882bf783e4421ee4715e"
    "pluginterfaces/base/ipluginbase.h|859424bfc7f14b61df4b209a85c09d413513127a|e10e9a4b9b0811c392af5758542e1f72875b56ebe7e1673d5075bc1160d9fd0b"
    "pluginterfaces/base/funknown.h|3f6de83b104484e09097411417b37bcd28a46a1c|e0d9609224fe15491c9ccd1463d964c303f1d5a6149fabf849e87f2c14be1951"
    "pluginterfaces/base/smartpointer.h|ca64ae8f6260abc4225869bcb55ef6b27f3f3abf|f690163a2d5fa71e76cce1909ae719561031b897f79342ddf38190b2ed9613b3"
    "pluginterfaces/vst/ivsthostapplication.h|1818efe85a6674bf706fd8f8040e28cb329fbe27|b7ef5b02f24c103e952e18b3e974e97b3c23d19c870a46c7c3aad36af8832332"
    "pluginterfaces/vst/ivstcomponent.h|e20ef5f429349bdccf55367d45051fcdb0ff97c5|cc587e34c009388d4187948c14a651df1481a01920abe895023094ca5b13faee"
    "base/source/fobject.h|6d092acfcc7bf3be91f8f00bcb3e6c27184d53eb|6c8ef34413eed9fabbc61befaa40b213bbec4fe9cd7195738297caaaf1001767"
    "base/source/fobject.cpp|a1da6cfffb23eb77d53d39da9429709f246955e6|07ced0bc6398ea2d6d364e787d8cb7f09d8d8052af5b3e219e3df2b62341f54f"
    "public.sdk/source/main/pluginfactory.cpp|a50c5000c1215ced5ae920a07c12153edb9b2498|81dc1e6b5619ef1f22b83eb7243aa45d3ec7c54dd5b9fd42a32310bf40658f11"
    "public.sdk/source/vst/vstcomponentbase.cpp|ccfde59797185b8a793b4a5bff2246c711faa18a|00f279150eb1cccaa78a5e6c62fbbf19b6149e7af531c90c1482102162b4b7ab"
    "public.sdk/source/vst/vstcomponent.cpp|d1dced4a441d35b73717da26eea0ada97406a874|e9d8e5b4e25d319e378b8c8d547f34a3aa01f733c9ed3b60ddbe4c73237d7968"
    "public.sdk/source/vst/vstaudioeffect.h|818cc4f3357c15c6f9a5ba649dbc70e87ced688f|d61a3f92770bcab1b6dfafd49ea9935de8576ab92e251569c5bc2756835c35d9"
    "public.sdk/source/vst/hosting/hostclasses.cpp|fd0e12498e7f9035e6d9f23c154cf50adfd8a6ab|f9fbcd410d09ea3352342fdcb645e6a9dd1a424ff5b42430287fd4885a063c1d"
    "public.sdk/samples/vst/again/source/again.h|061c0d4afb5f59410e75681d9998871f32fb1e72|304b289e902c302928e2a3ebdc117eeef6af2781855445712f515301d4295e23"
    "public.sdk/samples/vst/again/source/again.cpp|4676454679e37f188b99c2ec6e6def6b825da173|05ff84588eac6ced18f26bba4e98632b28a2b5ee9d8c00340139fcc6b0efb00b")

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
