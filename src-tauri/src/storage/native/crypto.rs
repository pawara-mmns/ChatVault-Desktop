//! Independent implementation of a deliberately narrow, documented envelope.
//! No upstream program/code is bundled. See docs/NATIVE_COMPATIBILITY.md.
use super::models::*;
use crate::storage::error::Result;
use aes_gcm::{
    AesGcm, Nonce, Tag,
    aead::{AeadInPlace, KeyInit, consts::U16},
};
use md5::Md5;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use zeroize::{Zeroize, Zeroizing};
pub const MAX_ENCRYPTED: u64 = 256 * 1024 * 1024;
pub const MAX_PLAIN: u64 = 1024 * 1024 * 1024;
pub fn standalone_checksum(
    path: &std::path::Path,
    mut check: impl FnMut(u64) -> Result<()>,
) -> Result<bool> {
    let mut file = crate::storage::paths::open_regular_file(path)?;
    let mut digest = Md5::new();
    let mut buffer = vec![0; 256 * 1024];
    let mut tail = Vec::new();
    let mut count = 0;
    loop {
        check(count)?;
        let n = file.read(&mut buffer).map_err(|_| fail("native_io"))?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if n >= 16 {
            digest.update(&tail);
            digest.update(&buffer[..n - 16]);
            tail.clear();
            tail.extend_from_slice(&buffer[n - 16..n]);
        } else {
            let k = (tail.len() + n).saturating_sub(16);
            digest.update(&tail[..k]);
            tail.drain(..k);
            tail.extend_from_slice(&buffer[..n]);
        }
    }
    check(count)?;
    Ok(tail.len() == 16 && digest.finalize().as_slice() == tail.as_slice())
}
#[derive(Clone)]
pub struct Header {
    pub format: &'static str,
    pub offset: usize,
    pub iv: [u8; 16],
    pub salt: Option<Vec<u8>>,
    pub account: Option<Vec<u8>>,
    pub version: Option<u8>,
}
pub enum Field<'a> {
    Integer(u64),
    Bytes(&'a [u8]),
}
fn varint(b: &[u8], at: &mut usize) -> Result<u64> {
    let mut v = 0;
    for shift in (0..35).step_by(7) {
        let n = *b.get(*at).ok_or_else(|| fail("native_format"))?;
        *at += 1;
        v |= ((n & 127) as u64) << shift;
        if n & 128 == 0 {
            return Ok(v);
        }
    }
    Err(fail("native_format"))
}
fn fields(b: &[u8]) -> Result<std::collections::BTreeMap<u64, Field<'_>>> {
    let mut out = std::collections::BTreeMap::new();
    let mut at = 0;
    while at < b.len() {
        let tag = varint(b, &mut at)?;
        let id = tag >> 3;
        if id == 0 || out.contains_key(&id) {
            return Err(fail("native_format"));
        }
        let field = match tag & 7 {
            0 => Field::Integer(varint(b, &mut at)?),
            2 => {
                let n = varint(b, &mut at)? as usize;
                let end = at
                    .checked_add(n)
                    .filter(|e| *e <= b.len())
                    .ok_or_else(|| fail("native_format"))?;
                let f = Field::Bytes(&b[at..end]);
                at = end;
                f
            }
            _ => return Err(fail("native_format")),
        };
        out.insert(id, field);
    }
    Ok(out)
}
pub fn header(b: &[u8]) -> Result<Header> {
    let mut at = 0;
    let n = varint(b, &mut at)? as usize;
    if n == 0 || n > 4096 || at + n > b.len() {
        return Err(fail("native_format"));
    }
    let f = fields(&b[at..at + n])?;
    if f.keys().any(|k| !matches!(k, 1 | 2 | 3 | 4 | 6)) {
        return Err(fail("native_format"));
    }
    let key_type = match f.get(&6).or(f.get(&1)) {
        Some(Field::Integer(n)) => *n,
        None => 0,
        _ => return Err(fail("native_format")),
    };
    if f.contains_key(&2) && f.contains_key(&3) {
        return Err(fail("native_format"));
    }
    let (format, cipher, iv_field) = match (f.get(&2), f.get(&3)) {
        (Some(Field::Bytes(c)), None) if key_type == 0 => ("crypt14", fields(c)?, 5),
        (None, Some(Field::Bytes(c))) if matches!(key_type, 1 | 3) => ("crypt15", fields(c)?, 1),
        _ => return Err(fail("native_format")),
    };
    let bytes = |id| match cipher.get(&id) {
        Some(Field::Bytes(b)) => Ok(b.to_vec()),
        _ => Err(fail("native_format")),
    };
    let iv: [u8; 16] = bytes(iv_field)?
        .try_into()
        .map_err(|_| fail("native_format"))?;
    let (salt, account, version) = if format == "crypt14" {
        if bytes(1)? != [0, 1] {
            return Err(fail("native_format"));
        }
        let version = bytes(2)?;
        if version.len() != 1 || !matches!(version[0], b'1'..=b'3') {
            return Err(fail("native_format"));
        }
        let salt = bytes(3)?;
        let account = bytes(4)?;
        if salt.len() != 32 || account.len() != 16 {
            return Err(fail("native_format"));
        }
        (Some(salt), Some(account), Some(version[0] - b'0'))
    } else {
        (None, None, None)
    };
    Ok(Header {
        format,
        offset: at + n,
        iv,
        salt,
        account,
        version,
    })
}
pub fn hex_key(text: &str) -> Result<Zeroizing<Vec<u8>>> {
    if text.len() != 64 || !text.as_bytes().iter().all(|c| c.is_ascii_hexdigit()) {
        return Err(fail("native_key"));
    }
    let mut out = Zeroizing::new(Vec::with_capacity(32));
    for i in (0..64).step_by(2) {
        out.push(u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| fail("native_key"))?);
    }
    Ok(out)
}
pub fn key_file(bytes: Zeroizing<Vec<u8>>, h: &Header) -> Result<Zeroizing<Vec<u8>>> {
    // Only the exact Java serialized byte-array envelope is accepted, never general object deserialization.
    const JAVA: [u8; 23] = [
        0xac, 0xed, 0, 5, 0x75, 0x72, 0, 2, 0x5b, 0x42, 0xac, 0xf3, 0x17, 0xf8, 6, 8, 0x54, 0xe0,
        2, 0, 0, 0x78, 0x70,
    ];
    let raw = if bytes.len() == 158 && bytes[..23] == JAVA && bytes[23..27] == 131u32.to_be_bytes()
    {
        &bytes[27..]
    } else {
        &bytes[..]
    };
    if h.format != "crypt14"
        || raw.len() != 131
        || raw[..2] != [0, 1]
        || !matches!(raw[2], 1..=3)
        || Some(raw[2]) != h.version
        || Some(&raw[3..35]) != h.salt.as_deref()
        || Some(&raw[35..51]) != h.account.as_deref()
        || Sha256::digest(&raw[35..51]).as_slice() != &raw[51..83]
        || raw[83..99].iter().any(|b| *b != 0)
    {
        return Err(fail("native_key"));
    }
    Ok(Zeroizing::new(raw[99..131].to_vec()))
}
pub fn decrypt(
    mut bytes: Zeroizing<Vec<u8>>,
    h: &Header,
    key: Zeroizing<Vec<u8>>,
    mut check: impl FnMut() -> Result<()>,
) -> Result<Zeroizing<Vec<u8>>> {
    check()?;
    if bytes.len() < h.offset + 33 || bytes.len() as u64 > MAX_ENCRYPTED {
        return Err(fail("native_format"));
    }
    let len = bytes.len();
    if Md5::digest(&bytes[..len - 16]).as_slice() != &bytes[len - 16..] {
        return Err(fail("native_integrity"));
    }
    let mut derived = Zeroizing::new([0u8; 32]);
    if key.len() != 32 {
        return Err(fail("native_key"));
    }
    if h.format == "crypt15" {
        hkdf::Hkdf::<Sha256>::new(Some(&[0u8; 32]), &key)
            .expand(b"backup encryption", derived.as_mut())
            .map_err(|_| fail("native_key"))?;
    } else {
        derived.copy_from_slice(&key);
    }
    let cipher = AesGcm::<aes::Aes256, U16>::new_from_slice(derived.as_ref())
        .map_err(|_| fail("native_key"))?;
    let tag = Tag::clone_from_slice(&bytes[len - 32..len - 16]);
    cipher
        .decrypt_in_place_detached(
            Nonce::<U16>::from_slice(&h.iv),
            b"",
            &mut bytes[h.offset..len - 32],
            &tag,
        )
        .map_err(|_| fail("native_auth"))?;
    check()?;
    bytes.copy_within(h.offset..len - 32, 0);
    bytes[len - 32 - h.offset..].zeroize();
    bytes.truncate(len - 32 - h.offset);
    Ok(bytes)
}
pub fn plaintext(
    bytes: &[u8],
    output: &mut impl Write,
    mut check: impl FnMut(u64) -> Result<()>,
) -> Result<u64> {
    let raw = bytes.starts_with(b"SQLite format 3\0");
    let mut plain = std::io::Cursor::new(bytes);
    let mut compressed = flate2::bufread::ZlibDecoder::new(bytes);
    let source: &mut dyn Read = if raw { &mut plain } else { &mut compressed };
    let mut chunk = Zeroizing::new(vec![0u8; 128 * 1024]);
    let mut count = 0;
    let mut first = true;
    loop {
        check(count)?;
        let n = source
            .read(&mut chunk)
            .map_err(|_| fail("native_database"))?;
        if n == 0 {
            break;
        }
        if first {
            if n < 16 || !chunk[..n].starts_with(b"SQLite format 3\0") {
                return Err(fail("native_database"));
            }
            first = false;
        }
        count += n as u64;
        if count > MAX_PLAIN
            || count
                > (bytes.len() as u64)
                    .saturating_mul(200)
                    .saturating_add(1024 * 1024)
        {
            return Err(fail("native_limit"));
        }
        output
            .write_all(&chunk[..n])
            .map_err(|_| fail("native_io"))?;
    }
    if !raw && compressed.total_in() != bytes.len() as u64 {
        return Err(fail("native_database"));
    }
    if count < 100 {
        return Err(fail("native_database"));
    }
    Ok(count)
}
