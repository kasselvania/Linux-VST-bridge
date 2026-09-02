#include "factory_census.h"

#include <algorithm>
#include <array>
#include <cstring>
#include <iomanip>
#include <set>
#include <sstream>
#include <stdexcept>
#include <type_traits>

namespace linux_vst_bridge::wf0 {
namespace {

std::string u32_hex(std::uint32_t value) {
    std::ostringstream out;
    out << std::hex << std::setfill('0') << std::setw(8) << value;
    return out.str();
}

std::string bytes_hex(const void* data, std::size_t size) {
    static constexpr char hex[] = "0123456789abcdef";
    const auto* bytes = static_cast<const unsigned char*>(data);
    std::string result;
    result.reserve(size * 2);
    for (std::size_t index = 0; index < size; ++index) {
        result.push_back(hex[bytes[index] >> 4]);
        result.push_back(hex[bytes[index] & 15]);
    }
    return result;
}

std::string tuid_hex(const Steinberg::TUID id) {
    std::string result = bytes_hex(id, 16);
    std::transform(result.begin(), result.end(), result.begin(),
                   [](unsigned char value) { return static_cast<char>(std::toupper(value)); });
    return result;
}

template <typename Unit, std::size_t Size>
std::string terminated_hex(const Unit (&value)[Size]) {
    std::size_t used = 0;
    while (used < Size && value[used] != 0) ++used;
    if (used == Size) throw std::runtime_error("unterminated SDK field");
    return bytes_hex(value, used * sizeof(Unit));
}

bool interface_result_valid(Steinberg::tresult result, const void* pointer) {
    return (result == Steinberg::kResultOk && pointer != nullptr) ||
           (result == Steinberg::kNoInterface && pointer == nullptr);
}

std::string call_result(Steinberg::tresult value) {
    return ",\"result_u32_hex\":\"" +
           u32_hex(static_cast<std::uint32_t>(value)) + "\"";
}

void copy_basic(const Steinberg::PClassInfo& info, ClassRecord& out) {
    out.raw_tuid_hex = tuid_hex(info.cid);
    out.cardinality = info.cardinality;
    out.category_hex = terminated_hex(info.category);
    out.name_hex = terminated_hex(info.name);
    out.extended = false;
}

void copy_info2(const Steinberg::PClassInfo2& info, ClassRecord& out) {
    out.raw_tuid_hex = tuid_hex(info.cid);
    out.cardinality = info.cardinality;
    out.category_hex = terminated_hex(info.category);
    out.name_hex = terminated_hex(info.name);
    out.extended = true;
    out.class_flags = info.classFlags;
    out.subcategories_hex = terminated_hex(info.subCategories);
    out.vendor_hex = terminated_hex(info.vendor);
    out.version_hex = terminated_hex(info.version);
    out.sdk_version_hex = terminated_hex(info.sdkVersion);
}

void copy_infow(const Steinberg::PClassInfoW& info, ClassRecord& out) {
    out.raw_tuid_hex = tuid_hex(info.cid);
    out.cardinality = info.cardinality;
    out.category_hex = terminated_hex(info.category);
    out.name_hex = terminated_hex(info.name);
    out.extended = true;
    out.class_flags = info.classFlags;
    out.subcategories_hex = terminated_hex(info.subCategories);
    out.vendor_hex = terminated_hex(info.vendor);
    out.version_hex = terminated_hex(info.version);
    out.sdk_version_hex = terminated_hex(info.sdkVersion);
}

} // namespace

FactoryCensusResult enumerate_factory(Steinberg::IPluginFactory* factory,
                                      EventWriter& events, int max_classes) {
    FactoryCensusResult result;
    result.base_acquired = factory != nullptr;
    if (factory == nullptr) {
        result.exit_code = 73;
        return result;
    }

    try {
        Steinberg::PFactoryInfo factory_info{};
        const auto info_attempt = events.call_started("get_factory_info", "IPluginFactory");
        const Steinberg::tresult info_result = factory->getFactoryInfo(&factory_info);
        events.call_completed(info_attempt, "get_factory_info", "IPluginFactory", "tresult",
                              call_result(info_result));
        if (info_result != Steinberg::kResultOk) {
            result.exit_code = 74;
            return result;
        }
        result.census.factory_vendor_hex = terminated_hex(factory_info.vendor);
        result.census.factory_url_hex = terminated_hex(factory_info.url);
        result.census.factory_email_hex = terminated_hex(factory_info.email);
        result.census.factory_flags = factory_info.flags;
        events.lifecycle("factory_info_obtained");

        void* raw2 = nullptr;
        const auto query2_attempt = events.call_started("query_factory_2", "IPluginFactory");
        const Steinberg::tresult query2 =
            factory->queryInterface(INLINE_UID_OF(Steinberg::IPluginFactory2), &raw2);
        events.call_completed(query2_attempt, "query_factory_2", "IPluginFactory", "tresult",
                              call_result(query2));
        result.census.factory2_query_result = static_cast<std::uint32_t>(query2);
        if (!interface_result_valid(query2, raw2)) {
            result.exit_code = 74;
            return result;
        }
        result.factory2 = static_cast<Steinberg::IPluginFactory2*>(raw2);
        result.census.factory2_supported = result.factory2 != nullptr;

        void* raw3 = nullptr;
        const auto query3_attempt = events.call_started("query_factory_3", "IPluginFactory");
        const Steinberg::tresult query3 =
            factory->queryInterface(INLINE_UID_OF(Steinberg::IPluginFactory3), &raw3);
        events.call_completed(query3_attempt, "query_factory_3", "IPluginFactory", "tresult",
                              call_result(query3));
        result.census.factory3_query_result = static_cast<std::uint32_t>(query3);
        if (!interface_result_valid(query3, raw3)) {
            result.exit_code = 74;
            return result;
        }
        result.factory3 = static_cast<Steinberg::IPluginFactory3*>(raw3);
        result.census.factory3_supported = result.factory3 != nullptr;
        events.lifecycle("factory_interface_versions_recorded");

        const auto count_attempt = events.call_started("count_classes", "IPluginFactory");
        const Steinberg::int32 count = factory->countClasses();
        events.call_completed(count_attempt, "count_classes", "IPluginFactory", "i32",
                              ",\"i32_result\":" + std::to_string(count));
        if (count < 0 || count > max_classes || count > 256) {
            result.exit_code = 75;
            return result;
        }
        events.lifecycle("class_count_obtained", ",\"class_count\":" + std::to_string(count));
        events.lifecycle("class_enumeration_in_progress");

        std::set<std::string> seen;
        result.census.classes.reserve(static_cast<std::size_t>(count));
        for (Steinberg::int32 ordinal = 0; ordinal < count; ++ordinal) {
            ClassRecord record;
            record.ordinal = ordinal;
            bool obtained = false;

            if (result.factory3 != nullptr) {
                Steinberg::PClassInfoW info{};
                const auto attempt = events.call_started("get_class_info_unicode", "IPluginFactory3",
                                                         ordinal, "factory_3_unicode");
                const Steinberg::tresult call =
                    result.factory3->getClassInfoUnicode(ordinal, &info);
                events.call_completed(attempt, "get_class_info_unicode", "IPluginFactory3",
                                      "tresult", call_result(call), ordinal,
                                      "factory_3_unicode");
                record.tier_attempts.emplace_back("IPluginFactory3.PClassInfoW",
                                                  static_cast<std::uint32_t>(call));
                if (call == Steinberg::kResultOk) {
                    copy_infow(info, record);
                    record.tier = "IPluginFactory3.PClassInfoW";
                    obtained = true;
                }
            }
            if (!obtained && result.factory2 != nullptr) {
                Steinberg::PClassInfo2 info{};
                const auto attempt = events.call_started("get_class_info_2", "IPluginFactory2",
                                                         ordinal, "factory_2");
                const Steinberg::tresult call = result.factory2->getClassInfo2(ordinal, &info);
                events.call_completed(attempt, "get_class_info_2", "IPluginFactory2", "tresult",
                                      call_result(call), ordinal, "factory_2");
                record.tier_attempts.emplace_back("IPluginFactory2.PClassInfo2",
                                                  static_cast<std::uint32_t>(call));
                if (call == Steinberg::kResultOk) {
                    copy_info2(info, record);
                    record.tier = "IPluginFactory2.PClassInfo2";
                    obtained = true;
                }
            }
            if (!obtained) {
                Steinberg::PClassInfo info{};
                const auto attempt = events.call_started("get_class_info_1", "IPluginFactory",
                                                         ordinal, "factory_1");
                const Steinberg::tresult call = factory->getClassInfo(ordinal, &info);
                events.call_completed(attempt, "get_class_info_1", "IPluginFactory", "tresult",
                                      call_result(call), ordinal, "factory_1");
                record.tier_attempts.emplace_back("IPluginFactory.PClassInfo",
                                                  static_cast<std::uint32_t>(call));
                if (call == Steinberg::kResultOk) {
                    copy_basic(info, record);
                    record.tier = "IPluginFactory.PClassInfo";
                    obtained = true;
                }
            }
            if (!obtained) {
                result.exit_code = 76;
                return result;
            }
            if (!seen.insert(record.raw_tuid_hex).second) {
                result.exit_code = 77;
                return result;
            }
            result.census.classes.emplace_back(std::move(record));
        }
        events.lifecycle("class_enumeration_complete",
                         ",\"class_count\":" + std::to_string(count));
    } catch (const std::exception&) {
        result.exit_code = 78;
    }
    return result;
}

int release_factory_interfaces(Steinberg::IPluginFactory* factory,
                               FactoryCensusResult& result, EventWriter& events,
                               int primary_exit) {
    if (result.factory3 != nullptr) {
        const auto attempt = events.call_started("release_factory_3", "IPluginFactory3");
        const Steinberg::uint32 count = result.factory3->release();
        events.call_completed(attempt, "release_factory_3", "IPluginFactory3", "u32",
                              ",\"u32_result\":" + std::to_string(count));
        result.factory3 = nullptr;
    }
    if (result.factory2 != nullptr) {
        const auto attempt = events.call_started("release_factory_2", "IPluginFactory2");
        const Steinberg::uint32 count = result.factory2->release();
        events.call_completed(attempt, "release_factory_2", "IPluginFactory2", "u32",
                              ",\"u32_result\":" + std::to_string(count));
        result.factory2 = nullptr;
    }
    if (result.base_acquired && factory != nullptr) {
        const auto attempt = events.call_started("release_factory_base", "IPluginFactory");
        const Steinberg::uint32 count = factory->release();
        events.call_completed(attempt, "release_factory_base", "IPluginFactory", "u32",
                              ",\"u32_result\":" + std::to_string(count));
        result.base_acquired = false;
    }
    return primary_exit;
}

std::string census_json_fields(const CensusRecord& census) {
    std::ostringstream out;
    out << ",\"factory\":{\"vendor_hex\":\"" << census.factory_vendor_hex
        << "\",\"url_hex\":\"" << census.factory_url_hex
        << "\",\"email_hex\":\"" << census.factory_email_hex
        << "\",\"flags_i32\":" << census.factory_flags << "}"
        << ",\"factory2_supported\":" << (census.factory2_supported ? "true" : "false")
        << ",\"factory3_supported\":" << (census.factory3_supported ? "true" : "false")
        << ",\"factory2_query_u32_hex\":\"" << u32_hex(census.factory2_query_result) << "\""
        << ",\"factory3_query_u32_hex\":\"" << u32_hex(census.factory3_query_result) << "\""
        << ",\"class_count\":" << census.classes.size() << ",\"classes\":[";
    for (std::size_t index = 0; index < census.classes.size(); ++index) {
        const auto& item = census.classes[index];
        if (index != 0) out << ',';
        out << "{\"ordinal\":" << item.ordinal << ",\"raw_tuid_hex\":\""
            << item.raw_tuid_hex << "\",\"cardinality\":" << item.cardinality
            << ",\"category_hex\":\"" << item.category_hex << "\",\"name_hex\":\""
            << item.name_hex << "\",\"tier\":\"" << item.tier << "\",\"tier_attempts\":[";
        for (std::size_t attempt = 0; attempt < item.tier_attempts.size(); ++attempt) {
            if (attempt != 0) out << ',';
            out << "{\"tier\":\"" << item.tier_attempts[attempt].first
                << "\",\"result_u32_hex\":\"" << u32_hex(item.tier_attempts[attempt].second)
                << "\"}";
        }
        out << "]";
        if (item.extended) {
            out << ",\"class_flags_u32\":" << item.class_flags
                << ",\"subcategories_hex\":\"" << item.subcategories_hex
                << "\",\"vendor_hex\":\"" << item.vendor_hex
                << "\",\"version_hex\":\"" << item.version_hex
                << "\",\"sdk_version_hex\":\"" << item.sdk_version_hex << "\"";
        } else {
            out << ",\"class_flags_u32\":null,\"subcategories_hex\":null,"
                   "\"vendor_hex\":null,\"version_hex\":null,\"sdk_version_hex\":null";
        }
        out << '}';
    }
    out << ']';
    return out.str();
}

} // namespace linux_vst_bridge::wf0
