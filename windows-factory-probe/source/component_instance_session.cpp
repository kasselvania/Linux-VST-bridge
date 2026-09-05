#include "offline_processing.h"
#include "public.sdk/source/vst/hosting/hostclasses.h"
#include "component_instance_session.h"

#include <algorithm>
#include <array>
#include <cstring>
#include <limits>
#include <stdexcept>
#include <string>
#include <bit>
#include "pluginterfaces/vst/vstspeaker.h"

namespace linux_vst_bridge::wf0 {

int scanner_output_failure_exit(bool pc0_mode, int first_primary) noexcept {
    return pc0_mode ? (first_primary == 0 ? 99 : first_primary) : 82;
}

namespace {

constexpr int kCreateBlocked = 83;
constexpr int kControllerIdBlocked = 84;
constexpr int kControllerIdMismatch = 85;
constexpr int kHostContextBlocked = 86;
constexpr int kInitializeBlocked = 87;
constexpr int kTerminateBlocked = 88;
constexpr int kReleaseBlocked = 89;
constexpr int kAudioProcessorQueryBlocked = 90;
constexpr int kAudioProcessorQueryInconsistent = 91;
constexpr int kAudioProcessorReleaseBlocked = 92;

const Steinberg::TUID kAgainProcessor =
    INLINE_UID(0x84E8DE5F, 0x92554F53, 0x96FAE413, 0x3C935A18);
const Steinberg::TUID kAgainController =
    INLINE_UID(0xD39D5B65, 0xD7AF42FA, 0x843F4AC8, 0x41EB04F0);
const Steinberg::TUID kUnknownProcessor =
    INLINE_UID(0x10213243, 0x54657687, 0x98A9BACB, 0xDCEDFE0F);

std::string bool_json(bool value) { return value ? "true" : "false"; }

std::string u32_hex(Steinberg::uint32 value) {
    static constexpr char digits[] = "0123456789abcdef";
    std::string result(8, '0');
    for (int index = 7; index >= 0; --index) {
        result[static_cast<std::size_t>(index)] = digits[value & 0x0fu];
        value >>= 4u;
    }
    return result;
}

std::string tuid_hex(const Steinberg::int8* value) {
    static constexpr char digits[] = "0123456789ABCDEF";
    std::string result(32, '0');
    for (std::size_t index = 0; index < 16; ++index) {
        const auto byte = static_cast<unsigned char>(value[index]);
        result[index * 2] = digits[(byte >> 4u) & 0x0fu];
        result[index * 2 + 1] = digits[byte & 0x0fu];
    }
    return result;
}

const char* blocker_name(int value) noexcept {
    switch (value) {
        case kCreateBlocked: return "WC0_COMPONENT_CREATE_BLOCKED";
        case kControllerIdBlocked: return "WC0_CONTROLLER_ID_BLOCKED";
        case kControllerIdMismatch: return "WC0_CONTROLLER_ID_MISMATCH";
        case kHostContextBlocked: return "WC0_HOST_CONTEXT_BLOCKED";
        case kInitializeBlocked: return "WC0_COMPONENT_INITIALIZE_BLOCKED";
        case kTerminateBlocked: return "WC0_COMPONENT_TERMINATE_BLOCKED";
        case kReleaseBlocked: return "WC0_COMPONENT_RELEASE_BLOCKED";
        case kAudioProcessorQueryBlocked: return "WA0_INTERFACE_QUERY_BLOCKED";
        case kAudioProcessorQueryInconsistent:
            return "WA0_INTERFACE_QUERY_INCONSISTENT";
        case kAudioProcessorReleaseBlocked: return "WA0_INTERFACE_RELEASE_BLOCKED";
        case 93: return "PC0_BUS_COUNT_BLOCKED";
        case 94: return "PC0_BUS_INFO_BLOCKED";
        case 95: return "PC0_BUS_ARRANGEMENT_BLOCKED";
        case 96: return "PC0_SAMPLE_FORMAT_BLOCKED";
        case 97: return "PC0_CONTRACT_INCOMPLETE";
        case 99: return "PC0_EVIDENCE_BLOCKED";
        default: return nullptr;
    }
}

void latch(int& primary, int candidate) noexcept {
    if (primary == 0) primary = candidate;
}

void state_event(ComponentAdmissionResult& result, ComponentState state,
                 EventWriter& events, const std::string& fields = {}) {
    result.state = state;
    events.lifecycle(component_state_name(state), fields);
}

bool create_tuple_consistent(Steinberg::tresult result, const void* output) noexcept {
    return (result == Steinberg::kResultOk) == (output != nullptr);
}

void audio_state_event(AudioProcessorLeaseResult& result,
                       AudioProcessorLeaseState state, EventWriter& events,
                       const std::string& fields = {}) {
    result.state = state;
    events.lifecycle(audio_processor_state_name(state), fields);
}

} // namespace

const char* component_state_name(ComponentState state) noexcept {
    switch (state) {
        case ComponentState::component_absent: return "component_absent";
        case ComponentState::component_create_in_flight: return "component_create_in_flight";
        case ComponentState::component_created: return "component_created";
        case ComponentState::controller_id_in_flight: return "controller_id_in_flight";
        case ComponentState::controller_id_verified: return "controller_id_verified";
        case ComponentState::host_context_ready: return "host_context_ready";
        case ComponentState::component_initialize_in_flight:
            return "component_initialize_in_flight";
        case ComponentState::component_initialized: return "component_initialized";
        case ComponentState::component_terminate_in_flight:
            return "component_terminate_in_flight";
        case ComponentState::component_terminated: return "component_terminated";
        case ComponentState::component_release_in_flight:
            return "component_release_in_flight";
        case ComponentState::component_released: return "component_released";
        case ComponentState::component_retirement_incomplete:
            return "component_retirement_incomplete";
    }
    return "component_absent";
}

const char* audio_processor_state_name(AudioProcessorLeaseState state) noexcept {
    switch (state) {
        case AudioProcessorLeaseState::audio_processor_absent:
            return "audio_processor_absent";
        case AudioProcessorLeaseState::audio_processor_query_in_flight:
            return "audio_processor_query_in_flight";
        case AudioProcessorLeaseState::audio_processor_query_returned_without_lease:
            return "audio_processor_query_returned_without_lease";
        case AudioProcessorLeaseState::audio_processor_lease_acquired:
            return "audio_processor_lease_acquired";
        case AudioProcessorLeaseState::audio_processor_release_in_flight:
            return "audio_processor_release_in_flight";
        case AudioProcessorLeaseState::audio_processor_lease_retired:
            return "audio_processor_lease_retired";
        case AudioProcessorLeaseState::audio_processor_retirement_incomplete:
            return "audio_processor_retirement_incomplete";
        case AudioProcessorLeaseState::audio_processor_ownership_unknown:
            return "audio_processor_ownership_unknown";
    }
    return "audio_processor_ownership_unknown";
}

HostCallbackSink::HostCallbackSink(EventWriter* events, DWORD scanner_thread) noexcept
    : events_(events), scanner_thread_(scanner_thread) {}

void HostCallbackSink::begin_plugin_call(unsigned long long attempt,
                                         const char* operation) noexcept {
    enclosing_attempt_ = attempt;
    enclosing_operation_ = operation;
    origin_ = "component";
}

void HostCallbackSink::end_plugin_call() noexcept {
    enclosing_attempt_ = 0;
    enclosing_operation_ = nullptr;
    origin_ = "none";
}

void HostCallbackSink::begin_owner_call() noexcept {
    enclosing_attempt_ = 0;
    enclosing_operation_ = nullptr;
    origin_ = "owner_local";
}

void HostCallbackSink::end_owner_call() noexcept { origin_ = "none"; }

void HostCallbackSink::record_reference(const char* operation,
                                        Steinberg::uint32 count) noexcept {
    HostCallbackRecord record;
    record.operation = operation;
    record.reference_count = count;
    record.reference_count_present = true;
    last_reference_count_ = count;
    retain_and_emit(record);
}

void HostCallbackSink::record_result(const char* operation,
                                     Steinberg::tresult result,
                                     bool output_null) noexcept {
    HostCallbackRecord record;
    record.operation = operation;
    record.result_u32 = static_cast<Steinberg::uint32>(result);
    record.result_present = true;
    record.output_null = output_null;
    retain_and_emit(record);
}

void HostCallbackSink::record_create_instance(Steinberg::tresult result,
                                              bool output_null,
                                              const Steinberg::TUID cid,
                                              const Steinberg::TUID iid, bool expected) noexcept {
    HostCallbackRecord record;
    record.operation = "createInstance";
    record.result_u32 = static_cast<Steinberg::uint32>(result);
    record.result_present = true;
    record.output_null = output_null;
    record.identifiers_present = true;
    if (cid != nullptr && iid != nullptr) {
        std::memcpy(record.cid.data(), cid, record.cid.size());
        std::memcpy(record.iid.data(), iid, record.iid.size());
        record.identifiers_present = true;
    }
    unexpected_object_request_ = unexpected_object_request_ || !expected;
    retain_and_emit(record);
}

void HostCallbackSink::retain_and_emit(HostCallbackRecord record) noexcept {
    if (closed_ || callback_in_flight_) {
        output_failed_ = true;
        return;
    }
    callback_in_flight_ = true;
    if (GetCurrentThreadId() != scanner_thread_) wrong_thread_ = true;
    record.origin = origin_;
    record.enclosing_attempt = enclosing_attempt_;
    record.enclosing_operation = enclosing_operation_;
    if (size_ >= records_.size()) {
        overflowed_ = true;
        callback_in_flight_ = false;
        return;
    }
    records_[size_++] = record;
    if (events_ != nullptr) {
        try {
            std::string fields =
                ",\"origin\":\"" + std::string(record.origin) + "\"" +
                ",\"enclosing_attempt_sequence\":" +
                (record.enclosing_attempt == 0
                     ? std::string("null")
                     : std::to_string(record.enclosing_attempt)) +
                ",\"enclosing_operation\":" +
                (record.enclosing_operation == nullptr
                     ? std::string("null")
                     : std::string("\"") + record.enclosing_operation + "\"") +
                ",\"thread_role\":\"scanner_main_thread\"";
            if (record.reference_count_present)
                fields += ",\"reference_count\":" +
                          std::to_string(record.reference_count);
            if (record.result_present)
                fields += ",\"result_u32_hex\":\"" +
                          u32_hex(record.result_u32) + "\"" +
                          ",\"output_null\":" + bool_json(record.output_null);
            if (record.identifiers_present)
                fields += ",\"cid_raw_tuid_hex\":\"" +
                          tuid_hex(record.cid.data()) + "\"" +
                          ",\"iid_raw_tuid_hex\":\"" +
                          tuid_hex(record.iid.data()) + "\"";
            events_->host_callback(record.operation, fields);
        } catch (...) {
            output_failed_ = true;
        }
    }
    callback_in_flight_ = false;
}

bool HostCallbackSink::healthy() const noexcept {
    return !callback_in_flight_ && !overflowed_ && !output_failed_ && !wrong_thread_;
}

bool HostCallbackSink::close() noexcept {
    if (!healthy()) return false;
    closed_ = true;
    return true;
}

bool HostCallbackSink::closed() const noexcept { return closed_ && healthy(); }

std::size_t HostCallbackSink::plugin_callback_count(const char* operation) const noexcept {
    std::size_t count = 0;
    for (std::size_t index = 0; index < size_; ++index) {
        if (std::strcmp(records_[index].origin, "component") == 0 &&
            std::strcmp(records_[index].operation, operation) == 0)
            ++count;
    }
    return count;
}

MinimalHostApplication::MinimalHostApplication(HostCallbackSink& sink, bool processing) noexcept
    : sink_(sink), processing_(processing) {}

Steinberg::tresult PLUGIN_API MinimalHostApplication::queryInterface(
    const Steinberg::TUID interface_id, void** object) noexcept {
    if (object == nullptr) {
        sink_.record_result("queryInterface", Steinberg::kInvalidArgument, true);
        return Steinberg::kInvalidArgument;
    }
    *object = nullptr;
    if (Steinberg::FUnknownPrivate::iidEqual(interface_id,
                                              INLINE_UID_OF(Steinberg::FUnknown)) ||
        Steinberg::FUnknownPrivate::iidEqual(
            interface_id, INLINE_UID_OF(Steinberg::Vst::IHostApplication))) {
        *object = static_cast<Steinberg::Vst::IHostApplication*>(this);
        addRef();
        sink_.record_result("queryInterface", Steinberg::kResultOk, false);
        return Steinberg::kResultOk;
    }
    sink_.record_result("queryInterface", Steinberg::kNoInterface, true);
    return Steinberg::kNoInterface;
}

Steinberg::uint32 PLUGIN_API MinimalHostApplication::addRef() noexcept {
    const auto value = static_cast<Steinberg::uint32>(InterlockedIncrement(&references_));
    sink_.record_reference("addRef", value);
    return value;
}

Steinberg::uint32 PLUGIN_API MinimalHostApplication::release() noexcept {
    const LONG remaining = InterlockedDecrement(&references_);
    const auto value = static_cast<Steinberg::uint32>(remaining);
    sink_.record_reference("release", value);
    if (remaining == 0) delete this;
    return value;
}

Steinberg::tresult PLUGIN_API MinimalHostApplication::getName(
    Steinberg::Vst::String128 name) noexcept {
    if (name == nullptr) {
        sink_.record_result("getName", Steinberg::kInvalidArgument, true);
        return Steinberg::kInvalidArgument;
    }
    constexpr char text[] = "Linux VST Bridge WC0";
    std::fill_n(name, 128, static_cast<Steinberg::Vst::TChar>(0));
    for (std::size_t index = 0; index < sizeof(text) - 1; ++index)
        name[index] = static_cast<Steinberg::Vst::TChar>(text[index]);
    sink_.record_result("getName", Steinberg::kResultOk, false);
    return Steinberg::kResultOk;
}

Steinberg::tresult PLUGIN_API MinimalHostApplication::createInstance(
    Steinberg::TUID cid, Steinberg::TUID interface_id, void** object) noexcept {
    if (object != nullptr) *object = nullptr;
    if (processing_ && object != nullptr &&
        Steinberg::FUnknownPrivate::iidEqual(cid, INLINE_UID_OF(Steinberg::Vst::IMessage)) &&
        Steinberg::FUnknownPrivate::iidEqual(interface_id, INLINE_UID_OF(Steinberg::Vst::IMessage))) {
        try { *object = new Steinberg::Vst::HostMessage(); }
        catch (...) { return Steinberg::kOutOfMemory; }
        sink_.record_create_instance(Steinberg::kResultOk, false, cid, interface_id, true);
        return Steinberg::kResultOk;
    }
    sink_.record_create_instance(Steinberg::kResultFalse, true, cid, interface_id);
    return Steinberg::kResultFalse;
}

AudioProcessorInterfaceLease::AudioProcessorInterfaceLease(
    Steinberg::Vst::IComponent& component, HostCallbackSink& callbacks,
    EventWriter& events) noexcept
    : component_(component), callbacks_(callbacks), events_(events) {}

namespace {
const char* media_name(int value) { return value == 0 ? "kAudio" : "kEvent"; }
const char* direction_name(int value) { return value == 0 ? "kInput" : "kOutput"; }
std::string coordinates(int media, int direction, int index = -1) {
    return ",\"media_type\":\"" + std::string(media_name(media)) +
        "\",\"direction\":\"" + direction_name(direction) + "\"" +
        (index < 0 ? "" : ",\"index\":" + std::to_string(index));
}
std::string u64_hex(Steinberg::uint64 value) {
    return u32_hex(static_cast<Steinberg::uint32>(value >> 32)) +
           u32_hex(static_cast<Steinberg::uint32>(value));
}
std::string json_string(const std::string& value) {
    std::string output = "\"";
    for (unsigned char byte : value) {
        if (byte == '"' || byte == '\\') { output += '\\'; output += static_cast<char>(byte); }
        else if (byte < 0x20) output += "\\u00" + u32_hex(byte).substr(6);
        else output += static_cast<char>(byte);
    }
    return output + "\"";
}
bool bus_name(const Steinberg::Vst::String128& input, std::string& output) {
    output.clear();
    for (std::size_t i = 0; i < 128; ++i) {
        unsigned cp = static_cast<unsigned short>(input[i]);
        if (cp == 0) return true;
        if (cp >= 0xd800 && cp <= 0xdbff) {
            if (++i >= 128) return false;
            const unsigned low = static_cast<unsigned short>(input[i]);
            if (low < 0xdc00 || low > 0xdfff) return false;
            cp = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00);
        } else if (cp >= 0xdc00 && cp <= 0xdfff) return false;
        if (cp < 0x80) output += static_cast<char>(cp);
        else if (cp < 0x800) {
            output += static_cast<char>(0xc0 | (cp >> 6));
            output += static_cast<char>(0x80 | (cp & 63));
        } else if (cp < 0x10000) {
            output += static_cast<char>(0xe0 | (cp >> 12));
            output += static_cast<char>(0x80 | ((cp >> 6) & 63));
            output += static_cast<char>(0x80 | (cp & 63));
        } else {
            output += static_cast<char>(0xf0 | (cp >> 18));
            output += static_cast<char>(0x80 | ((cp >> 12) & 63));
            output += static_cast<char>(0x80 | ((cp >> 6) & 63));
            output += static_cast<char>(0x80 | (cp & 63));
        }
    }
    return false;
}
} // namespace

