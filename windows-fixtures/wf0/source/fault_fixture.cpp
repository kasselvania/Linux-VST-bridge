#include "pluginterfaces/base/ipluginbase.h"

#include <windows.h>

#include <atomic>
#include <cstring>
#include <cstdlib>

#ifndef WF0_FAULT_KIND
#error WF0_FAULT_KIND must be defined
#endif

namespace {

constexpr int kMissingFactory = 0;
constexpr int kNullFactory = 1;
constexpr int kNoEntry = 2;
constexpr int kFactory1Only = 3;
constexpr int kFactory2Only = 4;
constexpr int kFactory3Fallback = 5;
constexpr int kInitFalse = 6;
constexpr int kFactoryInfoFalse = 7;
constexpr int kCountNegative = 8;
constexpr int kCountExcessive = 9;
constexpr int kClassInfoFalse = 10;
constexpr int kDuplicateClassId = 11;
constexpr int kHangEntry = 12;
constexpr int kHangFactory = 13;
constexpr int kHangClass = 14;
constexpr int kCrashEntry = 15;
constexpr int kCrashFactory = 16;
constexpr int kCrashClass = 17;
constexpr int kHangRelease = 18;
constexpr int kCrashRelease = 19;
constexpr int kExitFalse = 20;
constexpr int kTripwire = 21;

[[noreturn]] void hang_forever() {
    for (;;) Sleep(1000);
}

[[noreturn]] void crash_now() {
    RaiseException(0xE057F000u + static_cast<unsigned>(WF0_FAULT_KIND),
                   EXCEPTION_NONCONTINUABLE, 0, nullptr);
    std::abort();
}

template <typename Unit, std::size_t Size>
void copy_text(Unit (&destination)[Size], const char* source) {
    std::size_t index = 0;
    for (; index + 1 < Size && source[index] != '\0'; ++index)
        destination[index] = static_cast<Unit>(static_cast<unsigned char>(source[index]));
    destination[index] = 0;
}

const Steinberg::TUID kClassA = INLINE_UID(0x10203040, 0x50607080, 0x90A0B0C0, 0xD0E0F001);
const Steinberg::TUID kClassB = INLINE_UID(0x10203041, 0x50607080, 0x90A0B0C0, 0xD0E0F002);

class FaultFactory final : public Steinberg::IPluginFactory3 {
public:
    Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID iid,
                                                  void** object) override {
        if (object == nullptr) return Steinberg::kInvalidArgument;
        *object = nullptr;
        if (Steinberg::FUnknownPrivate::iidEqual(iid, INLINE_UID_OF(Steinberg::FUnknown)) ||
            Steinberg::FUnknownPrivate::iidEqual(iid,
                                                  INLINE_UID_OF(Steinberg::IPluginFactory))) {
            *object = static_cast<Steinberg::IPluginFactory*>(this);
        } else if (WF0_FAULT_KIND != kFactory1Only &&
                   Steinberg::FUnknownPrivate::iidEqual(
                       iid, INLINE_UID_OF(Steinberg::IPluginFactory2))) {
            *object = static_cast<Steinberg::IPluginFactory2*>(this);
        } else if (WF0_FAULT_KIND != kFactory1Only && WF0_FAULT_KIND != kFactory2Only &&
                   Steinberg::FUnknownPrivate::iidEqual(
                       iid, INLINE_UID_OF(Steinberg::IPluginFactory3))) {
            *object = static_cast<Steinberg::IPluginFactory3*>(this);
        }
        if (*object == nullptr) return Steinberg::kNoInterface;
        addRef();
        return Steinberg::kResultOk;
    }

    Steinberg::uint32 PLUGIN_API addRef() override {
        return static_cast<Steinberg::uint32>(++references_);
    }

    Steinberg::uint32 PLUGIN_API release() override {
        if (WF0_FAULT_KIND == kHangRelease) hang_forever();
        if (WF0_FAULT_KIND == kCrashRelease) crash_now();
        const auto remaining = --references_;
        if (remaining == 0) delete this;
        return static_cast<Steinberg::uint32>(remaining);
    }

    Steinberg::tresult PLUGIN_API getFactoryInfo(Steinberg::PFactoryInfo* info) override {
        if (WF0_FAULT_KIND == kFactoryInfoFalse) return Steinberg::kResultFalse;
        if (info == nullptr) return Steinberg::kInvalidArgument;
        *info = Steinberg::PFactoryInfo{};
        copy_text(info->vendor, "Linux VST Bridge WF0");
        copy_text(info->url, "https://example.invalid/wf0");
        copy_text(info->email, "wf0@example.invalid");
        info->flags = Steinberg::PFactoryInfo::kUnicode;
        return Steinberg::kResultOk;
    }

    Steinberg::int32 PLUGIN_API countClasses() override {
        if (WF0_FAULT_KIND == kCountNegative) return -1;
        if (WF0_FAULT_KIND == kCountExcessive) return 257;
        if (WF0_FAULT_KIND == kDuplicateClassId) return 2;
        return 1;
    }

