#pragma once
#include <windows.h>
#include <array>
#include <algorithm>
#include <cstdint>
#include <string>
#include "linux_vst_bridge/wf0_probe/events.h"
namespace linux_vst_bridge::wf0 {
// Private, opt-in diagnostics. Fixed storage; CPU queries occur only with the
// trace enabled. File output is drained after the delivery thread has joined.
struct DeliveryTrace {
    struct Cpu {
        uint64_t thread_user=UINT64_MAX,thread_kernel=UINT64_MAX;
        uint64_t process_user=UINT64_MAX,process_kernel=UINT64_MAX,cycles=UINT64_MAX;
    };
    static uint64_t ticks(FILETIME value){return (uint64_t(value.dwHighDateTime)<<32)|value.dwLowDateTime;}
    static Cpu cpu(){
        Cpu result{};FILETIME created{},exited{},kernel{},user{};
        if(GetThreadTimes(GetCurrentThread(),&created,&exited,&kernel,&user)){
            result.thread_user=ticks(user);result.thread_kernel=ticks(kernel);
        }
        if(GetProcessTimes(GetCurrentProcess(),&created,&exited,&kernel,&user)){
            result.process_user=ticks(user);result.process_kernel=ticks(kernel);
        }
        ULONG64 cycles=0;
        if(QueryThreadCycleTime(GetCurrentThread(),&cycles))result.cycles=cycles;
        return result;
    }
    struct Row {
        uint64_t epoch=0,sequence=0,position=0;std::array<uint64_t,8> at{};
        uint32_t event_count=0,note_on_count=0,note_offset=0,note_id=0;
        int32_t note_pitch=-1,note_channel=-1;
        uint32_t caller_tid=0;
        Cpu before{},after{};
    };
    bool enabled=false,armed=false,dumped=false;
    uint64_t frequency=0,min_tick=0;
    Row current{};
    std::array<Row,64> recent{};
    std::array<Row,384> retained{};
    std::array<Row,2> note_rows{};
    size_t note_count=0;
    size_t cursor=0,available=0,count=0,following=0,triggers=0;
    DeliveryTrace(){
        wchar_t value[2]{};enabled=GetEnvironmentVariableW(L"LVB_AP10_TRACE",value,2)==1&&value[0]==L'1';
        if(!enabled)return;
        LARGE_INTEGER f{};if(!QueryPerformanceFrequency(&f)||f.QuadPart<=0){enabled=false;return;}
        frequency=uint64_t(f.QuadPart);auto previous=now();min_tick=UINT64_MAX;
        for(int i=0;i<1024;++i){auto next=now();if(next>previous)min_tick=std::min(min_tick,next-previous);previous=next;}
    }
    uint64_t now()const{if(!enabled)return 0;LARGE_INTEGER t{};QueryPerformanceCounter(&t);return uint64_t(t.QuadPart);}
    void stamp(size_t i){if(enabled)current.at[i]=now();}
    void complete(){
        if(!enabled||!armed)return;
        if(current.note_on_count&&note_count<note_rows.size())note_rows[note_count++]=current;
        const bool slow=current.at[7]-current.at[1]>frequency/200 || current.at[1]-current.at[0]>frequency*6/1000;
        if(!following&&slow&&count+available+33<=retained.size()){
            ++triggers;
            for(size_t i=0;i<available;++i)retained[count++]=recent[(cursor+recent.size()-available+i)%recent.size()];
            following=33;
        }
        if(following){retained[count++]=current;--following;}
        recent[cursor]=current;cursor=(cursor+1)%recent.size();available=std::min(available+1,recent.size());
    }
    void dump(EventWriter& events){
        if(!enabled||dumped)return;dumped=true;
        events.lifecycle("ap10_windows_clock",",\"frequency\":"+std::to_string(frequency)+",\"minimum_observed_tick\":"+std::to_string(min_tick)+",\"triggers\":"+std::to_string(triggers)+",\"retained\":"+std::to_string(count));
        for(size_t i=0;i<count;++i){const auto&r=retained[i];std::string times="[";
            for(size_t j=0;j<r.at.size();++j){if(j)times+=",";times+=std::to_string(r.at[j]);}times+="]";
            events.lifecycle("ap10_windows_request",",\"epoch\":"+std::to_string(r.epoch)+",\"request_sequence\":"+std::to_string(r.sequence)+",\"position\":"+std::to_string(r.position)+",\"qpc\":"+times);
        }
        for(size_t i=0;i<note_count;++i){const auto&r=note_rows[i];
            auto delta=[](uint64_t before,uint64_t after){return before==UINT64_MAX||after==UINT64_MAX||after<before?UINT64_MAX:after-before;};
            events.lifecycle("fn1_note_process",",\"ordinal\":"+std::to_string(i+1)+
                ",\"epoch\":"+std::to_string(r.epoch)+",\"request_sequence\":"+std::to_string(r.sequence)+
                ",\"position\":"+std::to_string(r.position)+",\"event_count\":"+std::to_string(r.event_count)+
                ",\"note_on_count\":"+std::to_string(r.note_on_count)+",\"note_offset\":"+std::to_string(r.note_offset)+
                ",\"note_id\":"+std::to_string(r.note_id)+",\"pitch\":"+std::to_string(r.note_pitch)+
                ",\"channel\":"+std::to_string(r.note_channel)+",\"caller_tid\":"+std::to_string(r.caller_tid)+
                ",\"process_begin_qpc\":"+std::to_string(r.at[3])+",\"process_end_qpc\":"+std::to_string(r.at[4])+
                ",\"qpc_frequency\":"+std::to_string(frequency)+
                ",\"caller_user_100ns\":"+std::to_string(delta(r.before.thread_user,r.after.thread_user))+
                ",\"caller_kernel_100ns\":"+std::to_string(delta(r.before.thread_kernel,r.after.thread_kernel))+
                ",\"process_user_100ns\":"+std::to_string(delta(r.before.process_user,r.after.process_user))+
                ",\"process_kernel_100ns\":"+std::to_string(delta(r.before.process_kernel,r.after.process_kernel))+
                ",\"caller_cycles\":"+std::to_string(delta(r.before.cycles,r.after.cycles)));
        }
    }
};
}
