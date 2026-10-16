#pragma once
#include "ap11_gui.h"
#include <cstddef>
#include <cstdint>
#include <cstring>
// VST3 hosts deliver MIDI controllers (control change, channel aftertouch,
// pitch bend, program change) as parameter changes: they ask the controller's
// IMidiMapping which parameter a controller drives. The Windows host reads the
// vendor's complete table on its controller owner thread and publishes it on
// UI protocol 7 as one stream:
//   MidiMapBegin   count = assignments in the table, revision = stream serial
//   MidiMapEntries count = 1..64 records in this message, same revision
//   MidiMapEnd     count = assignments in the table, same revision
// Records are 8 bytes in the message's 512 title/units bytes:
//   bus u8, channel u8, controller u16 LE, parameter ID u32 LE.
// Unused record bytes are zero. A table without assignments (including a
// vendor without IMidiMapping) is Begin(0) + End(0).
namespace AP11 {
inline constexpr uint32_t MidiChannels = 16;
// 0-127 control change, 128 channel aftertouch, 129 pitch bend, 130 program
// change (Steinberg::Vst::ControllerNumbers up to kCtrlProgramChange).
inline constexpr uint32_t MidiControllers = 131;
inline constexpr uint32_t MidiBuses = 16; // event input buses read; more are ignored
inline constexpr uint32_t MidiAssignmentLimit = MidiBuses * MidiChannels * MidiControllers;
inline constexpr uint32_t MidiAssignmentsPerMessage = 64;
inline constexpr size_t MidiRecordBytes = 8;
struct MidiAssignment {
  uint32_t bus = 0, channel = 0, controller = 0, id = 0;
};
inline bool valid(const MidiAssignment &a) {
  return a.bus < MidiBuses && a.channel < MidiChannels && a.controller < MidiControllers;
}
// Dense index of one (bus, channel, controller) answer.
inline constexpr uint32_t midiSlot(uint32_t bus, uint32_t channel, uint32_t controller) {
  return (bus * MidiChannels + channel) * MidiControllers + controller;
}
static_assert(offsetof(ap11_gui_message_t, units) ==
                  offsetof(ap11_gui_message_t, title) + sizeof(ap11_gui_message_t::title) &&
              sizeof(ap11_gui_message_t::title) + sizeof(ap11_gui_message_t::units) ==
                  MidiAssignmentsPerMessage * MidiRecordBytes);
inline uint8_t *midiRecords(ap11_gui_message_t &m) { return reinterpret_cast<uint8_t *>(m.title); }
inline const uint8_t *midiRecords(const ap11_gui_message_t &m) {
  return reinterpret_cast<const uint8_t *>(m.title);
}
// Fills an Entries message with n (1..64) assignments; false leaves no claim.
inline bool packMidi(ap11_gui_message_t &m, const MidiAssignment *a, uint32_t n) {
  if (!a || !n || n > MidiAssignmentsPerMessage) return false;
  for (uint32_t i = 0; i < n; ++i)
    if (!valid(a[i])) return false;
  auto *p = midiRecords(m);
  std::memset(p, 0, MidiAssignmentsPerMessage * MidiRecordBytes);
  for (uint32_t i = 0; i < n; ++i, p += MidiRecordBytes) {
    p[0] = uint8_t(a[i].bus);
    p[1] = uint8_t(a[i].channel);
    p[2] = uint8_t(a[i].controller);
    p[3] = uint8_t(a[i].controller >> 8);
    for (unsigned b = 0; b < 4; ++b) p[4 + b] = uint8_t(a[i].id >> (8 * b));
  }
  m.kind = MidiMapEntries;
  m.count = n;
  return true;
}
// Reads an Entries message; out holds 64. Rejects out-of-range records and
// any bytes after the counted records.
inline bool unpackMidi(const ap11_gui_message_t &m, MidiAssignment *out) {
  if (!out || m.kind != MidiMapEntries || !m.count || m.count > MidiAssignmentsPerMessage)
    return false;
  const auto *p = midiRecords(m);
  for (uint32_t i = 0; i < MidiAssignmentsPerMessage; ++i, p += MidiRecordBytes) {
    if (i >= m.count) {
      for (size_t b = 0; b < MidiRecordBytes; ++b)
        if (p[b]) return false;
      continue;
    }
    MidiAssignment a;
    a.bus = p[0];
    a.channel = p[1];
    a.controller = uint32_t(p[2]) | uint32_t(p[3]) << 8;
    for (unsigned b = 0; b < 4; ++b) a.id |= uint32_t(p[4 + b]) << (8 * b);
    if (!valid(a)) return false;
    out[i] = a;
  }
  return true;
}
} // namespace AP11
