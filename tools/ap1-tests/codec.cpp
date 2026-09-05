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
    for (auto value : wire) {
        std::cout << std::hex << std::setfill('0') << std::setw(2) << unsigned(value);
    }
    std::cout << '\n';
}
