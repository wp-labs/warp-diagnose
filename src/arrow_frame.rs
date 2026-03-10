use std::fs;
use std::path::Path;

use anyhow::{Context, bail};
use arrow::record_batch::RecordBatch;

pub struct ArrowFrame {
    pub tag: String,
    pub batch: RecordBatch,
}

pub fn read_arrow_frames(path: &Path) -> anyhow::Result<Vec<ArrowFrame>> {
    let bytes = fs::read(path).with_context(|| format!("read arrow file '{}'", path.display()))?;
    let mut frames = Vec::new();
    let mut offset = 0usize;

    while offset < bytes.len() {
        if offset + 4 > bytes.len() {
            bail!(
                "truncated arrow frame header at offset {} in '{}'",
                offset,
                path.display()
            );
        }

        let frame_len =
            u32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("frame header")) as usize;
        offset += 4;

        if offset + frame_len > bytes.len() {
            bail!(
                "truncated arrow frame payload at offset {} in '{}'",
                offset,
                path.display()
            );
        }

        let frame = wp_arrow::ipc::decode_ipc(&bytes[offset..offset + frame_len])
            .map_err(|err| anyhow::anyhow!("decode arrow frame failed: {err}"))?;
        frames.push(ArrowFrame {
            tag: frame.tag,
            batch: frame.batch,
        });
        offset += frame_len;
    }

    Ok(frames)
}
