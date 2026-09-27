# Manager readback measurement — first bounded reduction

The full operator snapshot parses and validates the closed installed profile
set for canonical product projection. It then needs the same profiles to select
recommended revisions for the product cards. This change passes the one
validated set through both steps. It adds no persistent cache, changes no
profile authority, and leaves fresh registry, software, capacity, token and
action validation in place.

## Source-owned measurement

`snapshot_readback_latency_sample` is an opt-in ignored manager-binary test.
It creates one managed installer/product fixture, takes one cold snapshot, then
25 warm full snapshots. The latter are sorted to print p50, p95 and maximum
elapsed microseconds. Run it with:

```sh
cargo test --manifest-path bridge-manager/Cargo.toml --locked --bin linux-vst-bridge snapshot_readback_latency_sample -- --ignored --nocapture
```

On the local macOS development host, with the same fixture and test profile,
five runs of the original two-profile-load source yielded warm p50 values of
25,871, 26,109, 25,939, 25,909 and 26,174 µs. Five runs after reusing the
profile set yielded 25,106, 25,099, 25,243, 25,672 and 24,686 µs.
The respective medians across those runs were 25,939 and 25,106 µs, an 833 µs
reduction in this synthetic fixture. p95 values varied; no strong tail-latency
claim follows. This measures a one-product full readback on one development
host, not launch time, page switching, a larger library, the Deck, Linux, or
audio impact.

The next performance pass should time first usable Home, warm navigation,
installer feedback, long-operation polling and idle resource use on an exact
installed generation. A larger synthetic library should test how history and
inventory projection scale. Action admission must continue to revalidate
current authority even if a future presentation cache is introduced.
