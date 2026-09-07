#pragma once
#include <windows.h>
#include <array>
#include <algorithm>
#include <cstdint>
#include <string>
#include "linux_vst_bridge/wf0_probe/events.h"
namespace linux_vst_bridge::wf0 {
// Private, opt-in diagnostics. Fixed storage; no file/log/CPU-query work on the
// delivery thread. Output is drained only after that thread has joined.
struct DeliveryTrace {
    struct Row {uint64_t epoch=0,sequence=0,position=0;std::array<uint64_t,8> at{};};
    bool enabled=false,armed=false,dumped=false;
    uint64_t frequency=0,min_tick=0;
    Row current{};
    std::array<Row,64> recent{};
    std::array<Row,384> retained{};
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
            events.lifecycle("ap10_windows_request",",\"epoch\":"+std::to_string(r.epoch)+",\"sequence\":"+std::to_string(r.sequence)+",\"position\":"+std::to_string(r.position)+",\"qpc\":"+times);
        }
    }
};
}
