use ap1_native_client::mapping::{barrier, random};
use ap1_native_client::*;
use std::{
    io::{self, Write},
    path::PathBuf,
};
fn record(text: String) -> io::Result<()> {
    let mut out = io::stdout().lock();
    out.write_all(text.as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()
}
fn words(a: &[[u32; CAP + 2]; 2]) -> String {
    format!("[{:?},{:?}]", a[0], a[1])
}
fn run(stage: &mut &'static str) -> io::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    need(
        args.len() == 5 && args[1] == "--session-dir" && args[3] == "--session",
        "arguments",
    )?;
    let path = PathBuf::from(&args[2]);
    let id = &args[4];
    need(
        id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()),
        "session syntax",
    )?;
    let session: [u8; 16] =
        std::array::from_fn(|i| u8::from_str_radix(&id[2 * i..2 * i + 2], 16).unwrap());
    *stage = "mapping_setup";
    let endpoint = ap1_native_client::endpoint::Prepared::create(&path, session)?;
    *stage = "hello";
    let (mut mapping, mut socket) = endpoint.accept(1)?;
    *stage = "ready";
    let ready = receive(&mut socket, 15)?;
    need(
        ready.kind == READY
            && ready.session == session
            && ready.sequence == 0
            && ready.payload.is_empty(),
        "Ready differs",
    )?;
    // Chosen only after the real endpoint is activated/ready. Never sent to Windows.
    let seed = u64::from_le_bytes(random::<8>()?);
    record(format!("{{\"event\":\"ap1_client_ready\",\"seed\":{seed},\"mapping_count\":1,\"connection_count\":1,\"mapping_witness\":true,\"seed_chosen_after_ready\":true}}"))?;
    let mut state = ClientState {
        session,
        next: 1,
        slot: Slot::Writable,
    };
    let mut count = 0;
    for b in 0..LENGTHS.len() {
        *stage = "process";
        need(state.slot == Slot::Writable, "slot not writable")?;
        let frames = LENGTHS[b];
        let input = [recipe(seed, b, 0), recipe(seed, b, 1)];
        let mut poison = [POISON; CAP + 2];
        poison[0] = GUARD;
        poison[CAP + 1] = GUARD;
        for ch in 0..2 {
            mapping.write_plane(INPUT, ch, &input[ch])?;
            mapping.write_plane(OUTPUT, ch, &poison)?;
        }
        barrier();
        let request = state.process(frames, GAINS[b], SILENCE[b])?;
        // Failure latches ownership before any read/reuse. There is no resend path.
        let exchange = send(&mut socket, &request, 5)
            .and_then(|_| receive(&mut socket, 5))
            .and_then(|f| state.done(&f));
        let output_silence = match exchange {
            Ok(flags) => flags,
            Err(error) => {
                state.failed();
                return Err(error);
            }
        };
        barrier();
        let returned_input = [mapping.plane(INPUT, 0)?, mapping.plane(INPUT, 1)?];
        let output = [mapping.plane(OUTPUT, 0)?, mapping.plane(OUTPUT, 1)?];
        let comparison = compare(&input, &output, frames, GAINS[b], output_silence);
        let error = comparison
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or("null".into());
        count += frames * 2;
        record(format!("{{\"event\":\"ap1_client_block\",\"sequence\":{},\"frames\":{frames},\"gain\":{},\"silent\":{},\"input_silence_flags\":{},\"output_silence_flags\":{output_silence},\"input_bits\":{},\"output_bits\":{},\"maximum_absolute_error\":{error}}}",request.sequence,GAINS[b],SILENCE[b]==3,SILENCE[b],words(&returned_input),words(&output)))?;
        // Retain actual mapped words before either validation can fail.
        need(returned_input == input, "input/guard modified")?;
        comparison?;
    }
    *stage = "close";
    send(&mut socket, &state.close()?, 5)?;
    let closed = receive(&mut socket, 10)?;
    state.closed(&closed)?;
    drop(socket);
    mapping.close()?;
    *stage = "report";
    record(format!("{{\"event\":\"ap1_client_closed\",\"blocks\":10,\"samples_compared\":{count},\"maximum_absolute_error\":0.0,\"mapping_unmapped\":true,\"closed_received\":true,\"replays\":0}}"))?;
    Ok(())
}
fn main() {
    let mut stage = "setup";
    if let Err(error) = run(&mut stage) {
        let text = format!(
            "{{\"event\":\"ap1_client_error\",\"stage\":{stage:?},\"detail\":{:?}}}",
            error.to_string()
        );
        if record(text).is_err() {
            eprintln!("AP1 diagnostic persistence failed; earlier checkpoints retained");
        }
        std::process::exit(1);
    }
}
