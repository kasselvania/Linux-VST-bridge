#pragma once
// Original test-only SDK sinks. Fixed storage is constructed before activation;
// addPoint/addEvent never allocate. No transport or product result decoder.
#include "pluginterfaces/vst/ivstparameterchanges.h"
#include "pluginterfaces/vst/ivstevents.h"
#include <array>
#include <cmath>
namespace CompletionSDK {
using namespace Steinberg;
using namespace Steinberg::Vst;
template<class Interface> tresult query(const TUID iid, void** out, Interface* self) {
    if (!out) return kInvalidArgument;
    *out = nullptr;
    if (!FUnknownPrivate::iidEqual(iid, Interface::iid)
        && !FUnknownPrivate::iidEqual(iid, FUnknown::iid)) return kNoInterface;
    *out = self; self->addRef(); return kResultOk;
}
class Queue final : public IParamValueQueue {
    struct Point { int32 offset{}; ParamValue value{}; };
    std::array<Point, 128> points{};
    int32 size = 0;
    ParamID id = 0;
    uint32 references = 1;
public:
    void reset(ParamID value) { id = value; size = 0; }
    bool quiescent() const { return references == 1; }
    tresult PLUGIN_API queryInterface(const TUID iid, void** out) override { return query(iid, out, this); }
    uint32 PLUGIN_API addRef() override { return ++references; }
    uint32 PLUGIN_API release() override { return --references; } // lexical owner
    ParamID PLUGIN_API getParameterId() override { return id; }
    int32 PLUGIN_API getPointCount() override { return size; }
    tresult PLUGIN_API getPoint(int32 index, int32& offset, ParamValue& value) override {
        if (index < 0 || index >= size) return kInvalidArgument;
        offset = points[size_t(index)].offset; value = points[size_t(index)].value; return kResultOk;
    }
    tresult PLUGIN_API addPoint(int32 offset, ParamValue value, int32& index) override {
        if (size == int32(points.size()) || offset < 0 || !std::isfinite(value)) return kResultFalse;
        index = size; points[size_t(size++)] = {offset, value}; return kResultOk;
    }
};
class Parameters final : public IParameterChanges {
    std::array<Queue, 8> queues{};
    int32 size = 0;
    uint32 references = 1;
public:
    void clear() { size = 0; }
    bool quiescent() const {
        if (references != 1) return false;
        for (const auto& queue : queues) if (!queue.quiescent()) return false;
        return true;
    }
    tresult PLUGIN_API queryInterface(const TUID iid, void** out) override { return query(iid, out, this); }
    uint32 PLUGIN_API addRef() override { return ++references; }
    uint32 PLUGIN_API release() override { return --references; }
    int32 PLUGIN_API getParameterCount() override { return size; }
    IParamValueQueue* PLUGIN_API getParameterData(int32 index) override {
        return index >= 0 && index < size ? &queues[size_t(index)] : nullptr;
    }
    IParamValueQueue* PLUGIN_API addParameterData(const ParamID& id, int32& index) override {
        for (int32 i = 0; i < size; ++i) if (queues[size_t(i)].getParameterId() == id) {
            index = i; return &queues[size_t(i)];
        }
        if (size == int32(queues.size())) return nullptr;
        index = size++; auto& queue = queues[size_t(index)]; queue.reset(id); return &queue;
    }
};
class Events final : public IEventList {
    std::array<Event, 512> events{};
    int32 size = 0;
    uint32 references = 1;
public:
    void clear() { size = 0; }
    bool quiescent() const { return references == 1; }
    tresult PLUGIN_API queryInterface(const TUID iid, void** out) override { return query(iid, out, this); }
    uint32 PLUGIN_API addRef() override { return ++references; }
    uint32 PLUGIN_API release() override { return --references; }
    int32 PLUGIN_API getEventCount() override { return size; }
    tresult PLUGIN_API getEvent(int32 index, Event& event) override {
        if (index < 0 || index >= size) return kInvalidArgument;
        event = events[size_t(index)]; return kResultOk;
    }
    tresult PLUGIN_API addEvent(Event& event) override {
        // The declared fixture returns notes and inline MIDI CC only; refusing
        // pointer-bearing payloads prevents an accidental borrowed-data oracle.
        if (size == int32(events.size()) || (event.type != Event::kNoteOnEvent
            && event.type != Event::kNoteOffEvent && event.type != Event::kLegacyMIDICCOutEvent)) return kResultFalse;
        events[size_t(size++)] = event; return kResultOk;
    }
};
} // namespace CompletionSDK
