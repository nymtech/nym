use std::fmt;
use std::fmt::Write;

pub fn format_debug_bytes(bytes: &[u8]) -> Result<String, fmt::Error> {
    let mut out = String::new();
    const LINE_LEN: usize = 16;
    for (i, chunk) in bytes.chunks(LINE_LEN).enumerate() {
        let line_prefix = format!("[{}:{}]", 1 + i * LINE_LEN, i * LINE_LEN + chunk.len());
        write!(out, "{line_prefix:12}")?;
        let mut line = String::new();
        for b in chunk {
            line.push_str(format!("{:02x} ", b).as_str());
        }
        write!(
            out,
            "{line:48} {}",
            chunk
                .iter()
                .map(|&b| b as char)
                .map(|c| if c.is_alphanumeric() { c } else { '.' })
                .collect::<String>()
        )?;

        writeln!(out)?;
    }

    Ok(out)
}
