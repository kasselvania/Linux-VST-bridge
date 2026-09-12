#pragma once
#include "editor_session.h"
#include "retirement_status.h"
namespace linux_vst_bridge::wf0 {
// Shared final-owner path. Quiescence comes only from run_offline_processing;
// pending-work custody comes from the mapped session under its owner mutex.
// No destructor is invoked here. Caller must remain owned until containment.
template<class CloseEndpoint>
void complete_process_retirement(bool quiescent,bool no_pending,EditorSession* editor,
 RetirementStatus& status,uint64_t epoch,uint64_t sequence,uint64_t position,uint64_t generation,CloseEndpoint close_endpoint){
 ap1::require(quiescent,"process retirement requires joined processing");
 ap1::require(no_pending,"process retirement has pending work");
 ap1::require(!editor||editor->retire_process_scoped(),"process retirement view detach failed");
 close_endpoint();
 status.publish(RetirementStatus::complete,epoch,sequence,position,generation);
}
}