bool PreSetupProcessingContractCensus::checked_count(
    Steinberg::int32 count, std::size_t& aggregate) noexcept {
    if (count < 0 || count > 32 || aggregate > 64 ||
        static_cast<std::size_t>(count) > 64 - aggregate) return false;
    aggregate += static_cast<std::size_t>(count);
    return true;
}

void PreSetupProcessingContractCensus::transition(PreSetupCensusState state) {
    static constexpr const char* names[] = {
        "pre_setup_census_absent", "bus_count_in_flight", "bus_counts_validated",
        "detail_call_in_flight", "bus_info_complete", "speaker_arrangements_complete",
        "sample_format_call_in_flight", "pre_setup_contract_complete", "pre_setup_census_blocked",
    };
    state_ = state;
    events_.lifecycle(names[static_cast<unsigned>(state)]);
}

void PreSetupProcessingContractCensus::prepare_call(
    const char* operation, const char* interface_name,
    const std::string& coordinates_value) {
    // Prepare every possibly-throwing value before durable call_started. The
    // later in-flight flip is POD-only and cannot create a durable attempt
    // without the corresponding plug-in call.
    if (coordinates_value.size() >= active_coordinates_.size())
        throw std::length_error("PC0 call coordinates exceed their bound");
    active_operation_ = operation;
    active_interface_ = interface_name;
    active_coordinates_size_ = coordinates_value.size();
    std::copy_n(coordinates_value.data(), active_coordinates_size_,
                active_coordinates_.data());
    active_coordinates_[active_coordinates_size_] = '\0';
}

