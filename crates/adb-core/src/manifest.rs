//! Bounded metadata-only Android binary XML reader. No allocation uses an unchecked pool count.
use std::collections::HashMap;
type Result<T> = std::result::Result<T, String>;
fn bad() -> String {
    "Malformed or unsupported Android manifest.".into()
}
fn u16_at(b: &[u8], n: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        b.get(n..n.checked_add(2).ok_or_else(bad)?)
            .ok_or_else(bad)?
            .try_into()
            .map_err(|_| bad())?,
    ))
}
fn u32_at(b: &[u8], n: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(n..n.checked_add(4).ok_or_else(bad)?)
            .ok_or_else(bad)?
            .try_into()
            .map_err(|_| bad())?,
    ))
}
struct Pool<'a> {
    data: &'a [u8],
    count: usize,
    offsets: usize,
    start: usize,
    end: usize,
    utf8: bool,
}
impl<'a> Pool<'a> {
    fn new(b: &'a [u8]) -> Result<Self> {
        let header = u16_at(b, 2)? as usize;
        if header < 28 || header > b.len() {
            return Err(bad());
        }
        let count = u32_at(b, 8)? as usize;
        let styles = u32_at(b, 12)? as usize;
        let table_bytes = count
            .checked_add(styles)
            .and_then(|v| v.checked_mul(4))
            .ok_or_else(bad)?;
        let start = u32_at(b, 20)? as usize;
        let styles_start = u32_at(b, 24)? as usize;
        if header.checked_add(table_bytes).is_none_or(|n| n > start) || start > b.len() {
            return Err(bad());
        }
        let end = if styles_start == 0 {
            b.len()
        } else {
            styles_start
        };
        if end < start || end > b.len() {
            return Err(bad());
        }
        Ok(Self {
            data: b,
            count,
            offsets: header,
            start,
            end,
            utf8: u32_at(b, 16)? & 0x100 != 0,
        })
    }
    fn text(&self, index: u32, limit: usize) -> Result<String> {
        let index = index as usize;
        if index >= self.count {
            return Err(bad());
        }
        let offset = u32_at(self.data, self.offsets + index * 4)? as usize;
        let mut pos = self
            .start
            .checked_add(offset)
            .filter(|p| *p < self.end)
            .ok_or_else(bad)?;
        let b = &self.data[..self.end];
        if self.utf8 {
            let expected_units = length8(b, &mut pos)?;
            let len = length8(b, &mut pos)?;
            if len > limit {
                return Err("Manifest metadata string exceeds its limit.".into());
            }
            let end = pos.checked_add(len).ok_or_else(bad)?;
            if b.get(end) != Some(&0) {
                return Err(bad());
            }
            let text = std::str::from_utf8(b.get(pos..end).ok_or_else(bad)?).map_err(|_| bad())?;
            if text.encode_utf16().count() != expected_units {
                return Err(bad());
            }
            Ok(text.into())
        } else {
            let len = length16(b, &mut pos)?;
            if len > limit {
                return Err("Manifest metadata string exceeds its limit.".into());
            }
            let end = pos
                .checked_add(len.checked_mul(2).ok_or_else(bad)?)
                .ok_or_else(bad)?;
            if u16_at(b, end)? != 0 {
                return Err(bad());
            }
            let bytes = b.get(pos..end).ok_or_else(bad)?;
            let units: Vec<_> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16(&units).map_err(|_| bad())
        }
    }
}
fn length8(b: &[u8], pos: &mut usize) -> Result<usize> {
    let a = *b.get(*pos).ok_or_else(bad)?;
    *pos += 1;
    if a & 0x80 == 0 {
        Ok(a as usize)
    } else {
        let v = *b.get(*pos).ok_or_else(bad)?;
        *pos += 1;
        Ok(((a as usize & 0x7f) << 8) | v as usize)
    }
}
fn length16(b: &[u8], pos: &mut usize) -> Result<usize> {
    let a = u16_at(b, *pos)?;
    *pos += 2;
    if a & 0x8000 == 0 {
        Ok(a as usize)
    } else {
        let v = u16_at(b, *pos)?;
        *pos += 2;
        Ok(((a as usize & 0x7fff) << 16) | v as usize)
    }
}
#[derive(Debug)]
enum Value {
    Text(String),
    Number(u32),
    Other,
}
fn attributes(b: &[u8], header: usize, pool: &Pool<'_>) -> Result<HashMap<String, Value>> {
    let start = u16_at(b, header + 8)? as usize;
    let size = u16_at(b, header + 10)? as usize;
    let count = u16_at(b, header + 12)? as usize;
    if start < 20 || size < 20 || count > 512 {
        return Err(bad());
    }
    let base = header.checked_add(start).ok_or_else(bad)?;
    let end = base
        .checked_add(size.checked_mul(count).ok_or_else(bad)?)
        .ok_or_else(bad)?;
    if end > b.len() {
        return Err(bad());
    }
    let mut attrs = HashMap::new();
    for i in 0..count {
        let at = base + i * size;
        let ns = u32_at(b, at)?;
        let mut name = pool.text(u32_at(b, at + 4)?, 128)?;
        if ns != u32::MAX {
            let ns = pool.text(ns, 256)?;
            if ns != "http://schemas.android.com/apk/res/android" {
                continue;
            }
            name = format!("android:{name}");
        }
        if u16_at(b, at + 12)? != 8 {
            return Err(bad());
        }
        let raw = u32_at(b, at + 8)?;
        let kind = *b.get(at + 15).ok_or_else(bad)?;
        let data = u32_at(b, at + 16)?;
        if !matches!(
            name.as_str(),
            "package"
                | "split"
                | "android:versionCode"
                | "android:versionCodeMajor"
                | "android:versionName"
                | "android:minSdkVersion"
        ) {
            continue;
        }
        let value = if raw != u32::MAX {
            Value::Text(pool.text(raw, 65536)?)
        } else {
            match kind {
                3 => Value::Text(pool.text(data, 65536)?),
                0x10 | 0x11 => Value::Number(data),
                _ => Value::Other,
            }
        };
        if attrs.insert(name, value).is_some() {
            return Err("Duplicate manifest attribute.".into());
        }
    }
    Ok(attrs)
}
fn number(attrs: &HashMap<String, Value>, name: &str) -> Option<u32> {
    match attrs.get(name)? {
        Value::Number(n) => Some(*n),
        Value::Text(s) => s
            .strip_prefix("0x")
            .and_then(|v| u32::from_str_radix(v, 16).ok())
            .or_else(|| s.parse().ok()),
        Value::Other => None,
    }
}
fn text(attrs: &mut HashMap<String, Value>, name: &str) -> Option<String> {
    match attrs.remove(name)? {
        Value::Text(s) => Some(s),
        _ => None,
    }
}
#[derive(Debug)]
pub(crate) struct Metadata {
    pub package: String,
    pub version_code: u64,
    pub version_name: Option<String>,
    pub min_sdk: Option<u32>,
}
pub(crate) fn parse(bytes: &[u8]) -> Result<Metadata> {
    if bytes.len() > 2 * 1024 * 1024
        || u16_at(bytes, 0)? != 3
        || u16_at(bytes, 2)? != 8
        || u32_at(bytes, 4)? as usize != bytes.len()
    {
        return Err(bad());
    }
    let mut at = 8;
    let mut pool = None;
    let mut metadata = None;
    let mut depth = 0usize;
    let mut chunks = 0;
    while at < bytes.len() {
        chunks += 1;
        if chunks > 16384 {
            return Err("Manifest has too many chunks.".into());
        }
        let kind = u16_at(bytes, at)?;
        let header = u16_at(bytes, at + 2)? as usize;
        let size = u32_at(bytes, at + 4)? as usize;
        if header < 8 || size < header {
            return Err(bad());
        }
        let end = at
            .checked_add(size)
            .filter(|v| *v <= bytes.len())
            .ok_or_else(bad)?;
        let b = &bytes[at..end];
        match kind {
            1 => {
                if pool.is_some() {
                    return Err(bad());
                }
                pool = Some(Pool::new(b)?);
            }
            0x0102 => {
                if header < 16 || size < header + 20 {
                    return Err(bad());
                }
                let pool = pool.as_ref().ok_or_else(bad)?;
                let tag = pool.text(u32_at(b, header + 4)?, 128)?;
                if depth == 0 {
                    if tag != "manifest" || metadata.is_some() {
                        return Err(bad());
                    }
                    let mut attrs = attributes(b, header, pool)?;
                    if attrs.contains_key("split") {
                        return Err(
                            "This POC installs standalone APKs, not individual split APKs.".into(),
                        );
                    }
                    let package = text(&mut attrs, "package")
                        .filter(|v| crate::apk::valid_package(v))
                        .ok_or_else(bad)?;
                    let low = number(&attrs, "android:versionCode")
                        .ok_or("APK version code is missing or invalid.")?
                        as u64;
                    let high = number(&attrs, "android:versionCodeMajor").unwrap_or(0) as u64;
                    metadata = Some(Metadata {
                        package,
                        version_code: (high << 32) | low,
                        version_name: text(&mut attrs, "android:versionName"),
                        min_sdk: None,
                    });
                } else if depth == 1 && tag == "uses-sdk" {
                    let attrs = attributes(b, header, pool)?;
                    metadata.as_mut().ok_or_else(bad)?.min_sdk =
                        number(&attrs, "android:minSdkVersion");
                }
                depth = depth.checked_add(1).ok_or_else(bad)?;
            }
            0x0103 => {
                if header < 16 || size < header + 8 || depth == 0 {
                    return Err(bad());
                }
                depth -= 1;
            }
            _ => {}
        }
        at = end;
    }
    if depth != 0 {
        return Err(bad());
    }
    metadata.ok_or_else(bad)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_declared_lengths_are_bounded() {
        for n in [u32::MAX, 0x40000000, 1024] {
            let mut d = vec![0u8; 36];
            d[..2].copy_from_slice(&3u16.to_le_bytes());
            d[2..4].copy_from_slice(&8u16.to_le_bytes());
            d[4..8].copy_from_slice(&36u32.to_le_bytes());
            d[8..10].copy_from_slice(&1u16.to_le_bytes());
            d[10..12].copy_from_slice(&28u16.to_le_bytes());
            d[12..16].copy_from_slice(&n.to_le_bytes());
            d[16..20].copy_from_slice(&n.to_le_bytes());
            assert!(parse(&d).is_err());
        }
    }
}

