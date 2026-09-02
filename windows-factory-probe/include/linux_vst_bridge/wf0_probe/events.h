#pragma once

#include <cstdio>
#include <stdexcept>
#include <string>

namespace linux_vst_bridge::wf0 {

class EventWriter {
public:
    explicit EventWriter(std::size_t cap) : cap_(cap) {}

    unsigned long long sequence() const noexcept { return sequence_; }

    void lifecycle(const char* state, const std::string& fields = {}) {
        write("{\"event\":\"lifecycle\",\"sequence\":" + next() +
              ",\"state\":\"" + state + "\"" + fields + "}");
    }

    void final_lifecycle(const char* state, const std::string& fields) {
        write("{\"event\":\"lifecycle\",\"sequence\":" + next() +
              ",\"state\":\"" + state + "\"" + fields + "}", 786432);
    }

    unsigned long long call_started(const char* operation, const char* interface_name,
                                    int ordinal = -1, const char* tier = nullptr,
                                    const std::string& fields = {}) {
        const auto attempt = sequence_ + 1;
        write("{\"event\":\"call_started\",\"sequence\":" + next() +
              ",\"operation\":\"" + operation + "\",\"interface\":" +
              quoted_or_null(interface_name) + ",\"ordinal\":" +
              (ordinal < 0 ? "null" : std::to_string(ordinal)) + ",\"tier\":" +
              quoted_or_null(tier) + fields + "}");
        return attempt;
    }

    void call_completed(unsigned long long attempt, const char* operation,
                        const char* interface_name, const char* return_kind,
                        const std::string& result_fields = {}, int ordinal = -1,
                        const char* tier = nullptr,
                        const std::string& fields = {}) {
        write("{\"event\":\"call_completed\",\"sequence\":" + next() +
              ",\"attempt_sequence\":" + std::to_string(attempt) +
              ",\"operation\":\"" + operation + "\",\"interface\":" +
              quoted_or_null(interface_name) + ",\"ordinal\":" +
              (ordinal < 0 ? "null" : std::to_string(ordinal)) + ",\"tier\":" +
              quoted_or_null(tier) + ",\"return_kind\":\"" + return_kind + "\"" +
              result_fields + fields + "}");
    }

    void host_callback(const char* operation, const std::string& fields) {
        write("{\"event\":\"host_callback\",\"sequence\":" + next() +
              ",\"operation\":\"" + operation + "\"" + fields + "}");
    }

private:
    std::string next() { return std::to_string(++sequence_); }

    static std::string quoted_or_null(const char* value) {
        return value == nullptr ? "null" : std::string("\"") + value + "\"";
    }

    void write(const std::string& line, std::size_t event_cap = 4096) {
        if (line.size() > event_cap || bytes_ + line.size() + 1 > cap_ || sequence_ > 2048) {
            throw std::runtime_error("WF0 output bound exceeded");
        }
        if (std::fwrite(line.data(), 1, line.size(), stdout) != line.size() ||
            std::fputc('\n', stdout) == EOF || std::fflush(stdout) != 0) {
            throw std::runtime_error("WF0 output write or flush failed");
        }
        bytes_ += line.size() + 1;
    }

    unsigned long long sequence_{0};
    std::size_t bytes_{0};
    std::size_t cap_;
};

} // namespace linux_vst_bridge::wf0
