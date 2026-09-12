//! Offline development report: existing UIO1 clock law, generated fixture only.
//! Reads private scalars; emits no HWND, process, path, session or free text.
use linux_vst_bridge::{ui_observation::Bracket, Result};
use serde_json::{json, Value};
fn n(v: &Value, key: &str) -> Result<u64> {
    v[key]
        .as_u64()
        .ok_or_else(|| format!("missing scalar: {key}").into())
}
fn rows<'a>(v: &'a Value, key: &str, bound: usize) -> Result<&'a [Value]> {
    let a = v[key].as_array().ok_or("missing bounded array")?;
    if a.len() > bound {
        return Err("array capacity".into());
    }
    Ok(a)
}
fn project(raw: &Value) -> Result<Value> {
    if raw["completed"] != true {
        return Err("incomplete generated differential".into());
    }
    let brackets: Vec<Bracket> = serde_json::from_value(raw["clock_brackets"].clone())?;
    if brackets.is_empty() || brackets.len() > 8 {
        return Err("clock bracket capacity".into());
    }
    let input = rows(raw, "actions", 16)?;
    let win = rows(raw, "win32", 16384)?;
    let x11 = rows(raw, "xrecord", 4096)?;
    let id = &raw["identity"];
    let freq = n(id, "frequency")?;
    if freq == 0 || brackets.iter().any(|b| b.frequency != freq) {
        return Err("clock frequency mismatch".into());
    }
    let child = n(id, "child")?;
    let root = n(id, "root")?;
    let origin = input.first().ok_or("no input")?["motion"]["interval_ns"][0]
        .as_u64()
        .ok_or("origin")?;
    let time = |q: u64| -> Result<[u64; 2]> {
        brackets
            .iter()
            .min_by_key(|b| q.abs_diff(b.windows_qpc))
            .ok_or("no bracket")?
            .project(q)
    };
    let mut actions = Vec::new();
    for action in 1..=3 {
        let mut events = Vec::new();
        for (name, core, wm) in [("down", 4, 0x201), ("up", 5, 0x202)] {
            let issued = input
                .iter()
                .find(|r| r["action"] == action && r.get(name).is_some())
                .ok_or("input pair absent")?;
            let before = n(&json!({"t": issued[name]["interval_ns"][0]}), "t")?;
            let xr: Vec<_> = x11
                .iter()
                .filter(|r| r["action"] == action && r["kind"] == core)
                .collect();
            if xr.len() != 1 {
                return Err("exact one X11 receipt required".into());
            }
            let observed = n(xr[0], "observed_ns")?;
            let wr: Vec<_> = win
                .iter()
                .filter(|r| r["action"] == action && r["source"] == 5 && r["message"] == wm)
                .collect();
            if wr.len() != 1 || n(wr[0], "hwnd")? != child || n(wr[0], "active")? != root {
                return Err("exact child hardware retrieval required".into());
            }
            let retrieved = time(n(wr[0], "qpc")?)?;
            if !win.iter().any(|r| {
                r["action"] == action && r["source"] == 6 && r["message"] == wm && r["result"] == 0
            }) {
                return Err("nonconsuming mouse-hook return absent".into());
            }
            events.push(json!({"event":name,"issued_ns":before.checked_sub(origin).ok_or("time order")?,
                "x11_observed_no_later_than_ns":observed.checked_sub(origin).ok_or("time order")?,
                "win32_retrieval_interval_ns":[retrieved[0].checked_sub(origin).ok_or("time order")?,retrieved[1].checked_sub(origin).ok_or("time order")?],
                "x11_to_retrieval_lower_bound_ns":retrieved[0].saturating_sub(observed),
                "issued_to_retrieval_upper_bound_ns":retrieved[1].checked_sub(before).ok_or("time order")?,
                "target":"generated_child","active":"generated_parent","client":[wr[0]["x"].as_i64().ok_or("client x")?,wr[0]["y"].as_i64().ok_or("client y")?],
                "focus_is_child":n(wr[0],"focus")? == child,"hook_consumed":false}));
        }
        let heartbeat = win
            .iter()
            .filter(|r| r["action"] == action && r["source"] == 4)
            .filter_map(|r| r["result"].as_u64())
            .max();
        let condition =
            ["idle", "bounded_posted_and_sent", "idle_after_load"][(action - 1) as usize];
        actions.push(json!({"id":action,"condition":condition,
            "events":events,"max_heartbeat_ns":heartbeat.map(|q| u128::from(q)*1_000_000_000/u128::from(freq))}));
    }
    let final_status = &raw["final_status"];
    if n(final_status, "error")? != 0
        || n(final_status, "finished")? != 1
        || n(final_status, "submitted")? != n(final_status, "posted")?
        || n(final_status, "send_failures")? != 0
        || n(final_status, "sent")? != 64
        || n(final_status, "down")? != 3
        || n(final_status, "up")? != 3
    {
        return Err("incomplete traffic/window cleanup".into());
    }
    let observer = &raw["observer_status_final"];
    for field in [
        "dropped",
        "heartbeat_errors",
        "scope_errors",
        "unhook_errors",
    ] {
        if n(observer, field)? != 0 {
            return Err("observer incomplete".into());
        }
    }
    if n(observer, "closed")? != 1
        || n(observer, "detached")? != 1
        || n(&raw["xrecord_status"], "dropped")? != 0
    {
        return Err("observer custody incomplete".into());
    }
    let mut progress = Vec::new();
    if raw.get("traffic_samples").is_some() {
        if n(raw, "traffic_sample_overflow")? != 0 {
            return Err("traffic observation overflow".into());
        }
        let samples = rows(raw, "traffic_samples", 1200)?;
        for (label, row) in [
            (
                "first_all_sends_complete",
                samples
                    .iter()
                    .find(|r| r["phase"].as_u64().unwrap_or(0) >= 2 && r["sent"] == 64),
            ),
            (
                "last_before_loaded_down",
                samples
                    .iter()
                    .rev()
                    .find(|r| r["phase"].as_u64().unwrap_or(0) >= 2 && r["down"] == 1),
            ),
            (
                "first_loaded_down",
                samples
                    .iter()
                    .find(|r| r["phase"].as_u64().unwrap_or(0) >= 2 && r["down"] == 2),
            ),
        ] {
            let r = row.ok_or("missing traffic transition sample")?;
            progress.push(json!({"observation":label,"linux_observed_ns":n(r,"observed_ns")?.checked_sub(origin).ok_or("traffic time order")?,
                "phase":n(r,"phase")?,"posted_submitted":n(r,"submitted")?,"posted_handled":n(r,"posted")?,"sent_handled":n(r,"sent")?,
                "down_handled":n(r,"down")?,"up_handled":n(r,"up")?}));
        }
    }
    Ok(
        json!({"schema":1,"fixture":"generated_parent_child_no_vendor","actions":actions,
        "clock_domains":["linux_monotonic_ns","windows_qpc","x11_server_ms"],
        "clock_law":"UIO1 Bracket::project: measured round trip, 100ppm drift, 10s maximum distance",
        "input_boundary":"exact X11 receipt to first observable Win32 hardware-mouse retrieval; no internal admission timestamp",
        "traffic":{"submitted":final_status["submitted"],"handled":final_status["posted"],"sent_handled":64,"send_failures":0,"maximum_posted":4096,"admission_limit_seconds":6,"posted_chains":4,"synthetic_handler_sleep_ms":1},
        "observer":{"records":n(observer,"committed")?,"dropped":0,"closed":1,"detached":1},
        "xrecord":{"records":x11.len(),"dropped":0,"unparsed":n(&raw["xrecord_status"],"unparsed")?},
        "exact_three_mouse_pairs":true,"windows_destroyed":true,
        "bounded_traffic_progress":progress,
        "nonclaims":["no vendor workload equivalence","no physical-compositor equivalence","no product repair","no audio or rendering result"]}),
    )
}
fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or("one private record required")?;
    use std::io::Read;
    let mut data = Vec::new();
    std::fs::File::open(path)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut data)?;
    if data.len() > 8 * 1024 * 1024 {
        return Err("private record bound".into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&project(&serde_json::from_slice(&data)?)?)?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Value {
        let mut input = Vec::new();
        let mut win = Vec::new();
        let mut xr = Vec::new();
        input
            .push(json!({"action":1,"motion":{"interval_ns":[1_000_000_000u64,1_000_000_000u64]}}));
        for action in 1..=3 {
            for (name, core, wm) in [("down", 4, 0x201), ("up", 5, 0x202)] {
                let t = 1_000_000_000u64 + action * 1_000_000;
                input.push(json!({"action":action,name:{"interval_ns":[t,t+1000]}}));
                xr.push(json!({"action":action,"kind":core,"observed_ns":t+2000}));
                for source in [5, 6] {
                    win.push(json!({"action":action,"source":source,"message":wm,"qpc":t+100_000_000,"hwnd":2,"active":1,"focus":2,"x":20,"y":30,"result":0}));
                }
            }
        }
        json!({"completed":true,"identity":{"frequency":1_000_000_000u64,"root":1,"child":2},
            "clock_brackets":[{"linux_before_ns":1_000_000_000u64,"linux_after_ns":1_000_010_000u64,"windows_qpc":1_000_000_000u64,"frequency":1_000_000_000u64}],
            "actions":input,"win32":win,"xrecord":xr,"xrecord_status":{"dropped":0,"unparsed":0},
            "final_status":{"error":0,"finished":1,"submitted":4000,"posted":4000,"send_failures":0,"sent":64,"down":3,"up":3},
            "observer_status_final":{"dropped":0,"heartbeat_errors":0,"scope_errors":0,"unhook_errors":0,"closed":1,"detached":1,"committed":12}})
    }
    #[test]
    fn conservative_retrieval_boundary_and_private_projection() {
        let raw = fixture();
        let report = project(&raw).unwrap();
        assert_eq!(
            report["actions"][0]["events"][0]["x11_to_retrieval_lower_bound_ns"],
            99_987_900u64
        );
        let text = report.to_string();
        for private in ["hwnd", "pid", "start_ticks", "/tmp/", "session"] {
            assert!(!text.contains(private));
        }
    }
    #[test]
    fn refuses_incomplete_or_mismatched_receipts() {
        for key in ["error", "send_failures"] {
            let mut r = fixture();
            r["final_status"][key] = json!(1);
            assert!(project(&r).is_err());
        }
        let mut r = fixture();
        r["win32"][0]["hwnd"] = json!(99);
        assert!(project(&r).is_err());
        let mut r = fixture();
        r["xrecord_status"]["dropped"] = json!(1);
        assert!(project(&r).is_err());
        let mut r = fixture();
        r["identity"]["frequency"] = json!(0);
        assert!(project(&r).is_err());
        let mut r = fixture();
        r["clock_brackets"][0]["windows_qpc"] = json!(99_000_000_000u64);
        assert!(project(&r).is_err());
    }
}
