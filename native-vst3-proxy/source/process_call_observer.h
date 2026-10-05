#pragma once
#include "process_call_timing.h"
#include <atomic>
#include <cmath>
#include <cstdlib>
#include <cstring>
#include <limits>
#include <memory>
#include <new>
#include <time.h>
#include <unistd.h>
#ifdef __linux__
#include <sys/syscall.h>
#endif
namespace LVBCallTiming {
inline uint64_t clock() noexcept {
 timespec value{};
 if(clock_gettime(CLOCK_MONOTONIC,&value)||value.tv_sec<0)return 0;
 return uint64_t(value.tv_sec)*1000000000+uint64_t(value.tv_nsec);
}
// Fixed metadata syscall, not file/network I/O, waiting or thread-local setup.
inline uint32_t callingTid() noexcept {
#ifdef __linux__
 const auto value=::syscall(SYS_gettid);
 return value>0&&uint64_t(value)<=UINT32_MAX?uint32_t(value):0;
#else
 return 0; // No fabricated cross-platform thread identity.
#endif
}
inline bool optIn() noexcept {
 const auto* value=std::getenv("LVB_PROCESS_CALL_TRACE");
 return value&&std::strcmp(value,"1")==0;
}
class Recorder final {
#ifdef LVB_PROCESS_CALL_TESTS
 friend struct RecorderTestAccess;
#endif
 struct Slot {std::atomic<bool> ready{false};lvb_process_call_record_t record{};};
 static_assert(std::atomic<uint64_t>::is_always_lock_free);
 static_assert(std::atomic<uint32_t>::is_always_lock_free);
 static_assert(std::atomic<bool>::is_always_lock_free);
 static_assert(sizeof(Slot)==120);
 static constexpr uint64_t closed=uint64_t(1)<<63;
 inline static std::atomic<uint64_t> instances_{1};
 static uint64_t nextInstance()noexcept {
  auto next=instances_.load(std::memory_order_relaxed);
  for(unsigned attempt=0;attempt<4;++attempt){
   if(next==UINT64_MAX)return 0; // Never wrap/reuse an SDK observation identity.
   if(instances_.compare_exchange_weak(next,next+1,std::memory_order_relaxed))return next;
  }
  return 0; // Explicit incomplete identity if construction reservations contend.
 }
 const bool requested_;
 const uint32_t capacity_,pid_;
 const uint64_t instance_;
 std::unique_ptr<Slot[]> slots_;
 // Registration and sealing share one atomic RMW protocol: no check/register race.
 std::atomic<uint64_t> gate_{0},offered_{0},capacity_dropped_{0},
  contention_dropped_{0},allocation_dropped_{0},invalid_clocks_{0},
  invalid_identity_{0},after_seal_{0};
 std::atomic<uint32_t> reserved_{0};
 std::atomic<bool> overflow_{false};
 bool exported_=false; // Quiescent lifecycle owner only.
public:
 explicit Recorder(bool enabled=optIn(),uint32_t count=capacity) noexcept
  :requested_(enabled),capacity_(count&&count<=capacity?count:0),
   pid_(uint32_t(::getpid())),instance_(enabled?nextInstance():0) {
  if(requested_&&capacity_){
   slots_.reset(new(std::nothrow) Slot[capacity_]{});
   if(slots_){
    const auto page=::sysconf(_SC_PAGESIZE);
    if(page<=0){slots_.reset();return;}
    // Volatile touches defeat calloc/lazy-zero elision; every storage page is
    // writable before activation. No callback prepares or faults it deliberately.
    auto* bytes=reinterpret_cast<volatile unsigned char*>(slots_.get());
    const auto length=size_t(capacity_)*sizeof(Slot);
    for(size_t at=0;at<length;at+=size_t(page))bytes[at]=bytes[at];
    bytes[length-1]=bytes[length-1];
   }
  }
 }
 bool requestedEnabled()const noexcept{return requested_;}
 class Call final {
  Recorder& owner_;
  Slot* slot_=nullptr;
  bool registered_=false,returned_=false;
 public:
  Call(Recorder& owner,int32_t frames,int32_t mode,int32_t precision,uint64_t entry)noexcept
   :owner_(owner) {
   if(!owner_.requested_)return;
   const auto gate=owner_.gate_.fetch_add(1,std::memory_order_acq_rel);
   if(gate&closed){owner_.gate_.fetch_sub(1,std::memory_order_release);
    owner_.after_seal_.fetch_add(1,std::memory_order_relaxed);return;}
   registered_=true;
   const auto ordinal=owner_.offered_.fetch_add(1,std::memory_order_relaxed);
   if(ordinal==UINT64_MAX)owner_.overflow_.store(true,std::memory_order_relaxed);
   if(!owner_.slots_){owner_.allocation_dropped_.fetch_add(1,std::memory_order_relaxed);return;}
   auto index=owner_.reserved_.load(std::memory_order_relaxed);
   // A fixed four-attempt reservation; contention drops are explicit, never spin.
   for(unsigned attempt=0;attempt<4;++attempt){
    if(index>=owner_.capacity_){owner_.capacity_dropped_.fetch_add(1,std::memory_order_relaxed);return;}
    if(owner_.reserved_.compare_exchange_weak(index,index+1,std::memory_order_relaxed)){
     slot_=&owner_.slots_[index];break;
    }
   }
   if(!slot_){owner_.contention_dropped_.fetch_add(1,std::memory_order_relaxed);return;}
   auto& record=slot_->record;
   record.schema=schema;record.size=sizeof(record);record.instance=owner_.instance_;
   record.sequence=ordinal;record.entry_ns=entry;record.frames=frames;
   record.mode=mode;record.precision=precision;record.namespace_pid=owner_.pid_;
   record.namespace_tid=callingTid();
   if(entry)record.valid|=entry_valid;
   if(record.namespace_pid&&record.namespace_tid&&record.instance)record.valid|=identity_valid;
  }
  Call(const Call&)=delete;Call&operator=(const Call&)=delete;
  void configuration(double rate,int32_t maximum,uint64_t revision,int32_t phase,
      uint64_t handle)noexcept {
   if(!slot_)return;
   auto& r=slot_->record;r.sample_rate=rate;r.maximum=maximum;r.configuration=revision;
   r.phase=phase;r.backend_handle=handle;r.valid|=owner_context_valid;
   if(revision&&maximum>0&&std::isfinite(rate)&&rate>0)r.valid|=configuration_valid;
  }
  void project(int64_t samples)noexcept {
   if(slot_){slot_->record.project_samples=samples;slot_->record.valid|=project_valid;}
  }
  void result(int32_t status)noexcept {
   returned_=true;
   if(slot_){slot_->record.sdk_result=status;slot_->record.valid|=result_valid;}
  }
  void finishAt(uint64_t end)noexcept {
   if(!registered_)return;
   if(slot_){auto& record=slot_->record;record.return_ns=end;
    record.outcome=returned_?returned:unwound;
    if(end)record.valid|=return_valid;
    if(record.entry_ns&&end&&end>=record.entry_ns)record.valid|=duration_valid;
    else owner_.invalid_clocks_.fetch_add(1,std::memory_order_relaxed);
    if(!(record.valid&identity_valid))owner_.invalid_identity_.fetch_add(1,std::memory_order_relaxed);
    slot_->ready.store(true,std::memory_order_release);
   }
   owner_.gate_.fetch_sub(1,std::memory_order_release);registered_=false;
  }
  ~Call(){if(registered_)finishAt(clock());}
 };
 // Off-RT, normal quiescent termination only. No writer field is read before
 // its release publication; any registered writer refuses a complete snapshot.
 lvb_process_call_summary_t seal()noexcept {
  lvb_process_call_summary_t s{};s.schema=schema;s.size=sizeof(s);s.capacity=capacity_;
  s.namespace_pid=pid_;s.record_size=sizeof(lvb_process_call_record_t);s.instance=instance_;
  if(!requested_)return s;
  const auto previous=gate_.fetch_or(closed,std::memory_order_acq_rel);
  s.flags=requested|sealed|(slots_?allocated:0);s.unfinished_writers=previous&~closed;
  s.offered=offered_.load(std::memory_order_acquire);s.retained=reserved_.load(std::memory_order_acquire);
  s.capacity_dropped=capacity_dropped_.load();s.contention_dropped=contention_dropped_.load();
  s.allocation_dropped=allocation_dropped_.load();s.invalid_clocks=invalid_clocks_.load();
  s.invalid_identity=invalid_identity_.load();s.after_seal=after_seal_.load();
  s.sequence_overflow=overflow_.load()?1:0;
  if(slots_&&instance_&&!s.unfinished_writers&&!s.capacity_dropped&&!s.contention_dropped&&
    !s.allocation_dropped&&!s.invalid_clocks&&!s.invalid_identity&&!s.after_seal&&
    !s.sequence_overflow&&s.offered==s.retained)s.flags|=complete;
  return s;
 }
 // Plain copied span for the Rust export; no atomic/STL ownership crosses ABI.
 bool snapshot(const lvb_process_call_summary_t& s,lvb_process_call_record_t* out,uint32_t count)const noexcept {
  if(!(s.flags&sealed)||s.unfinished_writers||count!=s.retained||count>capacity_||
    (count&&(!out||!slots_)))return false;
  for(uint32_t i=0;i<count;++i){if(!slots_[i].ready.load(std::memory_order_acquire))return false;
   out[i]=slots_[i].record;}
  return true;
 }
 bool markExported()noexcept {if(exported_)return false;exported_=true;return true;}
};
}
