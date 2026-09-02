#pragma once

#include "linux_vst_bridge/wf0_probe/events.h"

#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/vst/ivstaudioprocessor.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivsthostapplication.h"

#include <windows.h>

#include <array>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <string>

namespace linux_vst_bridge::wf0 {

enum class ComponentCase {
    exact_again,
    unknown_processor,
    unsupported_interface,
};

enum class ComponentState {
    component_absent,
    component_create_in_flight,
    component_created,
    controller_id_in_flight,
    controller_id_verified,
    host_context_ready,
    component_initialize_in_flight,
    component_initialized,
    component_terminate_in_flight,
    component_terminated,
    component_release_in_flight,
    component_released,
    component_retirement_incomplete,
};

enum class AudioProcessorLeaseState {
    audio_processor_absent,
    audio_processor_query_in_flight,
    audio_processor_query_returned_without_lease,
    audio_processor_lease_acquired,
    audio_processor_release_in_flight,
    audio_processor_lease_retired,
    audio_processor_retirement_incomplete,
    audio_processor_ownership_unknown,
};

struct HostCallbackRecord {
    const char* operation{nullptr};
    const char* origin{nullptr};
    unsigned long long enclosing_attempt{0};
    const char* enclosing_operation{nullptr};
    Steinberg::uint32 reference_count{0};
    Steinberg::uint32 result_u32{0};
    bool reference_count_present{false};
    bool result_present{false};
    bool output_null{false};
    std::array<Steinberg::int8, 16> cid{};
    std::array<Steinberg::int8, 16> iid{};
    bool identifiers_present{false};
};

class HostCallbackSink final {
public:
    static constexpr std::size_t kCapacity = 64;

    HostCallbackSink(EventWriter* events, DWORD scanner_thread) noexcept;

    void begin_plugin_call(unsigned long long attempt, const char* operation) noexcept;
    void end_plugin_call() noexcept;
    void begin_owner_call() noexcept;
    void end_owner_call() noexcept;
    void record_reference(const char* operation, Steinberg::uint32 count) noexcept;
    void record_result(const char* operation, Steinberg::tresult result,
                       bool output_null) noexcept;
    void record_create_instance(Steinberg::tresult result, bool output_null,
                                const Steinberg::TUID cid,
                                const Steinberg::TUID iid) noexcept;

    std::size_t size() const noexcept { return size_; }
    const HostCallbackRecord& at(std::size_t index) const noexcept { return records_[index]; }
    Steinberg::uint32 last_reference_count() const noexcept { return last_reference_count_; }
    bool unexpected_object_request() const noexcept { return unexpected_object_request_; }
    bool healthy() const noexcept;
    bool close() noexcept;
    bool closed() const noexcept;
    bool overflowed() const noexcept { return overflowed_; }
    bool output_failed() const noexcept { return output_failed_; }
    bool wrong_thread() const noexcept { return wrong_thread_; }
    bool callback_in_flight() const noexcept { return callback_in_flight_; }
    std::size_t plugin_callback_count(const char* operation) const noexcept;

private:
    void retain_and_emit(HostCallbackRecord record) noexcept;

    EventWriter* events_{nullptr};
    DWORD scanner_thread_{0};
    std::array<HostCallbackRecord, kCapacity> records_{};
    std::size_t size_{0};
    unsigned long long enclosing_attempt_{0};
    const char* enclosing_operation_{nullptr};
    const char* origin_{"none"};
    Steinberg::uint32 last_reference_count_{1};
    bool callback_in_flight_{false};
    bool overflowed_{false};
    bool output_failed_{false};
    bool wrong_thread_{false};
    bool unexpected_object_request_{false};
    bool closed_{false};
};

class MinimalHostApplication final : public Steinberg::Vst::IHostApplication {
public:
    explicit MinimalHostApplication(HostCallbackSink& sink) noexcept;

    Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID interface_id,
                                                  void** object) noexcept override;
    Steinberg::uint32 PLUGIN_API addRef() noexcept override;
    Steinberg::uint32 PLUGIN_API release() noexcept override;
    Steinberg::tresult PLUGIN_API getName(Steinberg::Vst::String128 name) noexcept override;
    Steinberg::tresult PLUGIN_API createInstance(Steinberg::TUID cid,
                                                  Steinberg::TUID interface_id,
                                                  void** object) noexcept override;

private:
    ~MinimalHostApplication() = default;