void PreSetupProcessingContractCensus::begin_call(
    PreSetupCensusState state) noexcept {
    // call_started has already been durably published. Only non-throwing
    // internal state changes occur between that flush and the VST3 call.
    active_call_state_ = state;
    call_in_flight_ = true;
}

void PreSetupProcessingContractCensus::complete_call() noexcept {
    call_in_flight_ = false;
    active_operation_ = nullptr;
    active_interface_ = nullptr;
    active_coordinates_size_ = 0;
    active_coordinates_[0] = '\0';
}

int PreSetupProcessingContractCensus::block(int code) {
    latch(primary_, code);
    contract_.clear();
    state_ = PreSetupCensusState::pre_setup_census_blocked;
    const char* blocker = blocker_name(primary_);
    events_.lifecycle(
        "pre_setup_census_blocked",
        ",\"primary_blocker\":" +
            (blocker == nullptr ? std::string("null")
                                : std::string("\"") + blocker + "\""));
    return primary_;
}

int PreSetupProcessingContractCensus::run() {
    using namespace Steinberg;
    using namespace Steinberg::Vst;
    // Single use; an output-publication exception propagates past every cleanup
    // call. The process supervisor, never this borrower, contains that failure.
    if (state_ != PreSetupCensusState::pre_setup_census_absent) return block(97);
    try {
        std::size_t aggregate = 0;
        for (int domain = 0; domain < 4; ++domain) {
            const auto media = domain / 2, direction = domain % 2;
            const auto fields = coordinates(media, direction);
            prepare_call("get_bus_count", "IComponent", fields);
            const auto attempt = events_.call_started("get_bus_count", "IComponent", -1, nullptr, fields);
            begin_call(PreSetupCensusState::bus_count_in_flight);
            const int32 count = component_.getBusCount(media, direction);
            events_.call_completed(attempt, "get_bus_count", "IComponent", "int32",
                ",\"i32_result\":" + std::to_string(count), -1, nullptr, fields);
            complete_call();
            ++call_count_;
            if (!checked_count(count, aggregate)) return block(93);
            counts_[static_cast<std::size_t>(domain)] = count;
        }
        transition(PreSetupCensusState::bus_counts_validated);
        for (int domain = 0; domain < 4; ++domain) {
            for (int32 index = 0; index < counts_[static_cast<std::size_t>(domain)]; ++index) {
                BusInfo info{};
                const auto media = domain / 2, direction = domain % 2;
                const auto fields = coordinates(media, direction, index);
                prepare_call("get_bus_info", "IComponent", fields);
                const auto attempt = events_.call_started("get_bus_info", "IComponent", -1, nullptr, fields);
                begin_call(PreSetupCensusState::detail_call_in_flight);
                const tresult result = component_.getBusInfo(media, direction, index, info);
                events_.call_completed(attempt, "get_bus_info", "IComponent", "tresult",
                    ",\"result_u32_hex\":\"" + u32_hex(static_cast<uint32>(result)) + "\"",
                    -1, nullptr, fields);
                complete_call();
                ++call_count_;
                if (result != kResultTrue || info.mediaType != media || info.direction != direction ||
                    info.channelCount < 1 || info.channelCount > (media == kAudio ? 64 : 16) ||
                    (info.busType != kMain && info.busType != kAux) ||
                    (info.flags & ~(BusInfo::kDefaultActive | BusInfo::kIsControlVoltage)) != 0 ||
                    (media == kEvent && (info.flags & BusInfo::kIsControlVoltage) != 0)) return block(94);
                auto& bus = buses_[size_];
                if (!bus_name(info.name, bus.name)) return block(94);
                bus.media = media; bus.direction = direction; bus.index = index;
                bus.channels = info.channelCount; bus.type = info.busType; bus.flags = info.flags;
                ++size_;
            }
        }
        transition(PreSetupCensusState::bus_info_complete);
        for (std::size_t index = 0; index < size_; ++index) {
            auto& bus = buses_[index];
            if (bus.media != kAudio) continue;
            SpeakerArrangement arrangement{};
            const auto fields = ",\"direction\":\"" + std::string(direction_name(bus.direction)) +
                "\",\"audio_index\":" + std::to_string(bus.index);
            prepare_call("get_bus_arrangement", "IAudioProcessor", fields);
            const auto attempt = events_.call_started("get_bus_arrangement", "IAudioProcessor", -1, nullptr, fields);
            begin_call(PreSetupCensusState::detail_call_in_flight);
            const tresult result = audio_.getBusArrangement(bus.direction, bus.index, arrangement);
            events_.call_completed(attempt, "get_bus_arrangement", "IAudioProcessor", "tresult",
                ",\"result_u32_hex\":\"" + u32_hex(static_cast<uint32>(result)) + "\"",
                -1, nullptr, fields);
            complete_call();
            ++call_count_;
            if (result != kResultTrue || std::popcount(static_cast<uint64>(arrangement)) != bus.channels)
                return block(95);
            bus.arrangement = arrangement;
        }
        transition(PreSetupCensusState::speaker_arrangements_complete);
        for (int32 size = kSample32; size <= kSample64; ++size) {
            const auto fields = ",\"symbolic_size\":\"" + std::string(size == kSample32 ? "kSample32" : "kSample64") + "\"";
            prepare_call("can_process_sample_size", "IAudioProcessor", fields);
            const auto attempt = events_.call_started("can_process_sample_size", "IAudioProcessor", -1, nullptr, fields);
            begin_call(PreSetupCensusState::sample_format_call_in_flight);
            const tresult result = audio_.canProcessSampleSize(size);
            events_.call_completed(attempt, "can_process_sample_size", "IAudioProcessor", "tresult",
                ",\"result_u32_hex\":\"" + u32_hex(static_cast<uint32>(result)) + "\"",
                -1, nullptr, fields);
            complete_call();
            ++call_count_;
            if (result != kResultTrue && result != kResultFalse) return block(96);
            samples_[static_cast<std::size_t>(size)] = result;
        }
        std::string value = "{\"schema\":\"linux-vst-bridge-pc0-processing-contract/v1\",\"lifecycle_state\":\"Initialized\",\"counts\":[";
        for (int domain = 0; domain < 4; ++domain) {
            if (domain) value += ',';
            value += "{" + coordinates(domain / 2, domain % 2).substr(1) +
                ",\"count\":" + std::to_string(counts_[static_cast<std::size_t>(domain)]) + "}";
        }
        value += "],\"buses\":[";
        for (std::size_t i = 0; i < size_; ++i) {
            const auto& bus = buses_[i];
            if (i) value += ',';
            value += "{" + coordinates(bus.media, bus.direction, bus.index).substr(1) +
                ",\"name_utf8\":" + json_string(bus.name) + ",\"channel_count\":" + std::to_string(bus.channels) +
                ",\"bus_type\":\"" + (bus.type == kMain ? "kMain" : "kAux") +
                "\",\"flags_u32_hex\":\"" + u32_hex(bus.flags) +
                "\",\"default_active\":" + bool_json((bus.flags & BusInfo::kDefaultActive) != 0) +
                ",\"control_voltage\":" + bool_json((bus.flags & BusInfo::kIsControlVoltage) != 0) +
                ",\"speaker_arrangement\":";
            if (bus.media == kEvent) value += "null";
            else value += "{\"bits_u64_hex\":\"" + u64_hex(bus.arrangement) +
                "\",\"channel_count\":" + std::to_string(bus.channels) +
                ",\"recognized_layout\":" + (bus.arrangement == SpeakerArr::kStereo ? std::string("\"kStereo\"") : std::string("null")) + "}";
            value += "}";
        }
        value += "],\"sample_sizes\":[";
        for (int size = 0; size < 2; ++size) {
            if (size) value += ',';
            value += "{\"symbolic_size\":\"" + std::string(size == 0 ? "kSample32" : "kSample64") +
                "\",\"tresult_i32\":" + std::to_string(samples_[static_cast<std::size_t>(size)]) +
                ",\"tresult_u32_hex\":\"" + u32_hex(static_cast<uint32>(samples_[static_cast<std::size_t>(size)])) +
                "\",\"supported\":" + bool_json(samples_[static_cast<std::size_t>(size)] == kResultTrue) + "}";
        }
        value += "],\"call_count\":" + std::to_string(call_count_) + ",\"complete\":true,\"mutation_call_count\":0}";
        events_.final_lifecycle("pre_setup_contract_complete", ",\"processing_contract\":" + value);
        state_ = PreSetupCensusState::pre_setup_contract_complete;
        contract_ = std::move(value);
        return 0;
    } catch (...) {
        // No in-process release or unload is legal past an unproved writer boundary.
        throw (primary_ == 0 ? 99 : primary_);
    }
}

