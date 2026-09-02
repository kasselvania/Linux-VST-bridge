#include "pluginterfaces/base/ipluginbase.h"
#include "pluginterfaces/vst/ivstcomponent.h"
#include "pluginterfaces/vst/ivsthostapplication.h"

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

#ifdef WC0_FAULT_KIND
constexpr int kWc0CreateFailureNull = 0;
constexpr int kWc0CreateSuccessNull = 1;
constexpr int kWc0CreateFailureNonnull = 2;
constexpr int kWc0ControllerIdFailure = 3;
constexpr int kWc0ControllerIdMismatch = 4;
constexpr int kWc0InitializeFailure = 5;
constexpr int kWc0InitializeHang = 6;
constexpr int kWc0InitializeCrash = 7;
constexpr int kWc0TerminateFailure = 8;
constexpr int kWc0TerminateHang = 9;
constexpr int kWc0TerminateCrash = 10;
constexpr int kWc0ReleaseNonzero = 11;
constexpr int kWc0ReleaseHang = 12;
constexpr int kWc0ReleaseCrash = 13;
constexpr int kWc0HostObjectRequest = 14;
constexpr int kWc0HostReferenceLeak = 15;
#endif

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

const Steinberg::TUID kProcessor =
    INLINE_UID(0x84E8DE5F, 0x92554F53, 0x96FAE413, 0x3C935A18);
const Steinberg::TUID kController =
    INLINE_UID(0xD39D5B65, 0xD7AF42FA, 0x843F4AC8, 0x41EB04F0);
const Steinberg::TUID kWrongController =
    INLINE_UID(0x10203040, 0x50607080, 0x90A0B0C0, 0xD0E0F001);
const Steinberg::TUID kUnsupportedHostObject =
    INLINE_UID(0x01020304, 0x05060708, 0x11121314, 0x15161718);
const Steinberg::TUID kClassA =
    INLINE_UID(0x10203040, 0x50607080, 0x90A0B0C0, 0xD0E0F001);
const Steinberg::TUID kClassB =
    INLINE_UID(0x10203041, 0x50607080, 0x90A0B0C0, 0xD0E0F002);