    HostCallbackSink& sink_;
    volatile LONG references_{1};
};

struct AudioProcessorLeaseResult {
    AudioProcessorLeaseState state{
        AudioProcessorLeaseState::audio_processor_absent};
    int primary_exit{0};
    std::array<Steinberg::int8, 16> requested_iid{};
    bool query_output_zero_initialized{false};
    bool query_attempted{false};
    Steinberg::tresult query_result{Steinberg::kResultFalse};
    bool query_output_nonnull{false};
    bool query_tuple_consistent{false};
    bool lease_acquired{false};
    bool release_attempted{false};
    bool release_returned_ordinary{false};
    Steinberg::uint32 release_result{0};
    bool pointer_cleared{false};
    bool call_in_flight{false};
    bool callback_ledger_unchanged{false};
    bool audio_interface_quiescence{true};

    std::string json_fields() const;
};

class AudioProcessorInterfaceLease final {
public:
    AudioProcessorInterfaceLease(Steinberg::Vst::IComponent& component,
                                 HostCallbackSink& callbacks,
                                 EventWriter& events) noexcept;

    AudioProcessorLeaseResult acquire_and_retire();

private:
    Steinberg::Vst::IComponent& component_;
    HostCallbackSink& callbacks_;
    EventWriter& events_;
    Steinberg::Vst::IAudioProcessor* interface_{nullptr};
};

struct ComponentAdmissionResult {
    ComponentState state{ComponentState::component_absent};
    int primary_exit{0};
    bool create_attempted{false};
    std::array<Steinberg::int8, 16> processor_cid{};
    std::array<Steinberg::int8, 16> requested_iid{};
    Steinberg::tresult create_result{Steinberg::kResultFalse};
    bool create_output_nonnull{false};
    bool create_tuple_consistent{false};
    bool controller_id_attempted{false};
    Steinberg::tresult controller_id_result{Steinberg::kResultFalse};
    bool controller_id_buffer_zero_initialized{false};
    bool controller_id_valid{false};
    bool controller_id_matches{false};
    std::array<Steinberg::int8, 16> controller_id{};
    bool host_created{false};
    Steinberg::uint32 host_reference_baseline{1};
    Steinberg::uint32 host_reference_after_initialize{0};
    Steinberg::uint32 host_reference_after_terminate{0};
    bool host_reference_returned_to_baseline{false};
    Steinberg::uint32 host_owner_release_result{0};
    bool host_owner_release_attempted{false};
    bool initialize_attempted{false};
    Steinberg::tresult initialize_result{Steinberg::kResultFalse};
    bool initialize_succeeded{false};
    bool terminate_attempted{false};
    Steinberg::tresult terminate_result{Steinberg::kResultFalse};
    bool terminate_returned_ordinary{false};
    bool release_attempted{false};
    Steinberg::uint32 release_result{0};
    bool release_returned_ordinary{false};
    bool component_pointer_cleared{false};
    bool component_call_in_flight{false};
    bool callback_ledger_closed{false};
    bool audio_processor_session_ran{false};
    AudioProcessorLeaseResult audio_processor;
    bool object_quiescence{false};
    bool inherited_shutdown_permitted{false};
    std::unique_ptr<HostCallbackSink> callbacks;

    std::string json_fields() const;
};

ComponentAdmissionResult admit_component(Steinberg::IPluginFactory* factory,
                                          EventWriter& events,
                                          ComponentCase component_case);

const char* component_state_name(ComponentState state) noexcept;
const char* audio_processor_state_name(AudioProcessorLeaseState state) noexcept;
constexpr bool audio_release_matches_component_baseline(
    Steinberg::uint32 value) noexcept {
    return value == 1;
}
static_assert(audio_release_matches_component_baseline(1));
static_assert(!audio_release_matches_component_baseline(0));
static_assert(!audio_release_matches_component_baseline(2));
static_assert(!audio_release_matches_component_baseline(0xffffffffu));
bool run_component_owner_regressions() noexcept;

} // namespace linux_vst_bridge::wf0