AudioProcessorLeaseResult AudioProcessorInterfaceLease::acquire_and_retire(bool pre_setup_census, bool offline_processing) {
    AudioProcessorLeaseResult result;
    try {
    const Steinberg::int8* requested =
        INLINE_UID_OF(Steinberg::Vst::IAudioProcessor);
    std::memcpy(result.requested_iid.data(), requested, 16);

    void* output = nullptr;
    result.query_output_zero_initialized = true;
    result.audio_interface_quiescence = false;
    const std::size_t callbacks_before = callbacks_.size();
    audio_state_event(
        result, AudioProcessorLeaseState::audio_processor_query_in_flight,
        events_,
        ",\"requested_iid_raw_tuid_hex\":\"" + tuid_hex(requested) + "\"");
    result.call_in_flight = true;
    const auto query_attempt = events_.call_started(
        "query_audio_processor", "IComponent", -1, nullptr,
        ",\"object_role\":\"again_processor_component\""
        ",\"requested_interface\":\"Steinberg::Vst::IAudioProcessor\""
        ",\"requested_iid_raw_tuid_hex\":\"" + tuid_hex(requested) + "\"");
    callbacks_.begin_plugin_call(query_attempt, "query_audio_processor");
    result.query_attempted = true;
    result.query_result = component_.queryInterface(requested, &output);
    callbacks_.end_plugin_call();
    interface_ = static_cast<Steinberg::Vst::IAudioProcessor*>(output);
    result.query_output_nonnull = interface_ != nullptr;
    result.query_tuple_consistent =
        create_tuple_consistent(result.query_result, interface_);
    events_.call_completed(
        query_attempt, "query_audio_processor", "IComponent", "tresult",
        ",\"result_u32_hex\":\"" +
            u32_hex(static_cast<Steinberg::uint32>(result.query_result)) +
            "\",\"output_nonnull\":" + bool_json(result.query_output_nonnull),
        -1, nullptr,
        ",\"object_role\":\"again_processor_component\""
        ",\"requested_interface\":\"Steinberg::Vst::IAudioProcessor\"");
    result.call_in_flight = false;

    if (interface_ == nullptr) {
        result.pointer_cleared = true;
        result.audio_interface_quiescence = true;
        audio_state_event(
            result,
            AudioProcessorLeaseState::audio_processor_query_returned_without_lease,
            events_,
            ",\"result_u32_hex\":\"" +
                u32_hex(static_cast<Steinberg::uint32>(result.query_result)) +
                "\",\"tuple_consistent\":" +
                bool_json(result.query_tuple_consistent));
        latch(result.primary_exit,
              result.query_result == Steinberg::kResultOk
                  ? kAudioProcessorQueryInconsistent
                  : kAudioProcessorQueryBlocked);
    } else {
        result.lease_acquired = true;
        audio_state_event(
            result, AudioProcessorLeaseState::audio_processor_lease_acquired,
            events_,
            ",\"query_result_u32_hex\":\"" +
                u32_hex(static_cast<Steinberg::uint32>(result.query_result)) +
                "\",\"tuple_consistent\":" +
                bool_json(result.query_tuple_consistent));
        if (result.query_result != Steinberg::kResultOk)
            latch(result.primary_exit, kAudioProcessorQueryInconsistent);

        if (pre_setup_census && result.primary_exit == 0) {
            PreSetupProcessingContractCensus census(component_, *interface_, events_);
            latch(result.primary_exit, census.run());
            result.processing_contract = census.contract();
        }

        if (offline_processing && result.primary_exit == 0) {
            const auto processing = run_offline_processing(component_, *interface_, callbacks_, events_);
            latch(result.primary_exit, processing.success ? 0 : 110);
            if (!processing.quiescent) {
                result.audio_interface_quiescence = false;
                return result; // Supervisor contains unknown ownership; never unload it.
            }
        }

        audio_state_event(
            result, AudioProcessorLeaseState::audio_processor_release_in_flight,
            events_);
        result.call_in_flight = true;
        const auto release_attempt = events_.call_started(
            "release_audio_processor", "IAudioProcessor", -1, nullptr,
            ",\"object_role\":\"again_audio_processor_interface\"");
        callbacks_.begin_plugin_call(release_attempt, "release_audio_processor");
        result.release_attempted = true;
        result.release_result = interface_->release();
        callbacks_.end_plugin_call();
        result.release_returned_ordinary = true;
        events_.call_completed(
            release_attempt, "release_audio_processor", "IAudioProcessor",
            "reference_count",
            ",\"u32_result\":" + std::to_string(result.release_result),
            -1, nullptr,
            ",\"object_role\":\"again_audio_processor_interface\"");
        result.call_in_flight = false;
        interface_ = nullptr;
        result.pointer_cleared = true;
        if (audio_release_matches_component_baseline(result.release_result)) {
            result.audio_interface_quiescence = true;
            audio_state_event(
                result, AudioProcessorLeaseState::audio_processor_lease_retired,
                events_,
                ",\"component_owner_reference_baseline\":1"
                ",\"pointer_cleared\":true");
        } else {
            latch(result.primary_exit, kAudioProcessorReleaseBlocked);
            audio_state_event(
                result,
                AudioProcessorLeaseState::audio_processor_retirement_incomplete,
                events_,
                ",\"release_reference_count\":" +
                    std::to_string(result.release_result) +
                    ",\"pointer_cleared\":true");
        }
    }

    result.callback_ledger_unchanged =
        callbacks_.size() == callbacks_before && callbacks_.healthy();
    if (!result.callback_ledger_unchanged && !(offline_processing && callbacks_.healthy())) {
        result.audio_interface_quiescence = false;
        latch(result.primary_exit,
              result.release_attempted ? kAudioProcessorReleaseBlocked
                                       : kAudioProcessorQueryBlocked);
    }
    events_.lifecycle(
        result.audio_interface_quiescence
            ? "audio_interface_quiescence_proved"
            : "audio_interface_quiescence_unproved",
        ",\"audio_interface_quiescence\":" +
            bool_json(result.audio_interface_quiescence) +
            ",\"audio_processor_state\":\"" +
            audio_processor_state_name(result.state) + "\"" +
            ",\"callback_ledger_unchanged\":" +
            bool_json(result.callback_ledger_unchanged) +
            ",\"primary_blocker\":" +
            (blocker_name(result.primary_exit) == nullptr
                 ? std::string("null")
                 : std::string("\"") + blocker_name(result.primary_exit) + "\""));
        return result;
    } catch (int blocker) {
        throw (result.primary_exit == 0 ? blocker : result.primary_exit);
    } catch (...) {
        // Once output publication fails, no later in-process call is legal.  If
        // a semantic PC0 failure was already latched, preserve it as primary.
        throw (result.primary_exit == 0 ? 99 : result.primary_exit);
    }
}

