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
struct Nad1ProcessAdmission {
    bool opened=false, valid=false;
    uint64_t created=0;
    uint32_t error=0;
};
struct Nad1StopAdmission {
    Nad1StopStatus first{};
    bool valid=false, exit_witness=false;
    uint32_t pid=0, identity_error=0;
    uint64_t created=0;
};
// Admit either the exact retained process handle or the exact receipt published
// before that handle was released. A failed acquisition gets one bounded handoff
// recheck: fresh STOPPED, exact receipt and zero listeners. An opened but
// unverified process is never replaced by receipt evidence.
template<class IO> Nad1StopAdmission nad1_admit_stop_generation(
    IO& io,Nad1StopStatus first,uint32_t observed_pid,uint32_t prior_pid,uint64_t prior_created) {
    Nad1StopAdmission r;r.first=first;
    if(first.error)return r;
    r.valid=true;
    if(first.state==4){r.pid=observed_pid;if(prior_pid&&r.pid!=prior_pid){r.valid=false;r.identity_error=13;}}
    else if(prior_pid)r.pid=prior_pid;
    else if(first.state!=1){r.valid=false;r.identity_error=13;}
    if(!r.valid||!r.pid)return r;
    if(first.state==1&&prior_created){
        const int witness=io.exit_witness(r.pid,prior_created);
        if(witness<0){r.valid=false;r.identity_error=13;return r;}
        if(witness==1){r.exit_witness=true;r.created=prior_created;return r;}
    }
    auto process=io.acquire(r.pid,prior_created);
    if(process.opened){
        r.valid=process.valid&&process.created&&(!prior_created||process.created==prior_created);
        r.created=process.created;
        if(!r.valid)r.identity_error=process.error?process.error:13;
        return r;
    }
    r.valid=false;r.identity_error=process.error?process.error:13;
    auto fresh=io.refresh();
    if(fresh.error||fresh.state!=1||!prior_created)return r;
    const int witness=io.exit_witness(r.pid,prior_created);
    if(witness<0){r.identity_error=13;return r;}
    if(witness!=1||io.endpoints(r.pid)!=0)return r;
    r.first=fresh;r.valid=true;r.exit_witness=true;r.created=prior_created;r.identity_error=0;
    return r;
}
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
