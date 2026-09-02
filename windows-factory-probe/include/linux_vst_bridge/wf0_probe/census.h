#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace linux_vst_bridge::wf0 {

struct ClassRecord {
    std::int32_t ordinal{};
    std::string raw_tuid_hex;
    std::int32_t cardinality{};
    std::string category_hex;
    std::string name_hex;
    std::string tier;
    std::vector<std::pair<std::string, std::uint32_t>> tier_attempts;
    bool extended{};
    std::uint32_t class_flags{};
    std::string subcategories_hex;
    std::string vendor_hex;
    std::string version_hex;
    std::string sdk_version_hex;
};

struct CensusRecord {
    std::string factory_vendor_hex;
    std::string factory_url_hex;
    std::string factory_email_hex;
    std::int32_t factory_flags{};
    bool factory2_supported{};
    bool factory3_supported{};
    std::uint32_t factory2_query_result{};
    std::uint32_t factory3_query_result{};
    std::vector<ClassRecord> classes;
};

std::string census_json_fields(const CensusRecord& census);

} // namespace linux_vst_bridge::wf0
