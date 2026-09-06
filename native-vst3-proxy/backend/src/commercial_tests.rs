//! Actual version-5 socket/mapping and state handlers, with a deterministic peer.
use super::*;
use std::os::unix::fs::FileExt;
use std::thread;
fn opaque(value: f64) -> Vec<u8> {
    let mut b = vec![0; 16];
    b[..4].copy_from_slice(&3u32.to_le_bytes());
    b[8..12].copy_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&[0xde, 0xad, 0xff]);
    b.extend_from_slice(&0x200100u32.to_le_bytes());
    b.extend_from_slice(&value.to_le_bytes());
    b
}
#[test]
fn real_protocol_carries_offsets_ids_and_accepts_vendor_reserialization() {
    let path = std::env::temp_dir().join(format!(
        "ap8-{}.audio",
        u128::from_le_bytes(mapping::random().unwrap())
    ));
    let mapping = Mapping::new(&path).unwrap();
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let socket = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    let events = vec![
        events::Event {
            offset: 7,
            kind: events::NOTE_ON,
            id: 79,
            channel: 3,
            pitch: 67,
            value: 0.7,
            ..Default::default()
        },
        events::Event {
            offset: 23,
            kind: events::PARAMETER,
            id: 0x200100,
            value: 0.25,
            ..Default::default()
        },
        events::Event {
            offset: 93,
            kind: events::NOTE_OFF,
            id: 79,
            channel: 3,
            pitch: 67,
            value: 0.1,
            ..Default::default()
        },
    ];
    let expected = events.clone();
    let remote = thread::spawn(move || {
        let f = receive_version(&mut peer, 5, 5).unwrap();
        assert_eq!(f.kind, PROCESS);
        assert_eq!(get(&f.payload[32..40]), 1);
        assert_eq!(get(&f.payload[40..48]), 0);
        assert_eq!(events::decode(&f.payload[48..], 128).unwrap(), expected);
        for ch in 0..2 {
            let samples: Vec<_> = (0..128)
                .flat_map(|i| (i as f32 / 128.).to_le_bytes())
                .collect();
            file.write_all_at(&samples, (OUTPUT + ch * STRIDE + 4) as u64)
                .unwrap();
        }
        let payload = [
            128u32.to_le_bytes().as_slice(),
            (OUTPUT as u32).to_le_bytes().as_slice(),
            0u64.to_le_bytes().as_slice(),
            &f.payload[32..48],
        ]
        .concat();
        send_version(
            &mut peer,
            &Frame {
                kind: DONE,
                session: f.session,
                sequence: f.sequence,
                payload,
            },
            5,
            5,
        )
        .unwrap();
        let f = receive_version(&mut peer, 5, 5).unwrap();
        assert_eq!(f.kind, 18);
        assert_eq!(f.payload, opaque(0.25));
        // Vendor normalized/re-serialized its internal opaque bytes while retaining controls.
        let mut payload = opaque(0.25);
        payload[16] = 0xef;
        send_version(
            &mut peer,
            &Frame {
                kind: 19,
                session: f.session,
                sequence: f.sequence,
                payload,
            },
            5,
            5,
        )
        .unwrap();
        let f = receive_version(&mut peer, 5, 5).unwrap();
        assert_eq!(f.kind, 16);
        send_version(
            &mut peer,
            &Frame {
                kind: 17,
                session: f.session,
                sequence: f.sequence,
                payload: opaque(f64::NAN),
            },
            5,
            5,
        )
        .unwrap();
    });
    let mut session = Session {
        mapping: Some(mapping),
        socket,
        state: ClientState {
            session: [8; 16],
            next: 1,
            slot: Slot::Writable,
        },
        phase: 11,
        max: 256,
        minor: 5,
        epoch: 1,
        position: 0,
        witness: None,
        trace: Default::default(),
        owner: None,
        identity: Some(state::Identity {
            class: [8; 16],
            module: [9; 32],
        }),
    };
    let (out, flags) = session
        .process_positioned(128, f64::NAN, 0, [&[0.; 128], &[0.; 128]], (1, 0), &events)
        .unwrap();
    assert_eq!(flags, 0);
    assert_eq!(f32::from_bits(out[0][94]), 93.0 / 128.0);
    assert_eq!(session.position, 128);
    session.phase = 13;
    let bytes = session.component_state(Some(&opaque(0.25))).unwrap();
    assert_eq!(bytes[16], 0xef);
    assert_eq!(session.state.next, 3);
    assert!(session.component_state(None).is_err());
    assert_eq!(session.phase, ERROR);
    remote.join().unwrap();
    drop(session);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn opaque_bound_state_refuses_bad_extent_ids_and_values() {
    for mut b in [opaque(0.25), opaque(0.25), opaque(0.25)] {
        b.push(0);
        assert!(state::commercial_payload(&b).is_err());
    }
    assert!(state::commercial_payload(&opaque(f64::INFINITY)).is_err());
    let mut b = opaque(0.25);
    b[8..12].copy_from_slice(&2u32.to_le_bytes());
    let point = b[19..].to_vec();
    b.extend_from_slice(&point);
    assert!(state::commercial_payload(&b).is_err());
}