#[cfg(test)]
mod extra_bounds_tests {
    use super::*;
    #[test]
    fn pool_count_cannot_exceed_available_offset_table() {
        let mut b = vec![0u8; 28];
        b[2..4].copy_from_slice(&28u16.to_le_bytes());
        b[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        b[20..24].copy_from_slice(&28u32.to_le_bytes());
        assert!(Pool::new(&b).is_err());
    }
    #[test]
    fn string_offset_cannot_escape_pool() {
        let mut b = vec![0u8; 33];
        b[2..4].copy_from_slice(&28u16.to_le_bytes());
        b[8..12].copy_from_slice(&1u32.to_le_bytes());
        b[20..24].copy_from_slice(&32u32.to_le_bytes());
        b[28..32].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(Pool::new(&b).unwrap().text(0, 128).is_err());
    }
    #[test]
    fn truncated_binary_input_never_panics() {
        for n in 0..40 {
            assert!(parse(&vec![0u8; n]).is_err());
        }
    }
    #[test]
    fn utf8_byte_length_can_need_two_bytes_when_character_length_does_not() {
        let value = "é".repeat(100);
        let mut b = vec![0u8; 32];
        b[2..4].copy_from_slice(&28u16.to_le_bytes());
        b[8..12].copy_from_slice(&1u32.to_le_bytes());
        b[16..20].copy_from_slice(&0x100u32.to_le_bytes());
        b[20..24].copy_from_slice(&32u32.to_le_bytes());
        b.extend_from_slice(&[100, 0x80, 200]);
        b.extend_from_slice(value.as_bytes());
        b.push(0);
        assert_eq!(Pool::new(&b).unwrap().text(0, 65536).unwrap(), value);
    }
}