std::string AudioProcessorLeaseResult::json_fields() const {
    const char* blocker = blocker_name(primary_exit);
    return
        ",\"processing_contract\":" +
        (processing_contract.empty() ? std::string("null") : processing_contract) +
        ",\"audio_processor_lease\":{"
        "\"state\":\"" + std::string(audio_processor_state_name(state)) + "\"" +
        ",\"primary_blocker\":" +
        (blocker == nullptr ? std::string("null")
                            : std::string("\"") + blocker + "\"") +
        ",\"requested_interface\":\"Steinberg::Vst::IAudioProcessor\"" +
        ",\"requested_iid_raw_tuid_hex\":\"" +
        tuid_hex(requested_iid.data()) + "\"" +
        ",\"query\":{"
        "\"output_zero_initialized\":" +
        bool_json(query_output_zero_initialized) +
        ",\"attempted\":" + bool_json(query_attempted) +
        ",\"result_u32_hex\":\"" +
        u32_hex(static_cast<Steinberg::uint32>(query_result)) + "\"" +
        ",\"output_nonnull\":" + bool_json(query_output_nonnull) +
        ",\"tuple_consistent\":" + bool_json(query_tuple_consistent) + "}" +
        ",\"lease_acquired\":" + bool_json(lease_acquired) +
        ",\"release\":{"
        "\"attempted\":" + bool_json(release_attempted) +
        ",\"returned_ordinary\":" + bool_json(release_returned_ordinary) +
        ",\"reference_count\":" + std::to_string(release_result) + "}" +
        ",\"pointer_cleared\":" + bool_json(pointer_cleared) +
        ",\"call_in_flight\":" + bool_json(call_in_flight) +
        ",\"callback_ledger_unchanged\":" +
        bool_json(callback_ledger_unchanged) +
        ",\"audio_interface_quiescence\":" +
        bool_json(audio_interface_quiescence) + "}";
}

