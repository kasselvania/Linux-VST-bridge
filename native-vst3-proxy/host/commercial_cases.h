// Focused standard-SDK consumer of the commercial native proxy. No Windows DSP
// implementation, state decoder or AGain sample oracle lives in this host.
void commercialCase(const VST3::Hosting::PluginFactory& factory,HostApplication*host) {
 auto classes=factory.classInfos();auto component=factory.createInstance<IComponent>(classes[0].ID());
 need(bool(component),"commercial component");ok(component->initialize(host),"commercial initialize");
 FUnknownPtr<IAudioProcessor> p(component);need(bool(p),"commercial processor");
 auto controller=factory.createInstance<IEditController>(classes[1].ID());need(bool(controller),"commercial controller");ok(controller->initialize(host),"controller initialize");
 FUnknownPtr<IConnectionPoint> cp(component),cc(controller);need(cp&&cc,"controller connections");ok(cp->connect(cc),"component connect");ok(cc->connect(cp),"controller connect/readback");
 ParameterInfo volume{};ok(controller->getParameterInfo(0,volume),"first actual parameter");
 need(component->getBusCount(kAudio,kInput)==0&&component->getBusCount(kEvent,kInput)==1,"instrument buses");
 SpeakerArrangement output=SpeakerArr::kStereo;ok(p->setBusArrangements(nullptr,0,&output,1),"instrument arrangement");
 ProcessSetup setup{kRealtime,kSample32,256,48000.};ok(p->setupProcessing(setup),"instrument setup");
 ok(component->activateBus(kEvent,kInput,0,true),"notes active");ok(component->activateBus(kAudio,kOutput,0,true),"output active");
 for(int i=0;i<component->getBusCount(kEvent,kOutput);++i)ok(component->activateBus(kEvent,kOutput,i,true),"declared event output activation");
 need(p->getLatencySamples()==1024,"unchanged bridge latency");ok(component->setActive(true),"Windows instrument activate");
 std::vector<float> returned;returned.reserve(384*256);uint64_t rejections=0;std::exception_ptr failure;
 std::thread audio([&]{try{
  Block block;EventList notes(4);block.data.numInputs=0;block.data.inputs=nullptr;block.data.inputEvents=&notes;
  ok(callback([&]{return p->setProcessing(true);}),"instrument start");auto start=Clock::now();
  for(int b=0;b<384;++b){block.prepare(256,.25,b==144,false,0);notes.clear();
   if(b==4||b==98||b==144||b==242){Event e{};e.busIndex=0;e.sampleOffset=b==4?7:b==144?19:31;e.type=(b==4||b==144)?Event::kNoteOnEvent:Event::kNoteOffEvent;
    if(e.type==Event::kNoteOnEvent){e.noteOn.channel=2;e.noteOn.pitch=b==4?60:67;e.noteOn.velocity=.75f;e.noteOn.noteId=b==4?42:43;}
    else{e.noteOff.channel=2;e.noteOff.pitch=b==98?60:67;e.noteOff.velocity=.25f;e.noteOff.noteId=b==98?42:43;}
    ok(notes.addEvent(e),"test note");
   }
   if(b==144){block.parameters.clearQueue();int32 i=0,j=0;block.parameters.addParameterData(volume.id,i)->addPoint(0,.25,j);}
   std::this_thread::sleep_until(start+std::chrono::nanoseconds(uint64_t(b)*256*1000000000ULL/48000));
   auto r=callback([&]{return p->process(block.data);});rejections+=r!=kResultOk;ok(r,"commercial process");
   for(int i=0;i<256;++i){need(std::isfinite(block.out[0][i])&&std::isfinite(block.out[1][i]),"finite instrument output");returned.push_back(block.out[0][i]);}
  }
  ok(callback([&]{return p->setProcessing(false);}),"instrument stop");
 }catch(...){failure=std::current_exception();}});audio.join();
 if(!failure){
  LVBState::Stream captured;ok(component->getState(&captured),"opaque capture");captured.position=0;ok(controller->setComponentState(&captured),"actual control synchronization");
  need(std::abs(controller->getParamNormalized(volume.id)-.25)<1e-6,"Windows parameter capture");
  ok(component->setActive(false),"instrument deactivate");captured.position=0;ok(component->setState(&captured),"opaque restore");
  need(std::abs(controller->getParamNormalized(volume.id)-.25)<1e-6,"Windows restored parameter readback");
  auto measure=[&](size_t from,size_t to){double sum=0,peak=0;uint64_t nz=0,crossings=0;for(size_t i=from;i<to;++i){double v=returned[i];sum+=v*v;peak=std::max(peak,std::abs(v));nz+=v!=0;if(i>from&&returned[i-1]<=0&&v>0)++crossings;}std::cout<<"{\"event\":\"ap8_audio_window\",\"from\":"<<from<<",\"to\":"<<to<<",\"rms\":"<<std::sqrt(sum/double(to-from))<<",\"peak\":"<<peak<<",\"nonzero\":"<<nz<<",\"positive_crossings\":"<<crossings<<"}"<<std::endl;return peak;};
  auto first=measure(8192,22016),second=measure(40960,59392),released=measure(90112,98304);
  std::cout<<"{\"event\":\"ap8_host_state\",\"bytes\":"<<captured.bytes.size()<<",\"parameter_id\":"<<volume.id<<",\"restored_value\":"<<controller->getParamNormalized(volume.id)<<",\"rejected_callbacks\":"<<rejections<<",\"callback_effects\":0}"<<std::endl;
  need(first>0&&second>0,"notes must generate real audio");need(released<.00001,"note release must become quiet");
 }
 ok(cc->disconnect(cp),"controller disconnect");ok(cp->disconnect(cc),"component disconnect");cc=nullptr;cp=nullptr;
 component->terminate();p=nullptr;component=nullptr;ok(controller->terminate(),"controller terminate");controller=nullptr;
 if(failure)std::rethrow_exception(failure);
}
