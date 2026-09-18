// Source-owned bounded SCM observation law; Win32 I/O is supplied by the adapter.
// SERVICE_STOPPED has no valid SCM PID. The retained process handle is independent.
#pragma once
#include <cstdint>
struct Nad1StopStatus {
    uint32_t state=0, checkpoint=0, hint=0, win32=0, specific=0, error=0;
};
struct Nad1StopObservation {
    Nad1StopStatus before{}, last{};
    bool confirmed=false, control_sent=false;
    uint32_t control_error=0, process_wait=0xffffffffu, wait_error=0, endpoints=4;
    uint32_t queries=0, progress=0;
    uint64_t control_started=0, control_elapsed=0, elapsed=0;
};
template<class IO> Nad1StopObservation nad1_observe_stop(IO& io, bool generation_valid) {
    Nad1StopObservation r;const auto begin=io.now();
    r.before=r.last=io.query();++r.queries;
    if(r.before.error||!generation_valid){r.elapsed=io.now()-begin;return r;}
    // Pending/stopped observations never send a redundant control. An already
    // pending generation must have been retained while its PID was valid.
    if(r.before.state!=1 && r.before.state!=3){
        r.control_sent=true;r.control_started=io.now()-begin;
        io.begin_control(r.before.state);
        r.control_error=io.stop();r.control_elapsed=io.now()-begin-r.control_started;
    }
    // Observe even a rejected/racing control once; never retry the request.
    do {
        auto next=io.query();++r.queries;
        if(next.error){r.last.error=next.error;break;}
        if(next.state!=r.last.state||next.checkpoint!=r.last.checkpoint)++r.progress;
        r.last=next;r.process_wait=io.wait(r.wait_error);r.endpoints=io.endpoints();
        if(next.state==1&&r.process_wait==0&&r.endpoints==0){r.confirmed=true;break;}
        if(r.control_error!=0 && r.control_error!=1062 && r.control_error!=1061)break;
        if(io.now()-begin>=12000)break;
        io.sleep();
    }while(io.now()-begin<12000);
    r.elapsed=io.now()-begin;return r;
}