std::string ComponentAdmissionResult::json_fields() const {
    std::string callback_records = "[";
    if (callbacks != nullptr) {
        for (std::size_t index = 0; index < callbacks->size(); ++index) {
            if (index != 0) callback_records += ",";
            const auto& record = callbacks->at(index);
            callback_records +=
                "{\"operation\":\"" + std::string(record.operation) + "\"" +
                ",\"origin\":\"" + std::string(record.origin) + "\"" +
                ",\"enclosing_attempt_sequence\":" +
                (record.enclosing_attempt == 0
                     ? std::string("null")
                     : std::to_string(record.enclosing_attempt)) +
                ",\"enclosing_operation\":" +
                (record.enclosing_operation == nullptr
                     ? std::string("null")
                     : std::string("\"") + record.enclosing_operation + "\"");
            if (record.reference_count_present)
                callback_records += ",\"reference_count\":" +
                                    std::to_string(record.reference_count);
            if (record.result_present)
                callback_records += ",\"result_u32_hex\":\"" +
                                    u32_hex(record.result_u32) + "\"" +
                                    ",\"output_null\":" +
                                    bool_json(record.output_null);
            if (record.identifiers_present)
                callback_records += ",\"cid_raw_tuid_hex\":\"" +
                                    tuid_hex(record.cid.data()) + "\"" +
                                    ",\"iid_raw_tuid_hex\":\"" +
                                    tuid_hex(record.iid.data()) + "\"";
            callback_records += "}";
        }
    }
    callback_records += "]";
    const bool callback_closed = callbacks == nullptr || callbacks->closed();
    const bool callback_overflow = callbacks != nullptr && callbacks->overflowed();
    const bool callback_output_failed = callbacks != nullptr && callbacks->output_failed();
    const bool callback_wrong_thread = callbacks != nullptr && callbacks->wrong_thread();
    const bool unexpected_request =
        callbacks != nullptr && callbacks->unexpected_object_request();
    const std::string controller = controller_id_valid
        ? std::string("\"") + tuid_hex(controller_id.data()) + "\""
        : "null";
    const char* blocker = blocker_name(primary_exit);
    return
        ",\"component_session\":{"
        "\"state\":\"" + std::string(component_state_name(state)) + "\"" +
        ",\"primary_blocker\":" +
        (blocker == nullptr ? std::string("null")
                            : std::string("\"") + blocker + "\"") +
        ",\"processor_cid_raw_tuid_hex\":\"" + tuid_hex(processor_cid.data()) + "\"" +
        ",\"requested_iid_raw_tuid_hex\":\"" +
        tuid_hex(requested_iid.data()) + "\"" +
        ",\"create\":{\"attempted\":" + bool_json(create_attempted) +
        ",\"result_u32_hex\":\"" +
        u32_hex(static_cast<Steinberg::uint32>(create_result)) + "\"" +
        ",\"output_nonnull\":" + bool_json(create_output_nonnull) +
        ",\"tuple_consistent\":" + bool_json(create_tuple_consistent) + "}" +
        ",\"controller_id\":{\"attempted\":" +
        bool_json(controller_id_attempted) +
        ",\"buffer_zero_initialized\":" +
        bool_json(controller_id_buffer_zero_initialized) +
        ",\"result_u32_hex\":\"" +
        u32_hex(static_cast<Steinberg::uint32>(controller_id_result)) + "\"" +
        ",\"raw_tuid_hex\":" + controller +
        ",\"matches_expected\":" + bool_json(controller_id_matches) + "}" +
        ",\"initialize\":{\"attempted\":" + bool_json(initialize_attempted) +
        ",\"result_u32_hex\":\"" +
        u32_hex(static_cast<Steinberg::uint32>(initialize_result)) + "\"" +
        ",\"succeeded\":" + bool_json(initialize_succeeded) + "}" +
        ",\"terminate\":{\"attempted\":" + bool_json(terminate_attempted) +
        ",\"returned_ordinary\":" + bool_json(terminate_returned_ordinary) +
        ",\"result_u32_hex\":\"" +
        u32_hex(static_cast<Steinberg::uint32>(terminate_result)) + "\"}" +
        ",\"release\":{\"attempted\":" + bool_json(release_attempted) +
        ",\"returned_ordinary\":" + bool_json(release_returned_ordinary) +
        ",\"reference_count\":" + std::to_string(release_result) +
        ",\"pointer_cleared\":" + bool_json(component_pointer_cleared) + "}" +
        ",\"host\":{\"created\":" + bool_json(host_created) +
        ",\"name\":\"Linux VST Bridge WC0\"" +
        ",\"reference_baseline\":" + std::to_string(host_reference_baseline) +
        ",\"reference_after_initialize\":" +
        std::to_string(host_reference_after_initialize) +
        ",\"reference_after_terminate\":" +
        std::to_string(host_reference_after_terminate) +
        ",\"reference_returned_to_baseline\":" +
        bool_json(host_reference_returned_to_baseline) +
        ",\"owner_release_attempted\":" +
        bool_json(host_owner_release_attempted) +
        ",\"owner_release_result\":" +
        std::to_string(host_owner_release_result) + "}" +
        ",\"callbacks\":{\"capacity\":64,\"record_count\":" +
        std::to_string(callbacks == nullptr ? 0 : callbacks->size()) +
        ",\"closed\":" + bool_json(callback_closed) +
        ",\"overflowed\":" + bool_json(callback_overflow) +
        ",\"output_failed\":" + bool_json(callback_output_failed) +
        ",\"wrong_thread\":" + bool_json(callback_wrong_thread) +
        ",\"unexpected_object_request\":" + bool_json(unexpected_request) +
        ",\"callback_in_flight\":" +
        bool_json(callbacks != nullptr && callbacks->callback_in_flight()) +
        ",\"records\":" + callback_records + "}" +
        ",\"component_call_in_flight\":" + bool_json(component_call_in_flight) +
        ",\"callback_ledger_closed\":" + bool_json(callback_ledger_closed) +
        ",\"audio_processor_session_ran\":" +
        bool_json(audio_processor_session_ran) +
        audio_processor.json_fields() +
        ",\"object_quiescence\":" + bool_json(object_quiescence) +
        ",\"inherited_shutdown_permitted\":" +
        bool_json(inherited_shutdown_permitted) + "}";
}

