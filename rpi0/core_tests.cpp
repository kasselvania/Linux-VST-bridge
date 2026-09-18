#include "../windows-fixtures/rpi0/synth_core.h"
#include <cmath>
#include <cstdlib>
#include <iostream>
using lvb::rpi0::SynthCore;
void require(bool value,const char*reason){if(!value){std::cerr<<reason<<'\n';std::exit(1);}}
int main(){
  SynthCore synth;synth.reset(48000);float left=0,right=0;
  for(int i=0;i<64;++i){synth.sample(left,right);require(left==0&&right==0,"pre-note silence");}
  synth.note_on(3,60,.25f,1);for(int i=0;i<128;++i)synth.sample(left,right);const float quiet=std::abs(left);require(quiet>0&&left==right,"stereo note");
  SynthCore loud;loud.reset(48000);loud.note_on(3,60,1.f,1);for(int i=0;i<128;++i)loud.sample(left,right);require(std::abs(left)>quiet,"velocity");
  loud.sustain(3,true);loud.note_off(3,60,1);for(int i=0;i<4096;++i)loud.sample(left,right);require(loud.active()==1,"sustain");loud.sustain(3,false);for(int i=0;i<4096;++i)loud.sample(left,right);require(loud.active()==0,"release");
  loud.note_on(1,64,1,2);loud.note_on(2,67,1,3);loud.all_notes_off(1);for(int i=0;i<4096;++i)loud.sample(left,right);require(loud.active()==1,"channel all-notes-off");
  loud.gain(.25);auto state=loud.state();loud.gain(.75);require(loud.restore(state)&&loud.gain()==.25,"state");
  std::cout<<"RPI0 portable synth core: 7 passed\n";return 0;
}
