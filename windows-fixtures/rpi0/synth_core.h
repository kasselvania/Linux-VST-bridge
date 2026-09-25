#pragma once
#include <array>
#include <cmath>
#include <cstdint>
#include <cstring>

namespace lvb::rpi0 {
constexpr uint32_t gain_parameter = 0x52500001u;
constexpr uint32_t sustain_parameter_base = 0x52500100u;
constexpr uint32_t all_notes_off_parameter_base = 0x52500200u;
constexpr uint32_t state_magic = 0x30504952u;
constexpr uint32_t state_version = 1;
constexpr size_t polyphony = 16;
constexpr uint32_t audio_inputs = 0, audio_outputs = 1, event_inputs = 1, event_outputs = 0;

struct State { uint32_t magic=state_magic, version=state_version; double gain=.5; };
static_assert(sizeof(State)==16);

class SynthCore {
  struct Voice { double phase=0., increment=0.; float level=0.f, envelope=0.f; int32_t id=-1; uint8_t channel=0, pitch=0; uint64_t age=0; bool active=false, held=false; };
  std::array<Voice,polyphony> voices_{};
  std::array<float,2048> table_{};
  std::array<bool,16> sustain_{};
  double sample_rate_=48000., gain_=.5;
  uint64_t age_=0;
  static constexpr double reference=440.0;
  Voice* find(int32_t id,uint8_t channel,uint8_t pitch){
    for(auto&v:voices_)if(v.active&&v.channel==channel&&((id>=0&&v.id==id)||(id<0&&v.pitch==pitch)))return &v;
    return nullptr;
  }
 public:
  SynthCore(){for(size_t i=0;i<table_.size();++i){double p=double(i)/table_.size();table_[i]=float(4.0*std::abs(p-std::floor(p+.5))-1.0);}}
  void reset(double rate){sample_rate_=rate;voices_={};sustain_={};age_=0;}
  void gain(double value){if(std::isfinite(value)&&value>=0.&&value<=1.)gain_=value;}
  double gain()const{return gain_;}
  size_t active()const{size_t n=0;for(auto&v:voices_)n+=v.active?1:0;return n;}
  void note_on(uint8_t channel,uint8_t pitch,float velocity,int32_t id){
    Voice* selected=nullptr;for(auto&v:voices_)if(!v.active){selected=&v;break;}
    if(!selected){selected=&voices_[0];for(auto&v:voices_)if(v.age<selected->age)selected=&v;}
    const double hz=reference*std::pow(2.0,(int(pitch)-69)/12.0);
    *selected={0.,hz*table_.size()/sample_rate_,velocity,1.f,id,channel,pitch,++age_,true,true};
  }
  void note_off(uint8_t channel,uint8_t pitch,int32_t id){if(auto*v=find(id,channel,pitch)){v->held=false;if(!sustain_[channel])v->envelope=std::min(v->envelope,.999f);}}
  void sustain(uint8_t channel,bool on){sustain_[channel]=on;if(!on)for(auto&v:voices_)if(v.active&&v.channel==channel&&!v.held)v.envelope=std::min(v.envelope,.999f);}
  void all_notes_off(uint8_t channel){sustain_[channel]=false;for(auto&v:voices_)if(v.active&&v.channel==channel){v.held=false;v.envelope=std::min(v.envelope,.999f);}}
  void sample(float&left,float&right){
    float value=0.f;for(auto&v:voices_)if(v.active){
      auto index=size_t(v.phase)&(table_.size()-1);value+=table_[index]*v.level*v.envelope;
      v.phase+=v.increment;if(v.phase>=table_.size())v.phase-=table_.size();
      if(!v.held&&!sustain_[v.channel]){v.envelope*=.995f;if(v.envelope<.0001f)v.active=false;}
    }
    left=right=value*float(gain_*.12);
  }
  State state()const{return {state_magic,state_version,gain_};}
  bool restore(const State&s){if(s.magic!=state_magic||s.version!=state_version||!std::isfinite(s.gain)||s.gain<0||s.gain>1)return false;gain_=s.gain;return true;}
};
}
