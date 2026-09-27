/* Deterministic, private Pi comparison input. Build with:
 * cc -O2 -std=c11 tools/pi/jack_note_fixture.c -o jack_note_fixture $(pkg-config --cflags --libs jack)
 */
#define _POSIX_C_SOURCE 200809L
#include <jack/jack.h>
#include <jack/midiport.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

static jack_port_t *output;
static _Atomic uint64_t cursor;
static uint64_t period;
static _Atomic uint64_t start_frame;
static _Atomic uint64_t first_note_frame;
static _Atomic uint64_t note_ons;
static _Atomic uint64_t note_offs;
static _Atomic uint64_t write_failures;

static void emit(void *buffer, uint64_t begin, uint64_t end,
                 uint64_t frame, unsigned char status, unsigned char note) {
    if (frame < begin || frame >= end) return;
    const unsigned char bytes[3] = {status, note, status == 0x90 ? 100 : 0};
    if (jack_midi_event_write(buffer, (jack_nframes_t)(frame - begin), bytes, 3) != 0) {
        atomic_fetch_add_explicit(&write_failures, 1, memory_order_relaxed);
        return;
    }
    if (status == 0x90) {
        uint64_t unset = 0;
        atomic_compare_exchange_strong_explicit(&first_note_frame, &unset, frame,
                                                memory_order_relaxed, memory_order_relaxed);
        atomic_fetch_add_explicit(&note_ons, 1, memory_order_relaxed);
    } else {
        atomic_fetch_add_explicit(&note_offs, 1, memory_order_relaxed);
    }
}

static int process(jack_nframes_t frames, void *unused) {
    (void)unused;
    void *buffer = jack_port_get_buffer(output, frames);
    jack_midi_clear_buffer(buffer);
    const uint64_t begin = atomic_fetch_add_explicit(&cursor, frames, memory_order_relaxed);
    const uint64_t end = begin + frames;
    const uint64_t start = atomic_load_explicit(&start_frame, memory_order_acquire);
    if (!start || end <= start) return 0;
    uint64_t cycle = begin > start ? (begin - start) / period : 0;
    for (unsigned i = 0; i != 2; ++i, ++cycle) {
        const uint64_t base = start + cycle * period;
        if (base >= end) break;
        emit(buffer, begin, end, base, 0x90, 60);
        emit(buffer, begin, end, base + period / 8, 0x80, 60);
        emit(buffer, begin, end, base + period / 2, 0x90, 64);
        emit(buffer, begin, end, base + period * 5 / 8, 0x80, 64);
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: jack_note_fixture target-midi-port duration-seconds\n");
        return 64;
    }
    char *end = NULL;
    const unsigned long seconds = strtoul(argv[2], &end, 10);
    if (!end || *end || seconds < 3 || seconds > 240) return 64;
    jack_status_t status = 0;
    jack_client_t *client = jack_client_open("PiAS1Fixture", JackNoStartServer | JackUseExactName,
                                            &status);
    if (!client) return 1;
    output = jack_port_register(client, "out", JACK_DEFAULT_MIDI_TYPE, JackPortIsOutput, 0);
    const jack_nframes_t rate = jack_get_sample_rate(client);
    period = (uint64_t)rate * 2;
    if (!output || rate != 48000 || jack_set_process_callback(client, process, NULL) != 0 ||
        jack_activate(client) != 0 ||
        jack_connect(client, jack_port_name(output), argv[1]) != 0) {
        jack_client_close(client);
        return 2;
    }
    atomic_store_explicit(&start_frame,
                          atomic_load_explicit(&cursor, memory_order_acquire) + rate,
                          memory_order_release);
    struct timespec delay = {(time_t)seconds, 0};
    while (nanosleep(&delay, &delay) != 0) {}
    jack_deactivate(client);
    jack_client_close(client);
    printf("fixture=two-note-48k-v1 duration_s=%lu first_note_frame=%llu note_ons=%llu note_offs=%llu write_failures=%llu\n",
           seconds,
           (unsigned long long)atomic_load(&first_note_frame),
           (unsigned long long)atomic_load(&note_ons),
           (unsigned long long)atomic_load(&note_offs),
           (unsigned long long)atomic_load(&write_failures));
    return 0;
}
