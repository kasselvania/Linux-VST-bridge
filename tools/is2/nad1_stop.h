// Source-owned bounded SCM observation law; Win32 I/O is supplied by the adapter.
// SERVICE_STOPPED has no valid SCM PID. The retained process handle is independent.
#pragma once
#include <array>
#include <cstdint>
struct Nad1StopStatus {
    uint32_t state=0, checkpoint=0, hint=0, win32=0, specific=0, error=0;
    uint32_t controls=0, type=0;
};
struct Nad1ControlResult {
    bool submitted=false;
    uint32_t error=0;
    Nad1StopStatus status{};
};
enum class Nad2StopClassification : uint32_t {
    Confirmed=1,
    SubmittedProgressing=2,
    SubmittedNoTransition=3,
    NotSubmitted=4,
    StoppedProcessOrListenerRemains=5,
    ObservationUnavailable=6,
};
inline const char* nad2_stop_classification_name(Nad2StopClassification value) {
    switch(value) {
    case Nad2StopClassification::Confirmed:return "NAD2_STOP_CONFIRMED";
    case Nad2StopClassification::SubmittedProgressing:return "NAD2_STOP_SUBMITTED_PROGRESSING";
    case Nad2StopClassification::SubmittedNoTransition:return "NAD2_STOP_SUBMITTED_NO_TRANSITION";
    case Nad2StopClassification::NotSubmitted:return "NAD2_STOP_NOT_SUBMITTED";
    case Nad2StopClassification::StoppedProcessOrListenerRemains:return "NAD2_STOPPED_PROCESS_OR_LISTENER_REMAINS";
    case Nad2StopClassification::ObservationUnavailable:return "NAD2_STOP_OBSERVATION_UNAVAILABLE";
    }
    return "NAD2_STOP_OBSERVATION_UNAVAILABLE";
}
static constexpr uint32_t NAD2_STOP_TRANSITION_CAP=12;
struct Nad1StopTransition {
    // source: 1=query observation, 2=successful ControlService return.
    uint32_t source=0;
    uint64_t elapsed=0;
    Nad1StopStatus service{};
    uint32_t process_wait_class=0, wait_error=0, endpoints=4;
};
struct Nad1StopObservation {
    Nad1StopStatus before{}, last{};
    Nad1StopStatus control_status{};
    bool confirmed=false, control_sent=false, control_submitted=false, control_status_available=false;
    uint32_t control_error=0, process_wait=0xffffffffu, wait_error=0, endpoints=4;
    uint32_t initial_process_wait=0xffffffffu, initial_wait_error=0, initial_endpoints=4;
    uint32_t control_count=0;
    uint32_t queries=0, progress=0;
    uint32_t transition_total=0, transition_retained=0, transition_dropped=0;
    uint64_t control_started=0, control_elapsed=0, elapsed=0;
    Nad2StopClassification classification=Nad2StopClassification::ObservationUnavailable;
    std::array<Nad1StopTransition,NAD2_STOP_TRANSITION_CAP> transitions{};
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
static uint32_t nad1_process_wait_class(uint32_t value) {
    if(value==0)return 1;       // exact process handle signaled
    if(value==258)return 2;     // WAIT_TIMEOUT
    if(value==0xffffffffu)return 3; // WAIT_FAILED or not available
    return 4;                   // unexpected/unavailable result
}
static bool nad1_same_transition(const Nad1StopTransition& a,const Nad1StopTransition& b) {
    const auto& x=a.service;const auto& y=b.service;
    return x.type==y.type&&x.state==y.state&&x.controls==y.controls&&x.checkpoint==y.checkpoint
        &&x.hint==y.hint&&x.win32==y.win32&&x.specific==y.specific&&x.error==y.error
        &&a.process_wait_class==b.process_wait_class&&a.wait_error==b.wait_error&&a.endpoints==b.endpoints;
}
static void nad1_record_transition(Nad1StopObservation& r,const Nad1StopTransition& value) {
    ++r.transition_total;
    if(r.transition_retained<NAD2_STOP_TRANSITION_CAP)r.transitions[r.transition_retained++]=value;
    else {r.transitions[NAD2_STOP_TRANSITION_CAP-1]=value;++r.transition_dropped;}
}
static Nad2StopClassification nad1_classify_stop(const Nad1StopObservation& r,bool generation_valid) {
    if(r.confirmed)return Nad2StopClassification::Confirmed;
    if(!generation_valid||r.before.error||r.last.error||r.initial_wait_error||r.wait_error
       ||r.initial_endpoints==4||r.endpoints==4||nad1_process_wait_class(r.initial_process_wait)>=3
       ||nad1_process_wait_class(r.process_wait)>=3)return Nad2StopClassification::ObservationUnavailable;
    if((r.before.state!=1&&r.before.state!=3&&r.before.state!=4)
       ||(r.last.state!=1&&r.last.state!=3&&r.last.state!=4)
       ||(r.control_status_available&&r.control_status.state!=1
          &&r.control_status.state!=3&&r.control_status.state!=4))
        return Nad2StopClassification::ObservationUnavailable;
    if(r.last.state==1&&(r.process_wait!=0||r.endpoints!=0))
        return Nad2StopClassification::StoppedProcessOrListenerRemains;
    if(r.control_sent&&!r.control_submitted)return Nad2StopClassification::NotSubmitted;
    if(r.control_submitted&&r.control_status.state==4&&r.before.state==4&&r.last.state==4&&r.progress==0
       &&r.initial_process_wait==258&&r.process_wait==258&&r.initial_endpoints==r.endpoints
       &&r.endpoints!=0)return Nad2StopClassification::SubmittedNoTransition;
    const bool process_retiring=r.initial_process_wait==258&&r.process_wait==0;
    const bool listeners_retiring=r.initial_endpoints<=3&&r.endpoints<=3
        &&r.initial_endpoints!=r.endpoints&&(r.endpoints&r.initial_endpoints)==r.endpoints;
    if((r.control_submitted||r.before.state==3)
       &&(r.control_status.state==3||r.last.state==3||process_retiring||listeners_retiring))
        return Nad2StopClassification::SubmittedProgressing;
    return Nad2StopClassification::ObservationUnavailable;
}
template<class IO> Nad1StopObservation nad1_observe_stop(IO& io, bool generation_valid) {
    Nad1StopObservation r;const auto begin=io.now();
    r.before=r.last=io.query();++r.queries;
    Nad1StopTransition compared{};bool have_compared=false;
    auto record=[&](Nad1StopTransition value){
        if(have_compared&&nad1_same_transition(compared,value))return;
        compared=value;have_compared=true;nad1_record_transition(r,value);
    };
    if(r.before.error||!generation_valid){
        record({1,io.now()-begin,r.before,nad1_process_wait_class(0xffffffffu),0,4});
        r.elapsed=io.now()-begin;r.classification=nad1_classify_stop(r,generation_valid);return r;
    }
    r.initial_process_wait=io.wait(r.initial_wait_error);r.initial_endpoints=io.endpoints();
    r.process_wait=r.initial_process_wait;r.wait_error=r.initial_wait_error;r.endpoints=r.initial_endpoints;
    record({1,io.now()-begin,r.before,nad1_process_wait_class(r.initial_process_wait),r.initial_wait_error,r.initial_endpoints});
    // Pending/stopped observations never send a redundant control. An already
    // pending generation must have been retained while its PID was valid.
    if(r.before.state!=1 && r.before.state!=3){
        r.control_sent=true;r.control_count=1;r.control_started=io.now()-begin;
        io.begin_control(r.before.state);
        auto control=io.stop();r.control_elapsed=io.now()-begin-r.control_started;
        r.control_error=control.error;r.control_submitted=control.submitted;
        if(control.submitted){
            r.control_status=control.status;r.control_status_available=true;
            uint32_t error=0;auto wait=io.wait(error);auto endpoints=io.endpoints();
            record({2,io.now()-begin,control.status,nad1_process_wait_class(wait),error,endpoints});
        }
    }
    // Observe even a rejected/racing control once; never retry the request.
    do {
        auto next=io.query();++r.queries;
        if(next.error){r.last=next;record({1,io.now()-begin,next,nad1_process_wait_class(0xffffffffu),0,4});break;}
        if(next.state!=r.last.state||next.checkpoint!=r.last.checkpoint)++r.progress;
        r.last=next;r.process_wait=io.wait(r.wait_error);r.endpoints=io.endpoints();
        record({1,io.now()-begin,next,nad1_process_wait_class(r.process_wait),r.wait_error,r.endpoints});
        if(next.state==1&&r.process_wait==0&&r.endpoints==0){r.confirmed=true;break;}
        if(r.control_error!=0 && r.control_error!=1062 && r.control_error!=1061)break;
        if(io.now()-begin>=12000)break;
        io.sleep();
    }while(io.now()-begin<12000);
    r.elapsed=io.now()-begin;r.classification=nad1_classify_stop(r,generation_valid);return r;
}
