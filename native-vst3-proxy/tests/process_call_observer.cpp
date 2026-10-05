#include "process_call_observer.h"
#include <algorithm>
#include <array>
#include <cstdio>
#include <dlfcn.h>
#include <thread>
#include <vector>
using namespace LVBCallTiming;
namespace LVBCallTiming {
struct RecorderTestAccess {
 static void next(Recorder& r,uint64_t value){r.offered_.store(value);}
 static void exhaustInstances(){Recorder::instances_.store(UINT64_MAX);}
};
}
static void require(bool ok,const char* why){if(!ok){std::fprintf(stderr,"FAIL: %s\n",why);std::exit(1);}}
int main(){
 auto begin=reinterpret_cast<void(*)()>(dlsym(RTLD_DEFAULT,"ap3_audit_begin"));
 auto end=reinterpret_cast<uint64_t(*)()>(dlsym(RTLD_DEFAULT,"ap3_audit_end"));
 require(begin&&end,"existing audit preload");
 Recorder disabled(false,2);begin();{Recorder::Call call(disabled,1,0,0,10);call.result(0);}require(end()==0,"disabled callback effects");
 Recorder timed(true,2);
 ap23_windows_process_timing_t wt{1,64,2,3,1,6,7,7,1,0};
 begin();{Recorder::Call call(timed,512,0,0,10);call.windows(wt,0);call.result(0);call.finishAt(20);}
 require(end()==0,"correlated Windows timer adds no audited callback effects");
 {Recorder::Call call(timed,0,0,0,10);call.windows(wt,0);call.windows(wt,0);call.result(0);call.finishAt(20);}
 std::array<lvb_process_call_record_t,2> correlated{};const auto ts=timed.seal();
 require(timed.snapshot(ts,correlated.data(),2),"Windows timing snapshot");
 require((correlated[0].valid&windows_process_valid)&&correlated[0].windows_process_ns==6&&
  correlated[0].windows_requests==1&&correlated[0].windows_first_sequence==7,"exact Windows request retained");
 require(!(correlated[1].valid&windows_process_valid),"repeated/stale Windows reply cannot fabricate correlation");
 require(disabled.seal().offered==0,"disabled does not collect or allocate lazily");
 Recorder small(true,2);
 for(int32_t n:{0,1,64}){begin();{Recorder::Call call(small,n,0,0,10);call.result(0);call.finishAt(20);}require(end()==0,"enabled fixed query/atomics have no audited effects");}
 const auto limited=small.seal();require(limited.offered==3&&limited.retained==2&&limited.capacity_dropped==1&&!(limited.flags&complete),"saturation preserves first records and coverage loss");
 std::array<lvb_process_call_record_t,2> first{};require(small.snapshot(limited,first.data(),2),"partial snapshot safe");
 require(first[0].sequence==0&&first[0].frames==0&&first[1].sequence==1&&first[1].frames==1,"no ring overwrite after saturation");
 require(small.markExported()&&!small.markExported(),"single lifecycle export");
 Recorder refused(true,0);{Recorder::Call call(refused,1,0,0,10);call.result(0);call.finishAt(20);}
 auto s=refused.seal();require(s.offered==1&&s.allocation_dropped==1&&!(s.flags&allocated)&&!(s.flags&complete),"allocation refusal stays explicit");
 Recorder clocks(true,3);for(auto pair:std::array<std::array<uint64_t,2>,3>{{{0,20},{10,0},{10,9}}}){
  Recorder::Call call(clocks,1,0,0,pair[0]);call.result(0);call.finishAt(pair[1]);}
 require(clocks.seal().invalid_clocks==3,"missing/backwards clocks never fabricate durations");
 Recorder overflow(true,2);RecorderTestAccess::next(overflow,UINT64_MAX);
 for(int i=0;i<2;++i){Recorder::Call call(overflow,1,0,0,10);call.result(0);call.finishAt(20);}
 s=overflow.seal();require(s.sequence_overflow==1&&!(s.flags&complete),"ordinal wrap cannot look complete");
 std::array<lvb_process_call_record_t,2> wrapped{};require(overflow.snapshot(s,wrapped.data(),2)&&wrapped[0].sequence==UINT64_MAX&&wrapped[1].sequence==0,"ordinal overflow cannot overwrite storage");
 Recorder held(true,4);std::atomic<bool> entered{false},release{false};
 std::thread writer([&]{Recorder::Call call(held,64,0,0,10);entered.store(true,std::memory_order_release);
  while(!release.load(std::memory_order_acquire))std::this_thread::yield(); // Test-side publication hold, not observer work.
  call.result(0);call.finishAt(20);});
 while(!entered.load(std::memory_order_acquire))std::this_thread::yield();
 s=held.seal();lvb_process_call_record_t record{};
 require(s.unfinished_writers==1&&!(s.flags&complete)&&!held.snapshot(s,&record,1),"seal cannot snapshot unfinished writer");
 {Recorder::Call late(held,1,0,0,10);late.result(0);}release.store(true,std::memory_order_release);writer.join();
 s=held.seal();require(s.unfinished_writers==0&&s.after_seal==1&&!(s.flags&complete)&&held.snapshot(s,&record,1),"closed registration never races writer publication");
 Recorder concurrent(true,128);std::array<std::thread,8> threads;
 std::atomic<unsigned> turn{0};std::atomic<bool> concurrent_start{false};
 for(unsigned index=0;index<threads.size();++index)threads[index]=std::thread([&,index]{
  // One uncontended call per thread authenticates every caller deterministically;
  // subsequent calls deliberately exercise bounded concurrent reservations.
  while(turn.load(std::memory_order_acquire)!=index)std::this_thread::yield();
  begin();{Recorder::Call call(concurrent,1,0,0,10);call.result(0);call.finishAt(20);}require(end()==0,"first caller fixed effects");
  turn.fetch_add(1,std::memory_order_release);
  while(!concurrent_start.load(std::memory_order_acquire))std::this_thread::yield();
  for(int i=0;i<7;++i){begin();{Recorder::Call call(concurrent,1,0,0,10);call.result(0);call.finishAt(20);}require(end()==0,"concurrent fixed callback effects");}
 });
 while(turn.load(std::memory_order_acquire)!=threads.size())std::this_thread::yield();
 concurrent_start.store(true,std::memory_order_release);
 for(auto& thread:threads){thread.join();}
 s=concurrent.seal();
 require(s.offered==64&&s.retained+s.contention_dropped==64&&s.unfinished_writers==0,"bounded concurrent reservation accounting");
 std::vector<lvb_process_call_record_t> rows(size_t(s.retained));require(concurrent.snapshot(s,rows.data(),uint32_t(rows.size())),"all admitted writers publish once");
 std::vector<uint64_t> ordinals;for(const auto& row:rows)ordinals.push_back(row.sequence);std::sort(ordinals.begin(),ordinals.end());
 require(std::adjacent_find(ordinals.begin(),ordinals.end())==ordinals.end(),"unique entry reservations under concurrency");
 std::vector<uint32_t> tids;for(const auto& row:rows){require(row.namespace_tid!=callingTid()&&row.namespace_pid==uint32_t(getpid()),"actual worker calling TID differs from test owner");tids.push_back(row.namespace_tid);}
 std::sort(tids.begin(),tids.end());tids.erase(std::unique(tids.begin(),tids.end()),tids.end());require(tids.size()==8,"all eight real calling thread identities retained");
 RecorderTestAccess::exhaustInstances();Recorder exhausted(true,1);
 {Recorder::Call call(exhausted,1,0,0,10);call.result(0);call.finishAt(20);}
 s=exhausted.seal();require(s.instance==0&&s.invalid_identity==1&&!(s.flags&complete),"instance identity exhaustion refuses reuse/completeness");
 std::puts("Observer disabled/enabled audit, saturation, allocation/clock/ordinal failure, sealing and concurrency PASS");
}
