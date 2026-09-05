#include "ap1_protocol.h"
#include <iomanip>
#include <iostream>
using namespace linux_vst_bridge::ap1;

int main(int argc, char** argv) {
    if (argc != 2) { return 2; }
    std::vector<uint8_t> bytes;
    const std::string text = argv[1];
    require(text.size() % 2 == 0, "hex vector length");
    for (size_t i = 0; i < text.size(); i += 2) {
        bytes.push_back(uint8_t(std::stoul(text.substr(i, 2), nullptr, 16)));
    }
    auto frame = decode(bytes);
    auto wire = encode(frame);
    require(wire == bytes, "vector round trip");
    auto req = request(frame);
    require(req.frames == 63 && req.gain == 0.75, "vector values");

    Sequence state;
    state.session = frame.session;
    state.next = frame.sequence;
    state.begin(frame);
    bool rejected = false;
    try { state.begin(frame); } catch (...) { rejected = true; }
    require(rejected && state.failed, "duplicate request");
    for (size_t offset : {size_t(0), size_t(4), size_t(6), size_t(10), size_t(12), size_t(32), size_t(48)}) {
        auto bad = bytes;
        bad[offset] = 255;
        bool reject = false;
        try { decode(bad); } catch (...) { reject = true; }
        require(reject, "bad header accepted");
    }
    for (size_t n = 0; n < bytes.size(); ++n) {
        bool reject = false;
        try { decode({bytes.begin(), bytes.begin() + n}); } catch (...) { reject = true; }
        require(reject, "truncation accepted");
    }
    for (int fault = 0; fault < 5; ++fault) {
        Sequence q;
        q.session = frame.session;
        q.next = frame.sequence;
        auto bad = frame;
        if (fault == 0) { bad.session[0] ^= 1; }
        if (fault == 1) { ++bad.sequence; }
        if (fault == 2) { put(bad.payload.data(), 257, 4); }
        if (fault == 3) { put(bad.payload.data() + 8, 1, 4); }
        if (fault == 4) { put(bad.payload.data() + 16, 0x7ff8000000000000ULL, 8); }
        bool reject = false;
        try { q.begin(bad); } catch (...) { reject = true; }
        require(reject && q.failed && !q.outstanding, "invalid process admitted");
    }
    Sequence q;
    q.session = frame.session;
    q.next = 64;
    auto last = frame;
    last.sequence = 64;
    q.begin(last);
    q.complete();
    q.close({Close, q.session, 65, {}});
    require(q.closed, "64th block close");
    // Exercise the production native result handler called by MappedSession::done.
    // These cases fail under the old output==input predicate.
    const float zero[2] = {0.f, -0.f};
    const float signal[2] = {0.25f, -0.5f};
    const auto check_result = [](Request r, const float* l, const float* rr, uint64_t flags) {
        auto p = processing_result(r, l, rr, flags);
        require(p.size()==16 && get(p.data(),4)==r.frames &&
                get(p.data()+4,4)==output_offset && get(p.data()+8,8)==flags,
                "actual result flags/extent not retained");
    };
    check_result({2,0.,0},zero,zero,3);       // Zero gain, non-silent input.
    check_result({2,.5,1},zero,signal,0);     // One silent input; output unflagged.
    check_result({2,.5,2},signal,zero,0);     // Symmetric channel.
    check_result({2,.5,0},signal,signal,0);   // Existing ordinary case.
    check_result({2,.5,3},zero,zero,3);       // Existing all-silent case.
    check_result({2,.5,1},zero,signal,1);     // Correct optional silence claim.
    check_result({2,0.,0},zero,zero,0);       // Zeros need not carry flags.
    for (uint64_t invalid_flags : {uint64_t(4), uint64_t(1)<<32, uint64_t(1)<<63}) {
        bool reject=false;
        try { processing_result({2,0.,0},zero,zero,invalid_flags); }
        catch (...) { reject=true; }
        require(reject,"invalid high output silence bits accepted");
    }
    for (uint64_t flags : {uint64_t(1),uint64_t(2),uint64_t(3)}) {
        bool reject=false;
        try { processing_result({2,.5,0},signal,signal,flags); }
        catch (...) { reject=true; }
        require(reject,"false silence claim accepted");
    }
    const float nonfinite[2]={std::nanf(""),0.f};
    bool reject=false;
    try { processing_result({2,0.,0},nonfinite,zero,3); }
    catch (...) { reject=true; }
    require(reject,"nonfinite silent output accepted");
    Frame activate{Activate,frame.session,1,{0,1,0,0}};
    auto successor=encode(activate,2);
    require(decode(successor,2).kind==Activate,"AP2 successor framing");
    bool incompatible=false;
    try{decode(successor);}catch(...){incompatible=true;}
    require(incompatible,"AP2 must not be admitted as AP1");
    for (auto value : wire) {
        std::cout << std::hex << std::setfill('0') << std::setw(2) << unsigned(value);
    }
    std::cout << '\n';
}