void forbidden_method_tripwire() {
    HANDLE marker = CreateFileW(
        L"C:\\wf0\\session\\forbidden-component-method.marker",
        GENERIC_WRITE, 0, nullptr, CREATE_NEW, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (marker != INVALID_HANDLE_VALUE) CloseHandle(marker);
}

#ifdef WC0_FAULT_KIND
class FaultComponent final : public Steinberg::Vst::IComponent {
public:
    FaultComponent()
        : references_(WC0_FAULT_KIND == kWc0ReleaseNonzero ? 2 : 1) {}

    Steinberg::tresult PLUGIN_API queryInterface(const Steinberg::TUID iid,
                                                  void** object) override {
        if (object == nullptr) return Steinberg::kInvalidArgument;
        *object = nullptr;
        if (Steinberg::FUnknownPrivate::iidEqual(
                iid, INLINE_UID_OF(Steinberg::FUnknown)) ||
            Steinberg::FUnknownPrivate::iidEqual(
                iid, INLINE_UID_OF(Steinberg::IPluginBase)) ||
            Steinberg::FUnknownPrivate::iidEqual(
                iid, INLINE_UID_OF(Steinberg::Vst::IComponent))) {
            *object = static_cast<Steinberg::Vst::IComponent*>(this);
            addRef();
            return Steinberg::kResultOk;
        }
        return Steinberg::kNoInterface;
    }

    Steinberg::uint32 PLUGIN_API addRef() override {
        return static_cast<Steinberg::uint32>(++references_);
    }

    Steinberg::uint32 PLUGIN_API release() override {
        if (WC0_FAULT_KIND == kWc0ReleaseHang) hang_forever();
        if (WC0_FAULT_KIND == kWc0ReleaseCrash) crash_now();
        const long remaining = --references_;
        if (remaining == 0) delete this;
        return static_cast<Steinberg::uint32>(remaining);
    }

    Steinberg::tresult PLUGIN_API initialize(Steinberg::FUnknown* context) override {
        if (WC0_FAULT_KIND == kWc0InitializeHang) hang_forever();
        if (WC0_FAULT_KIND == kWc0InitializeCrash) crash_now();
        if (context == nullptr || initialized_) return Steinberg::kInvalidArgument;
        if (WC0_FAULT_KIND == kWc0InitializeFailure)
            return Steinberg::kResultFalse;
        context_ = context;
        context_->addRef();
        initialized_ = true;
        if (WC0_FAULT_KIND == kWc0HostObjectRequest) {
            auto* host = static_cast<Steinberg::Vst::IHostApplication*>(context);
            void* object = reinterpret_cast<void*>(1);
            const auto outcome = host->createInstance(
                const_cast<Steinberg::int8*>(kUnsupportedHostObject),
                const_cast<Steinberg::int8*>(kUnsupportedHostObject), &object);
            if (outcome != Steinberg::kResultFalse || object != nullptr)
                return Steinberg::kResultFalse;
        }
        return Steinberg::kResultOk;
    }

    Steinberg::tresult PLUGIN_API terminate() override {
        if (WC0_FAULT_KIND == kWc0TerminateHang) hang_forever();
        if (WC0_FAULT_KIND == kWc0TerminateCrash) crash_now();
        if (!initialized_) return Steinberg::kResultFalse;
        if (WC0_FAULT_KIND != kWc0HostReferenceLeak) context_->release();
        context_ = nullptr;
        initialized_ = false;
        return WC0_FAULT_KIND == kWc0TerminateFailure
            ? Steinberg::kResultFalse
            : Steinberg::kResultOk;
    }

    Steinberg::tresult PLUGIN_API getControllerClassId(
        Steinberg::TUID class_id) override {
        if (WC0_FAULT_KIND == kWc0ControllerIdFailure)
            return Steinberg::kResultFalse;
        std::memcpy(class_id,
                    WC0_FAULT_KIND == kWc0ControllerIdMismatch
                        ? kWrongController
                        : kController,
                    sizeof(Steinberg::TUID));
        return Steinberg::kResultTrue;
    }

    Steinberg::tresult PLUGIN_API setIoMode(Steinberg::Vst::IoMode) override {
        forbidden_method_tripwire();
        return Steinberg::kNotImplemented;
    }

    Steinberg::int32 PLUGIN_API getBusCount(
        Steinberg::Vst::MediaType, Steinberg::Vst::BusDirection) override {
        forbidden_method_tripwire();
        return 0;
    }

    Steinberg::tresult PLUGIN_API getBusInfo(
        Steinberg::Vst::MediaType, Steinberg::Vst::BusDirection,
        Steinberg::int32, Steinberg::Vst::BusInfo&) override {
        forbidden_method_tripwire();
        return Steinberg::kNotImplemented;
    }

    Steinberg::tresult PLUGIN_API getRoutingInfo(
        Steinberg::Vst::RoutingInfo&, Steinberg::Vst::RoutingInfo&) override {
        forbidden_method_tripwire();
        return Steinberg::kNotImplemented;
    }

    Steinberg::tresult PLUGIN_API activateBus(
        Steinberg::Vst::MediaType, Steinberg::Vst::BusDirection,
        Steinberg::int32, Steinberg::TBool) override {
        forbidden_method_tripwire();
        return Steinberg::kNotImplemented;
    }

    Steinberg::tresult PLUGIN_API setActive(Steinberg::TBool) override {
        forbidden_method_tripwire();
        return Steinberg::kNotImplemented;
    }

    Steinberg::tresult PLUGIN_API setState(Steinberg::IBStream*) override {
        forbidden_method_tripwire();
        return Steinberg::kNotImplemented;
    }

    Steinberg::tresult PLUGIN_API getState(Steinberg::IBStream*) override {
        forbidden_method_tripwire();
        return Steinberg::kNotImplemented;
    }

private:
    ~FaultComponent() = default;

    std::atomic<long> references_;
    Steinberg::FUnknown* context_{nullptr};
    bool initialized_{false};
};
#endif

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

    Steinberg::tresult PLUGIN_API createInstance(Steinberg::FIDString cid,
                                                  Steinberg::FIDString iid,
                                                  void** object) override {
        if (object == nullptr) return Steinberg::kInvalidArgument;
        *object = nullptr;
#ifdef WC0_FAULT_KIND
        if (WC0_FAULT_KIND == kWc0CreateFailureNull)
                return Steinberg::kResultFalse;
        if (WC0_FAULT_KIND == kWc0CreateSuccessNull)
                return Steinberg::kResultOk;
        if (!Steinberg::FUnknownPrivate::iidEqual(cid, kProcessor) ||
            !Steinberg::FUnknownPrivate::iidEqual(
                iid, INLINE_UID_OF(Steinberg::Vst::IComponent)))
            return Steinberg::kResultFalse;
        *object = static_cast<Steinberg::Vst::IComponent*>(new FaultComponent());
        return WC0_FAULT_KIND == kWc0CreateFailureNonnull
            ? Steinberg::kResultFalse
            : Steinberg::kResultOk;
#else
        if (WF0_FAULT_KIND == kTripwire) {
            HANDLE marker = CreateFileW(L"C:\\wf0\\session\\create-instance-tripwire.marker",
                                        GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                                        FILE_ATTRIBUTE_NORMAL, nullptr);
            if (marker != INVALID_HANDLE_VALUE) CloseHandle(marker);
        }
        return Steinberg::kResultFalse;
#endif
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
