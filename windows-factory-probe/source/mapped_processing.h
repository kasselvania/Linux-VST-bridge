#pragma once
#include "offline_processing.h"
#include "ap1_protocol.h"
#include <memory>
#include <string>
namespace linux_vst_bridge::wf0 {
class MappedSession final : public ExternalProcessing {
public:
    MappedSession(const std::wstring& directory, const std::string& session, EventWriter&);
    ~MappedSession();
    void ready() override;
    bool next(ExternalBlock&,float*,float*) override;
    void done(const float*,const float*,uint64_t) override;
    void finish(bool success);
private:
    struct Impl;std::unique_ptr<Impl> impl_;
};
}