    Steinberg::tresult PLUGIN_API getClassInfo(Steinberg::int32 index,
                                                Steinberg::PClassInfo* info) override {
        if (WF0_FAULT_KIND == kClassInfoFalse) return Steinberg::kResultFalse;
        if (info == nullptr || index < 0 || index >= countClasses())
            return Steinberg::kInvalidArgument;
        *info = Steinberg::PClassInfo{};
        const auto& id = (index == 0 || WF0_FAULT_KIND == kDuplicateClassId) ? kClassA : kClassB;
        std::memcpy(info->cid, id, sizeof(Steinberg::TUID));
        info->cardinality = Steinberg::PClassInfo::kManyInstances;
        copy_text(info->category, "Audio Module Class");
        copy_text(info->name, "WF0 fault fixture");
        return Steinberg::kResultOk;
    }

    Steinberg::tresult PLUGIN_API createInstance(Steinberg::FIDString,
                                                  Steinberg::FIDString, void** object) override {
        if (object != nullptr) *object = nullptr;
        if (WF0_FAULT_KIND == kTripwire) {
            HANDLE marker = CreateFileW(L"C:\\wf0\\session\\create-instance-tripwire.marker",
                                        GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                                        FILE_ATTRIBUTE_NORMAL, nullptr);
            if (marker != INVALID_HANDLE_VALUE) CloseHandle(marker);
        }
        return Steinberg::kResultFalse;
    }

    Steinberg::tresult PLUGIN_API getClassInfo2(Steinberg::int32 index,
                                                 Steinberg::PClassInfo2* info) override {
        if (WF0_FAULT_KIND == kClassInfoFalse) return Steinberg::kResultFalse;
        if (info == nullptr) return Steinberg::kInvalidArgument;
        Steinberg::PClassInfo basic{};
        const auto result = getClassInfo(index, &basic);
        if (result != Steinberg::kResultOk) return result;
        *info = Steinberg::PClassInfo2{};
        std::memcpy(info->cid, basic.cid, sizeof(Steinberg::TUID));
        info->cardinality = basic.cardinality;
        copy_text(info->category, "Audio Module Class");
        copy_text(info->name, "WF0 fault fixture");
        info->classFlags = 1;
        copy_text(info->subCategories, "Fx");
        copy_text(info->vendor, "Linux VST Bridge");
        copy_text(info->version, "1.0.0");
        copy_text(info->sdkVersion, "VST 3.8.1");
        return Steinberg::kResultOk;
    }

    Steinberg::tresult PLUGIN_API getClassInfoUnicode(
        Steinberg::int32 index, Steinberg::PClassInfoW* info) override {
        if (WF0_FAULT_KIND == kFactory3Fallback || WF0_FAULT_KIND == kClassInfoFalse)
            return Steinberg::kResultFalse;
        if (WF0_FAULT_KIND == kHangClass) hang_forever();
        if (WF0_FAULT_KIND == kCrashClass) crash_now();
        if (info == nullptr) return Steinberg::kInvalidArgument;
        Steinberg::PClassInfo basic{};
        const auto result = getClassInfo(index, &basic);
        if (result != Steinberg::kResultOk) return result;
        *info = Steinberg::PClassInfoW{};
        std::memcpy(info->cid, basic.cid, sizeof(Steinberg::TUID));
        info->cardinality = basic.cardinality;
        copy_text(info->category, "Audio Module Class");
        copy_text(info->name, "WF0 fault fixture");
        info->classFlags = 1;
        copy_text(info->subCategories, "Fx");
        copy_text(info->vendor, "Linux VST Bridge");
        copy_text(info->version, "1.0.0");
        copy_text(info->sdkVersion, "VST 3.8.1");
        return Steinberg::kResultOk;
    }

    Steinberg::tresult PLUGIN_API setHostContext(Steinberg::FUnknown*) override {
        return Steinberg::kNotImplemented;
    }

private:
    std::atomic<long> references_{1};
};

} // namespace

#if WF0_FAULT_KIND != 2
extern "C" __declspec(dllexport) bool InitDll() {
    if (WF0_FAULT_KIND == kHangEntry) hang_forever();
    if (WF0_FAULT_KIND == kCrashEntry) crash_now();
    return WF0_FAULT_KIND != kInitFalse;
}

extern "C" __declspec(dllexport) bool ExitDll() {
    return WF0_FAULT_KIND != kExitFalse;
}
#endif

#if WF0_FAULT_KIND != 0
extern "C" __declspec(dllexport) Steinberg::IPluginFactory* PLUGIN_API GetPluginFactory() {
    if (WF0_FAULT_KIND == kHangFactory) hang_forever();
    if (WF0_FAULT_KIND == kCrashFactory) crash_now();
    if (WF0_FAULT_KIND == kNullFactory) return nullptr;
    return new FaultFactory();
}
#endif
