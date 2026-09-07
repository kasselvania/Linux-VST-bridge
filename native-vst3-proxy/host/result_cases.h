// SDK-produced output crosses the real Windows host/mailbox/queue/native callback.
// Assertions are on this consumer's sinks, never on a fake transport peer.
void resultCase(const VST3::Hosting::PluginFactory&factory,HostApplication*host,const std::string&scenario){
 const bool null_sink=scenario=="ap10-null",inactive=scenario=="ap10-inactive",overflow=scenario=="ap10-overflow",reject_sink=scenario=="ap10-reject";
 auto classes=factory.classInfos();auto component=factory.createInstance<IComponent>(classes[0].ID());need(bool(component),"return component");ok(component->initialize(host),"return initialize");FUnknownPtr<IAudioProcessor> p(component);need(bool(p),"return processor");
 auto controller=factory.createInstance<IEditController>(classes[1].ID());need(bool(controller),"return controller");ok(controller->initialize(host),"return controller initialize");
 FUnknownPtr<IConnectionPoint> cp(component),cc(controller);need(cp&&cc,"return connection");ok(cp->connect(cc),"return connect");ok(cc->connect(cp),"return readback");
 need(component->getBusCount(kEvent,kOutput)==1,"declared event output");BusInfo info{};ok(component->getBusInfo(kEvent,kOutput,0,info),"event output info");need(!(info.flags&BusInfo::kDefaultActive),"fixture output is not default active");
 SpeakerArrangement stereo=SpeakerArr::kStereo;ok(p->setBusArrangements(&stereo,1,&stereo,1),"return stereo");ProcessSetup setup{kRealtime,kSample32,512,48000.};ok(p->setupProcessing(setup),"return setup");
 for(int direction=0;direction<2;++direction){ok(component->activateBus(kAudio,direction,0,true),"audio activation");ok(component->activateBus(kEvent,direction,0,direction==kInput||!inactive),"explicit event activation");}
 const uint64_t bridge=p->getLatencySamples()-13;need(bridge==512,"bridge delay excludes vendor 13");ok(component->setActive(true),"return activate");
 struct Seen{ap10_event_result_t e;uint64_t absolute;std::array<uint8_t,512> payload;};std::vector<Seen> seen;seen.reserve(128);uint64_t point_count=0,late_off=0,position=0;bool saw_failure=false,flush_feedback=false;std::exception_ptr failure;
 std::array<uint64_t,10> expected{};uint64_t on_due=0,off_due=0;
 std::thread audio([&]{try{
  AP10Results::Collector sink;sink.buses=1;sink.channels[0]=16;
  std::array<std::array<float,512>,2> samples{};std::array<float*,2> channels{samples[0].data(),samples[1].data()};AudioBusBuffers ib{},ob{};ib.numChannels=ob.numChannels=2;ib.channelBuffers32=ob.channelBuffers32=channels.data();
  ParameterChanges input(2);ProcessData data{};data.processMode=kRealtime;data.symbolicSampleSize=kSample32;data.numInputs=data.numOutputs=1;data.inputs=&ib;data.outputs=&ob;data.inputParameterChanges=&input;
  data.outputEvents=null_sink?nullptr:&sink;data.outputParameterChanges=null_sink?nullptr:&sink;
  auto capture=[&](uint64_t at){for(uint32_t i=0;i<sink.values.events;++i){const auto&e=sink.values.event[i];Seen v{e,at+uint64_t(e.offset),{}};std::copy_n(sink.values.payload+e.payload_offset,e.payload_size,v.payload.data());seen.push_back(v);}point_count+=sink.values.points;};
  auto command=[&](double value,int offset=0){int32 i=0,j=0;input.addParameterData(1,i)->addPoint(offset,value,j);};
  ok(callback([&]{return p->setProcessing(true);}),"return start");auto start=Clock::now();
  for(int b=0;b<112;++b){
   int n=std::array<int,6>{128,64,257,511,384,256}[size_t(b)%6];input.clearQueue();
   if(b==24){n=511;command(overflow?.4:0.,270);for(int i=0;i<10;++i)expected[i]=position+270+uint64_t(i)+bridge;}
   if(b==48&&!overflow&&!null_sink&&!inactive&&!reject_sink){command(.1);on_due=position+5+bridge;}
   if(b==49&&!overflow&&!null_sink&&!inactive&&!reject_sink){command(.2);off_due=position+7+bridge;}
   if(b==80&&!overflow&&!null_sink&&!inactive&&!reject_sink)command(.7);
   data.numSamples=n;sink.reset(n);if(reject_sink&&b>=24)sink.failed=true;
   for(auto&ch:samples)ch.fill(.125f);
   std::this_thread::sleep_until(start+std::chrono::nanoseconds(position*1000000000ULL/48000));
   auto result=callback([&]{return p->process(data);});capture(position);position+=uint64_t(n);
   if(result!=kResultOk){saw_failure=true;break;}
   if(b==64&&!null_sink&&!inactive){
    // Zero-frame feedback must be drainable without advancing the sample clock.
    ProcessData flush{};flush.processMode=kRealtime;flush.symbolicSampleSize=kSample32;flush.inputParameterChanges=&input;flush.outputParameterChanges=&sink;flush.outputEvents=&sink;
    input.clearQueue();command(.6);sink.reset(0);ok(callback([&]{return p->process(flush);}),"zero-frame request");input.clearQueue();
    for(int j=0;j<20&&!flush_feedback;++j){std::this_thread::sleep_for(std::chrono::milliseconds(1));sink.reset(0);ok(callback([&]{return p->process(flush);}),"zero-frame result");flush_feedback=sink.values.points>0;capture(position);}
    // Restart pacing after the deliberately non-advancing flush window.
    start=Clock::now()-std::chrono::nanoseconds(position*1000000000ULL/48000);
   }
  }
  ok(callback([&]{return p->setProcessing(false);}),"return stop");
  if(!saw_failure){
   // Required cleanup is emitted in the first valid callback after a restart.
   ok(callback([&]{return p->setProcessing(true);}),"return restart");input.clearQueue();data.numSamples=128;sink.reset(128);auto r=callback([&]{return p->process(data);});capture(0);ok(r,"return restart cleanup");ok(callback([&]{return p->setProcessing(false);}),"return restop");
  }
 }catch(...){failure=std::current_exception();}});audio.join();
 if(!failure&&!saw_failure){LVBState::Stream state;ok(component->getState(&state),"return state");ok(component->setActive(false),"return deactivate");state.position=0;ok(component->setState(&state),"return state restore");}
 cc->disconnect(cp);cp->disconnect(cc);cc=nullptr;cp=nullptr;auto closed=component->terminate();p=nullptr;component=nullptr;controller->terminate();controller=nullptr;
 if(failure)std::rethrow_exception(failure);
 if(overflow||reject_sink){need(saw_failure,"expected explicit result failure");}
 else{
  ok(closed,"return close");need(!saw_failure,"healthy result path");
  if(null_sink){need(seen.empty()&&point_count==0,"null sinks receive nothing");}
  else if(inactive){need(seen.empty()&&point_count>0,"inactive events retain parameter feedback");}
  else{
   need(seen.size()==14,"ten variants plus on/off and stop cleanup");
   uint16_t kinds[]={0,1,2,3,4,5,6,7,8,65535};
   for(int i=0;i<10;++i){const auto&v=seen[size_t(i)];need(v.e.kind==kinds[i]&&v.e.bus==0&&v.e.flags==0xc001&&v.e.ppq==7.25,"returned variant metadata");need(v.absolute==expected[i],"P+s+D once across chunked/variable blocks");}
   need(seen[0].e.a==-1234&&seen[1].e.a==-1234,"signed note identity");need(seen[2].payload[1]==1&&seen[5].payload[0]=='A'&&seen[6].payload[0]=='A'&&seen[7].payload[0]=='A',"owned temporary pointer payloads");need(seen[8].e.value==0xfedcba9876543210ULL,"integer expression exact bits");
   need(seen[10].e.kind==0&&seen[10].absolute>=on_due,"held note delivered");need(seen[11].e.kind==1&&seen[11].absolute>off_due&&seen[11].e.offset==0,"late note-off delivered after expired audio");late_off=seen[11].absolute-off_due;
   need(seen[12].e.kind==0&&seen[13].e.kind==1&&seen[13].absolute==0,"bounded stop cleanup for handed-down note");need(flush_feedback&&point_count>=4,"separate parameter feedback and zero-frame flush");
  }
 }
 std::cout<<"{\"event\":\"ap10_sdk_return_result\",\"scenario\":\""<<scenario<<"\",\"events\":"<<seen.size()<<",\"parameter_points\":"<<point_count<<",\"late_note_off_frames\":"<<late_off<<",\"zero_frame_feedback\":"<<flush_feedback<<",\"explicit_failure\":"<<saw_failure<<",\"callback_effects\":0}"<<std::endl;
}