ComponentAdmissionResult admit_component(Steinberg::IPluginFactory* factory,
                                          EventWriter& events,
                                          ComponentCase component_case,
                                          bool pre_setup_census, bool offline_processing) {
    ComponentAdmissionResult result;
    try {
    const Steinberg::int8* processor = component_case == ComponentCase::unknown_processor
        ? kUnknownProcessor
        : kAgainProcessor;
    const Steinberg::int8* requested =
        component_case == ComponentCase::unsupported_interface
            ? INLINE_UID_OF(Steinberg::Vst::IHostApplication)
            : INLINE_UID_OF(Steinberg::Vst::IComponent);
    std::memcpy(result.processor_cid.data(), processor, 16);
    std::memcpy(result.requested_iid.data(), requested, 16);

    void* component_output = nullptr;
    Steinberg::Vst::IComponent* component = nullptr;
    state_event(result, ComponentState::component_create_in_flight, events);
    result.component_call_in_flight = true;
    const auto create_attempt = events.call_started(
        "create_component", "IPluginFactory",
        -1, nullptr,
        ",\"object_role\":\"again_processor_component\"" +
        std::string(",\"processor_cid_raw_tuid_hex\":\"") + tuid_hex(processor) +
        "\",\"requested_iid_raw_tuid_hex\":\"" + tuid_hex(requested) + "\"");
    result.create_attempted = true;
    result.create_result = factory->createInstance(
        processor, requested, &component_output);
    component = static_cast<Steinberg::Vst::IComponent*>(component_output);
    result.create_output_nonnull = component != nullptr;
    result.create_tuple_consistent =
        create_tuple_consistent(result.create_result, component);
    events.call_completed(
        create_attempt, "create_component", "IPluginFactory", "tresult",
        ",\"result_u32_hex\":\"" +
            u32_hex(static_cast<Steinberg::uint32>(result.create_result)) +
            "\",\"output_nonnull\":" + bool_json(result.create_output_nonnull),
        -1, nullptr, ",\"object_role\":\"again_processor_component\"");
    result.component_call_in_flight = false;

    if (result.create_result != Steinberg::kResultOk || component == nullptr) {
        latch(result.primary_exit, kCreateBlocked);
        if (component == nullptr) {
            state_event(result, ComponentState::component_absent, events);
        } else {
            state_event(result, ComponentState::component_created, events,
                        ",\"anomalous_failure_nonnull\":true");
        }
    } else {
        state_event(result, ComponentState::component_created, events);
    }

    if (result.primary_exit == 0 && component != nullptr) {
        state_event(result, ComponentState::controller_id_in_flight, events);
        result.component_call_in_flight = true;
        std::fill(result.controller_id.begin(), result.controller_id.end(), 0);
        result.controller_id_buffer_zero_initialized = true;
        const auto controller_attempt = events.call_started(
            "get_controller_class_id", "IComponent", -1, nullptr,
            ",\"object_role\":\"again_processor_component\"");
        result.controller_id_attempted = true;
        result.controller_id_result =
            component->getControllerClassId(result.controller_id.data());
        events.call_completed(
            controller_attempt, "get_controller_class_id", "IComponent", "tresult",
            ",\"result_u32_hex\":\"" +
                u32_hex(static_cast<Steinberg::uint32>(result.controller_id_result)) +
                "\"" +
                (result.controller_id_result == Steinberg::kResultTrue
                     ? std::string(",\"controller_cid_raw_tuid_hex\":\"") +
                           tuid_hex(result.controller_id.data()) + "\""
                     : std::string(",\"controller_cid_raw_tuid_hex\":null")),
            -1, nullptr, ",\"object_role\":\"again_processor_component\"");
        result.component_call_in_flight = false;
        if (result.controller_id_result != Steinberg::kResultTrue) {
            latch(result.primary_exit, kControllerIdBlocked);
            state_event(result, ComponentState::component_created, events,
                        ",\"controller_id_valid\":false");
        } else {
            result.controller_id_valid = true;
            result.controller_id_matches =
                std::memcmp(result.controller_id.data(), kAgainController, 16) == 0;
            if (!result.controller_id_matches) {
                latch(result.primary_exit, kControllerIdMismatch);
                state_event(result, ComponentState::component_created, events,
                            ",\"controller_id_mismatch\":true");
            } else {
                state_event(result, ComponentState::controller_id_verified, events,
                            ",\"controller_cid_raw_tuid_hex\":\"" +
                                tuid_hex(result.controller_id.data()) + "\"");
            }
        }
    }

    result.callbacks = std::make_unique<HostCallbackSink>(&events, GetCurrentThreadId());
    HostCallbackSink& callbacks = *result.callbacks;
    MinimalHostApplication* host = nullptr;
    if (result.primary_exit == 0 && component != nullptr) {
        host = new MinimalHostApplication(callbacks, offline_processing);
        result.host_created = true;
        state_event(result, ComponentState::host_context_ready, events,
                    ",\"host_name\":\"Linux VST Bridge WC0\",\"reference_baseline\":1");
        state_event(result, ComponentState::component_initialize_in_flight, events);
        result.component_call_in_flight = true;
        const auto initialize_attempt = events.call_started(
            "initialize_component", "IComponent", -1, nullptr,
            ",\"object_role\":\"again_processor_component\"");
        callbacks.begin_plugin_call(initialize_attempt, "initialize_component");
        result.initialize_attempted = true;
        result.initialize_result = component->initialize(host);
        callbacks.end_plugin_call();
        result.host_reference_after_initialize = callbacks.last_reference_count();
        events.call_completed(
            initialize_attempt, "initialize_component", "IComponent", "tresult",
            ",\"result_u32_hex\":\"" +
                u32_hex(static_cast<Steinberg::uint32>(result.initialize_result)) +
                "\",\"host_reference_count\":" +
                std::to_string(result.host_reference_after_initialize),
            -1, nullptr, ",\"object_role\":\"again_processor_component\"");
        result.component_call_in_flight = false;
        result.initialize_succeeded = result.initialize_result == Steinberg::kResultOk;
        if (!result.initialize_succeeded) {
            latch(result.primary_exit, kInitializeBlocked);
            state_event(result, ComponentState::host_context_ready, events,
                        ",\"initialize_failed\":true");
        } else {
            state_event(result, ComponentState::component_initialized, events,
                        ",\"host_reference_count\":" +
                            std::to_string(result.host_reference_after_initialize));
            if (callbacks.unexpected_object_request() || !callbacks.healthy() ||
                result.host_reference_after_initialize != 2 ||
                callbacks.plugin_callback_count("addRef") != 1) {
                latch(result.primary_exit, kHostContextBlocked);
            }
        }
    }

    if (result.initialize_succeeded && component != nullptr) {
        result.audio_processor_session_ran = true;
        AudioProcessorInterfaceLease lease(*component, callbacks, events);
        result.audio_processor = lease.acquire_and_retire(pre_setup_census, offline_processing);
        latch(result.primary_exit, result.audio_processor.primary_exit);
    }

    const bool audio_retirement_permitted =
        !result.initialize_succeeded ||
        (result.audio_processor_session_ran &&
         result.audio_processor.audio_interface_quiescence);

    if (result.initialize_succeeded && component != nullptr &&
        audio_retirement_permitted) {
        state_event(result, ComponentState::component_terminate_in_flight, events);
        result.component_call_in_flight = true;
        const auto terminate_attempt = events.call_started(
            "terminate_component", "IComponent", -1, nullptr,
            ",\"object_role\":\"again_processor_component\"");
        callbacks.begin_plugin_call(terminate_attempt, "terminate_component");
        result.terminate_attempted = true;
        result.terminate_result = component->terminate();
        callbacks.end_plugin_call();
        result.terminate_returned_ordinary = true;
        result.host_reference_after_terminate = callbacks.last_reference_count();
        events.call_completed(
            terminate_attempt, "terminate_component", "IComponent", "tresult",
            ",\"result_u32_hex\":\"" +
                u32_hex(static_cast<Steinberg::uint32>(result.terminate_result)) +
                "\",\"host_reference_count\":" +
                std::to_string(result.host_reference_after_terminate),
            -1, nullptr, ",\"object_role\":\"again_processor_component\"");
        result.component_call_in_flight = false;
        if (result.terminate_result != Steinberg::kResultOk) {
            latch(result.primary_exit, kTerminateBlocked);
            state_event(result, ComponentState::component_initialized, events,
                        ",\"terminate_failed\":true,\"result_u32_hex\":\"" +
                            u32_hex(static_cast<Steinberg::uint32>(
                                result.terminate_result)) +
                            "\"");
        } else {
            state_event(result, ComponentState::component_terminated, events,
                        ",\"result_u32_hex\":\"" +
                            u32_hex(static_cast<Steinberg::uint32>(
                                result.terminate_result)) +
                            "\"");
        }
    }

    if (component != nullptr && audio_retirement_permitted) {
        state_event(result, ComponentState::component_release_in_flight, events);
        result.component_call_in_flight = true;
        const auto release_attempt = events.call_started(
            "release_component", "IComponent", -1, nullptr,
            ",\"object_role\":\"again_processor_component\"");
        result.release_attempted = true;
        result.release_result = component->release();
        result.release_returned_ordinary = true;
        events.call_completed(
            release_attempt, "release_component", "IComponent", "reference_count",
            ",\"u32_result\":" + std::to_string(result.release_result),
            -1, nullptr, ",\"object_role\":\"again_processor_component\"");
        result.component_call_in_flight = false;
        component = nullptr;
        result.component_pointer_cleared = true;
        if (result.release_result == 0) {
            state_event(result, ComponentState::component_released, events,
                        ",\"component_absence_proved\":true");
        } else {
            latch(result.primary_exit, kReleaseBlocked);
            state_event(result, ComponentState::component_retirement_incomplete, events,
                        ",\"release_reference_count\":" +
                            std::to_string(result.release_result));
        }
    }

    if (host != nullptr && audio_retirement_permitted) {
        result.host_reference_returned_to_baseline =
            result.initialize_succeeded
                ? (result.terminate_returned_ordinary &&
                   result.host_reference_after_terminate ==
                       result.host_reference_baseline)
                : (result.host_reference_after_initialize ==
                   result.host_reference_baseline);
        if (result.initialize_succeeded &&
            (!result.host_reference_returned_to_baseline ||
             callbacks.plugin_callback_count("release") != 1)) {
            latch(result.primary_exit, kHostContextBlocked);
        }
        if (!result.initialize_succeeded &&
            !result.host_reference_returned_to_baseline)
            latch(result.primary_exit, kHostContextBlocked);
        callbacks.begin_owner_call();
        result.host_owner_release_attempted = true;
        result.host_owner_release_result = host->release();
        host = nullptr;
        callbacks.end_owner_call();
        if (result.host_owner_release_result != 0 || !callbacks.healthy())
            latch(result.primary_exit, kHostContextBlocked);
    } else if (host == nullptr) {
        result.host_reference_returned_to_baseline = true;
    }

    callbacks.close();
    const bool no_component_path = !result.create_output_nonnull;
    const bool component_retired = no_component_path ||
        (result.release_returned_ordinary && result.release_result == 0 &&
         result.component_pointer_cleared);
    result.callback_ledger_closed = callbacks.closed();
    const bool host_retired = !result.host_created ||
        (result.host_owner_release_attempted && result.host_owner_release_result == 0 &&
         result.host_reference_returned_to_baseline && result.callback_ledger_closed);
    const bool initialized_path_exact = !result.initialize_succeeded ||
        (result.terminate_attempted && result.terminate_returned_ordinary);
    result.object_quiescence = component_retired && host_retired &&
        initialized_path_exact && !result.component_call_in_flight &&
        !callbacks.callback_in_flight() && result.callback_ledger_closed &&
        result.audio_processor.audio_interface_quiescence;
    result.inherited_shutdown_permitted = result.object_quiescence;
    events.lifecycle(
        result.object_quiescence ? "object_quiescence_proved"
                                 : "object_quiescence_unproved",
        ",\"object_quiescence\":" + bool_json(result.object_quiescence) +
            ",\"component_state\":\"" + component_state_name(result.state) + "\"" +
            ",\"primary_blocker\":" +
            (blocker_name(result.primary_exit) == nullptr
                 ? std::string("null")
                 : std::string("\"") + blocker_name(result.primary_exit) + "\""));
        return result;
    } catch (int blocker) {
        throw (result.primary_exit == 0 ? blocker : result.primary_exit);
    } catch (...) {
        // Physical process containment owns every object after an unpaired
        // writer boundary.  Preserve an earlier lifecycle blocker if present.
        throw (result.primary_exit == 0 ? 99 : result.primary_exit);
    }
}

