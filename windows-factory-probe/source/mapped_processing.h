#pragma once
#include "offline_processing.h"
#include "ap1_protocol.h"
#include <memory>
#include <string>
namespace linux_vst_bridge::wf0 {
class MappedSession final : public ExternalProcessing {
public:
    MappedSession(const std::wstring& directory, const std::string& session, EventWriter&, bool hosted=false, bool sustained=false);
    ~MappedSession();
    bool hosted() const override;
    bool sustained() const override;
    uint16_t next_transition() override;
    uint32_t lifecycle_request(uint16_t) override;
    void lifecycle_ack(uint16_t) override;
    void ready() override;
    bool next(ExternalBlock&,float*,float*) override;
    void done(const float*,const float*,uint64_t) override;
    void finish(bool success);
private:
    struct Impl;std::unique_ptr<Impl> impl_;
};
}
