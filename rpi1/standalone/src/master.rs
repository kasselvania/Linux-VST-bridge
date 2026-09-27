//! Experiment-owned access to the exact Pigments census Master Volume parameter.
use ap2_backend::rpi0::Message;
use std::io;

pub const ID: u32 = 0;

pub fn normalized(text: &str) -> io::Result<f64> {
    let value: f64 = text.parse().map_err(|_| invalid())?;
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(invalid());
    }
    Ok(value)
}

pub fn readback(message: &Message) -> io::Result<f64> {
    let text =
        |s: &[u16]| String::from_utf16(&s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]);
    if message.kind != 110
        || message.id != ID
        || message.result != 0
        || text(&message.title).ok().as_deref() != Some("Master Volume")
        || text(&message.units).ok().as_deref() != Some("dB")
        || !message.value.is_finite()
        || !(0.0..=1.0).contains(&message.value)
    {
        return Err(invalid());
    }
    Ok(message.value)
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "Pigments Master Volume identity/value",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_control_refuses_nonfinite_and_out_of_range() {
        for text in ["NaN", "inf", "-0.1", "1.1", "0.5 extra", ""] {
            assert!(normalized(text).is_err());
        }
        for text in ["0", "0.5", "1"] {
            assert!(normalized(text).is_ok());
        }
    }

    #[test]
    fn readback_requires_exact_census_parameter_and_available_value() {
        let mut m = Message {
            kind: 110,
            id: ID,
            value: 0.5,
            ..Message::default()
        };
        for (slot, ch) in m.title.iter_mut().zip("Master Volume".encode_utf16()) {
            *slot = ch;
        }
        m.units[..2].copy_from_slice(&[b'd' as u16, b'B' as u16]);
        assert_eq!(readback(&m).unwrap(), 0.5);
        m.id = 1;
        assert!(readback(&m).is_err());
        m.id = ID;
        m.result = 1;
        assert!(readback(&m).is_err());
        m.result = 0;
        m.value = f64::NAN;
        assert!(readback(&m).is_err());
        m.value = 0.5;
        m.title[0] = b'X' as u16;
        assert!(readback(&m).is_err());
    }
}
