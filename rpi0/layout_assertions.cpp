#include <atomic>
#include <bit>
#include <cstddef>
#include <cstdint>
struct alignas(8) Event {uint32_t kind,id;int32_t channel,pitch,note,offset;double value;};
struct alignas(8) Context {uint32_t present,state;double rate;int64_t project,system,continuous;double music,bar,cycle_start,cycle_end,tempo;int32_t numerator,denominator,clock,reserved;};
struct alignas(8) Delivery {uint64_t missing,expired,gaps,delivered,priming;};
struct alignas(8) Architecture {char magic[8];uint32_t version,extent,endian,pointer_bits,event_extent,context_extent,gui_extent,atomic32,atomic64,page_size,reserved,padding;uint64_t digest;};
static_assert(sizeof(void*)==8);
static_assert(std::endian::native==std::endian::little);
static_assert(std::atomic<uint32_t>::is_always_lock_free&&std::atomic<uint64_t>::is_always_lock_free);
static_assert(sizeof(Event)==32&&alignof(Event)==8);
static_assert(sizeof(Context)==96&&alignof(Context)==8);
static_assert(sizeof(Delivery)==40&&alignof(Delivery)==8);
static_assert(sizeof(Architecture)==64&&alignof(Architecture)==8);
int main(){return 0;}
