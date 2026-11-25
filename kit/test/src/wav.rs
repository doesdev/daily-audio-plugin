//! WAV files, 16 bit: enough to write a demo someone can listen to.

use std::io::Write;
use std::path::Path;

/// `channels` (each the same length) as a 16 bit WAV file's bytes.
pub fn encode(rate: u32, channels: &[Vec<f32>]) -> Vec<u8> {
    let count = channels.len().max(1) as u16;
    let frames = channels.first().map_or(0, Vec::len);
    let data = frames as u32 * u32::from(count) * 2;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(count) * 2).to_le_bytes());
    out.extend_from_slice(&(count * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for i in 0..frames {
        for ch in channels {
            let sample = (ch[i].clamp(-1.0, 1.0) * 32_767.0).round() as i16;
            out.extend_from_slice(&sample.to_le_bytes());
        }
    }
    out
}

pub fn write(path: &Path, rate: u32, channels: &[Vec<f32>]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::File::create(path)?.write_all(&encode(rate, channels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_header_a_player_reads_and_clips_what_is_too_loud() {
        let bytes = encode(48_000, &[vec![0.0, 1.0, -2.0], vec![0.5, 0.0, 0.0]]);

        assert_eq!(bytes.len(), 44 + 12);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 36 + 12);
        assert_eq!(
            u16::from_le_bytes(bytes[22..24].try_into().unwrap()),
            2,
            "stereo"
        );
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            48_000
        );
        let sample =
            |i: usize| i16::from_le_bytes(bytes[44 + i * 2..46 + i * 2].try_into().unwrap());
        assert_eq!(
            [sample(0), sample(1), sample(2), sample(4)],
            [0, 16_384, 32_767, -32_767]
        );
    }
}
