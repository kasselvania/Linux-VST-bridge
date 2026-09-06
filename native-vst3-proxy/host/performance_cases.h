// Same real SDK consumer for baseline and candidate binaries. No DAW latency
// compensation and no sound-device path. AGain is the exact displacement oracle.
void performanceCase(const VST3::Hosting::PluginFactory& factory,HostApplication*host,
                     bool commercial,double rate,int size,int seconds,int voices) {
 need(size>=1&&size<=1024&&seconds>=2&&seconds<=120&&voices>=1&&voices<=48,"performance arguments");
 auto classes=factory.classInfos();auto component=factory.createInstance<IComponent>(classes[0].ID());need(bool(component),"performance component");
 ok(component->initialize(host),"performance initialize");FUnknownPtr<IAudioProcessor> p(component);need(bool(p),"performance processor");
 auto controller=factory.createInstance<IEditController>(classes[1].ID());need(bool(controller),"performance controller");ok(controller->initialize(host),"performance controller initialize");
 FUnknownPtr<IConnectionPoint> cp(component),cc(controller);need(cp&&cc,"performance connections");ok(cp->connect(cc),"performance connect");ok(cc->connect(cp),"performance readback");
 SpeakerArrangement stereo=SpeakerArr::kStereo;ok(p->setBusArrangements(commercial?nullptr:&stereo,commercial?0:1,&stereo,1),"performance buses");
 ProcessSetup setup{kRealtime,kSample32,size,rate};auto configured=p->setupProcessing(setup);
 std::cout<<"{\"event\":\"ap9_host_setup\",\"sample_rate\":"<<rate<<",\"host_block\":"<<size<<",\"setup_result\":"<<configured<<",\"native_float32\":"<<(p->canProcessSampleSize(kSample32)==kResultTrue)<<",\"native_float64\":"<<(p->canProcessSampleSize(kSample64)==kResultTrue)<<",\"reported_latency\":"<<p->getLatencySamples()<<"}"<<std::endl;
 if(configured!=kResultOk){cc->disconnect(cp);cp->disconnect(cc);cc=nullptr;cp=nullptr;component->terminate();p=nullptr;component=nullptr;controller->terminate();return;}
 const auto delay=p->getLatencySamples();need(delay<8192,"delay extent");
 if(commercial)ok(component->activateBus(kEvent,kInput,0,true),"performance note bus");else ok(component->activateBus(kAudio,kInput,0,true),"performance input");
 ok(component->activateBus(kAudio,kOutput,0,true),"performance output");ok(component->setActive(true),"performance activate");
 std::exception_ptr failure;uint64_t compared=0,wrong=0,nonzero=0,returned_zero=0,deadline=0,late=0,rejections=0,callback_max=0;double energy=0,peak=0,max_error=0;std::vector<uint64_t> durations;durations.reserve(size_t(rate*seconds/size)+1);
 rusage before{},after{};double wall=0;uint64_t position=0;std::array<double,120> per_second_energy{};std::array<uint64_t,120> per_second_samples{};
 std::thread audio([&]{try {
  std::array<std::array<float,1026>,2> input{},output{};std::array<float*,2> in{input[0].data()+1,input[1].data()+1},out{output[0].data()+1,output[1].data()+1};
  AudioBusBuffers ib{},ob{};ib.numChannels=ob.numChannels=2;ib.channelBuffers32=in.data();ob.channelBuffers32=out.data();
  ParameterChanges parameters(1);EventList notes(128);ProcessData data{};data.processMode=kRealtime;data.symbolicSampleSize=kSample32;data.numSamples=size;data.numInputs=commercial?0:1;data.numOutputs=1;data.inputs=commercial?nullptr:&ib;data.outputs=&ob;data.inputParameterChanges=&parameters;data.inputEvents=&notes;
  std::array<std::array<float,8192>,2> expected{};uint32_t random=0x951ac;double gain=.5;
  ok(callback([&]{return p->setProcessing(true);}),"performance start");auto start=Clock::now();getrusage(RUSAGE_SELF,&before);std::cout<<"{\"event\":\"ap9_audio_begin\"}"<<std::endl;
  const uint64_t blocks=uint64_t(rate*seconds)/size,cycle=std::max<uint64_t>(8,uint64_t(rate*2)/size);
  for(uint64_t b=0;b<blocks;++b){
   parameters.clearQueue();notes.clear();
   if(commercial){
    // Fixed initialized patch. A chord every two seconds, one-second hold,
    // then a full second for the release. Stable IDs and nonzero offsets.
    auto phase=b%cycle;
    if(phase==2||phase==cycle/2){for(int voice=0;voice<voices;++voice){Event e{};e.busIndex=0;e.sampleOffset=phase==2?std::min(7,size-1):size-1;e.type=phase==2?Event::kNoteOnEvent:Event::kNoteOffEvent;
     if(e.type==Event::kNoteOnEvent){e.noteOn.channel=0;e.noteOn.pitch=36+voice;e.noteOn.velocity=.65f;e.noteOn.noteId=voice+1;}else{e.noteOff.channel=0;e.noteOff.pitch=36+voice;e.noteOff.velocity=.2f;e.noteOff.noteId=voice+1;}ok(notes.addEvent(e),"performance note");}}
    if(b==0||voices>1){int32 i=0,j=0;auto*q=parameters.addParameterData(0,i);q->addPoint(0,.15,j);if(voices>1)q->addPoint(size-1,.15+double(b%8)*.002,j);}
   }else if(b%std::max<uint64_t>(1,cycle/4)==0){gain=gain==.5?.25:.5;int32 i=0,j=0;parameters.addParameterData(0,i)->addPoint(0,gain,j);}
   for(int ch=0;ch<2;++ch){input[ch].fill(0);output[ch].fill(std::bit_cast<float>(uint32_t(0x7fc12345)));input[ch].front()=input[ch][size+1]=output[ch].front()=output[ch][size+1]=12345.f;
    for(int i=0;i<size;++i){random=random*1664525+1013904223;in[ch][i]=commercial?0.f:float(int(random%31)-15)/32.f;expected[ch][(position+i)%8192]=in[ch][i]*float(gain);}}
   auto scheduled=start+std::chrono::nanoseconds(uint64_t(double(position)*1e9/rate));std::this_thread::sleep_until(scheduled);auto began=Clock::now();auto allowance=std::chrono::nanoseconds(uint64_t(size*1e9/rate));late+=began>scheduled+allowance;
   auto result=callback([&]{return p->process(data);});auto ns=std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now()-began).count();durations.push_back(uint64_t(ns));callback_max=std::max(callback_max,uint64_t(ns));deadline+=ns>allowance.count();rejections+=result!=kResultOk;
   for(int ch=0;ch<2;++ch){need(output[ch].front()==12345.f&&output[ch][size+1]==12345.f,"performance output guard");for(int i=0;i<size;++i){double v=out[ch][i];need(std::isfinite(v),"performance finite output");energy+=v*v;peak=std::max(peak,std::abs(v));nonzero+=v!=0;auto second=std::min<size_t>(119,size_t(double(position+i)/rate));per_second_energy[second]+=v*v;per_second_samples[second]++;
    if(!commercial){float want=position+i<delay?0.f:expected[ch][(position+i-delay)%8192];double error=std::abs(v-want);wrong+=error!=0;returned_zero+=error!=0&&v==0;max_error=std::max(max_error,error);++compared;}}}
   position+=size;
  }
  getrusage(RUSAGE_SELF,&after);wall=std::chrono::duration<double>(Clock::now()-start).count();ok(callback([&]{return p->setProcessing(false);}),"performance stop");
 }catch(...){failure=std::current_exception();}});audio.join();
 if(!failure){LVBState::Stream state;ok(component->getState(&state),"performance capture");ok(component->setActive(false),"performance deactivate");state.position=0;ok(component->setState(&state),"performance restore");std::cout<<"{\"event\":\"ap9_state_roundtrip\",\"bytes\":"<<state.bytes.size()<<"}"<<std::endl;}
 cc->disconnect(cp);cp->disconnect(cc);cc=nullptr;cp=nullptr;auto terminated=component->terminate();p=nullptr;component=nullptr;controller->terminate();controller=nullptr;
 if(failure)std::rethrow_exception(failure);ok(terminated,"performance terminate");
 std::sort(durations.begin(),durations.end());auto pct=[&](double q){return durations[std::min(durations.size()-1,size_t(q*durations.size()))];};auto cpu=[](const rusage&r){return double(r.ru_utime.tv_sec+r.ru_stime.tv_sec)+double(r.ru_utime.tv_usec+r.ru_stime.tv_usec)/1e6;};
 std::cout<<"{\"event\":\"ap9_host_result\",\"commercial\":"<<commercial<<",\"voices\":"<<voices<<",\"sample_rate\":"<<rate<<",\"block\":"<<size<<",\"latency_frames\":"<<delay<<",\"frames\":"<<position<<",\"wall_seconds\":"<<wall<<",\"native_cpu_seconds\":"<<cpu(after)-cpu(before)<<",\"maxrss_kib\":"<<after.ru_maxrss<<",\"callback_p50_ns\":"<<pct(.5)<<",\"callback_p99_ns\":"<<pct(.99)<<",\"callback_max_ns\":"<<callback_max<<",\"callback_deadline_misses\":"<<deadline<<",\"host_schedule_misses\":"<<late<<",\"rejections\":"<<rejections<<",\"compared_samples\":"<<compared<<",\"wrong_samples\":"<<wrong<<",\"wrong_zero_samples\":"<<returned_zero<<",\"maximum_error\":"<<max_error<<",\"rms\":"<<sqrt(energy/double(position*2))<<",\"peak\":"<<peak<<",\"nonzero_samples\":"<<nonzero<<",\"callback_effects\":0}"<<std::endl;
 for(int i=0;i<seconds;++i)std::cout<<"{\"event\":\"ap9_host_audio_second\",\"second\":"<<i<<",\"rms\":"<<sqrt(per_second_energy[i]/double(std::max<uint64_t>(1,per_second_samples[i])))<<"}"<<std::endl;
}