bool run_component_owner_regressions() noexcept {
    try {
        static constexpr ComponentState states[] = {
            ComponentState::component_absent,
            ComponentState::component_create_in_flight,
            ComponentState::component_created,
            ComponentState::controller_id_in_flight,
            ComponentState::controller_id_verified,
            ComponentState::host_context_ready,
            ComponentState::component_initialize_in_flight,
            ComponentState::component_initialized,
            ComponentState::component_terminate_in_flight,
            ComponentState::component_terminated,
            ComponentState::component_release_in_flight,
            ComponentState::component_released,
            ComponentState::component_retirement_incomplete,
        };
        static constexpr const char* names[] = {
            "component_absent",
            "component_create_in_flight",
            "component_created",
            "controller_id_in_flight",
            "controller_id_verified",
            "host_context_ready",
            "component_initialize_in_flight",
            "component_initialized",
            "component_terminate_in_flight",
            "component_terminated",
            "component_release_in_flight",
            "component_released",
            "component_retirement_incomplete",
        };
        static_assert(sizeof(states) / sizeof(states[0]) == 13);
        for (std::size_t index = 0; index < 13; ++index) {
            if (std::strcmp(component_state_name(states[index]), names[index]) != 0)
                return false;
        }
        static constexpr AudioProcessorLeaseState audio_states[] = {
            AudioProcessorLeaseState::audio_processor_absent,
            AudioProcessorLeaseState::audio_processor_query_in_flight,
            AudioProcessorLeaseState::audio_processor_query_returned_without_lease,
            AudioProcessorLeaseState::audio_processor_lease_acquired,
            AudioProcessorLeaseState::audio_processor_release_in_flight,
            AudioProcessorLeaseState::audio_processor_lease_retired,
            AudioProcessorLeaseState::audio_processor_retirement_incomplete,
            AudioProcessorLeaseState::audio_processor_ownership_unknown,
        };
        static constexpr const char* audio_names[] = {
            "audio_processor_absent",
            "audio_processor_query_in_flight",
            "audio_processor_query_returned_without_lease",
            "audio_processor_lease_acquired",
            "audio_processor_release_in_flight",
            "audio_processor_lease_retired",
            "audio_processor_retirement_incomplete",
            "audio_processor_ownership_unknown",
        };
        static_assert(sizeof(audio_states) / sizeof(audio_states[0]) == 8);
        for (std::size_t index = 0; index < 8; ++index) {
            if (std::strcmp(audio_processor_state_name(audio_states[index]),
                            audio_names[index]) != 0)
                return false;
        }
        if (!create_tuple_consistent(Steinberg::kResultOk,
                                     reinterpret_cast<void*>(1)) ||
            !create_tuple_consistent(Steinberg::kResultFalse, nullptr) ||
            create_tuple_consistent(Steinberg::kResultOk, nullptr) ||
            create_tuple_consistent(Steinberg::kResultFalse,
                                    reinterpret_cast<void*>(1)))
            return false;
        if (!audio_release_matches_component_baseline(1) ||
            audio_release_matches_component_baseline(0) ||
            audio_release_matches_component_baseline(2) ||
            audio_release_matches_component_baseline(
                (std::numeric_limits<Steinberg::uint32>::max)()))
            return false;

        HostCallbackSink sink(nullptr, GetCurrentThreadId());
        auto* host = new MinimalHostApplication(sink);
        Steinberg::Vst::String128 name{};
        if (host->getName(name) != Steinberg::kResultOk) return false;
        constexpr char expected[] = "Linux VST Bridge WC0";
        for (std::size_t index = 0; index < sizeof(expected); ++index) {
            if (name[index] != static_cast<Steinberg::Vst::TChar>(expected[index]))
                return false;
        }
        void* unknown_identity = reinterpret_cast<void*>(1);
        if (host->queryInterface(INLINE_UID_OF(Steinberg::FUnknown),
                                 &unknown_identity) !=
                Steinberg::kResultOk ||
            unknown_identity != static_cast<Steinberg::Vst::IHostApplication*>(host))
            return false;
        void* host_identity = reinterpret_cast<void*>(1);
        if (host->queryInterface(INLINE_UID_OF(Steinberg::Vst::IHostApplication),
                                 &host_identity) != Steinberg::kResultOk ||
            host_identity != unknown_identity)
            return false;
        static_cast<Steinberg::FUnknown*>(unknown_identity)->release();
        static_cast<Steinberg::FUnknown*>(host_identity)->release();
        Steinberg::TUID unsupported =
            INLINE_UID(0x01020304, 0x05060708, 0x11121314, 0x15161718);
        void* absent = reinterpret_cast<void*>(1);
        if (host->queryInterface(unsupported, &absent) != Steinberg::kNoInterface ||
            absent != nullptr)
            return false;
        void* created = reinterpret_cast<void*>(1);
        if (host->createInstance(unsupported, unsupported, &created) !=
                Steinberg::kResultFalse ||
            created != nullptr || !sink.unexpected_object_request())
            return false;
        int primary = kControllerIdMismatch;
        latch(primary, kReleaseBlocked);
        if (primary != kControllerIdMismatch) return false;
        int audio_primary = kAudioProcessorQueryInconsistent;
        latch(audio_primary, kAudioProcessorReleaseBlocked);
        if (audio_primary != kAudioProcessorQueryInconsistent) return false;
        sink.begin_owner_call();
        const auto final = host->release();
        host = nullptr;
        sink.end_owner_call();
        return final == 0 && sink.close() && sink.closed() &&
               sink.last_reference_count() == 0;
    } catch (...) {
        return false;
    }
}

} // namespace linux_vst_bridge::wf0
