use qrcodegen::{QrCode, QrCodeEcc};
use wasm_bindgen::prelude::*;

mod selo;

const ULID_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const KSUID_ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const BASE58_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const BASE64URL_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const KSUID_EPOCH: u64 = 1_400_000_000;
const ULID_MAX_TIME: u64 = (1u64 << 48) - 1;
const KSUID_MAX_TIME: u64 = KSUID_EPOCH + u32::MAX as u64;

struct Ddd {
    code: &'static str,
    state: &'static str,
    region: &'static str,
}

struct Country {
    iso: &'static str,
    name: &'static str,
    code: &'static str,
    note: &'static str,
}

#[wasm_bindgen]
pub fn rot13(input: &str) -> String {
    input
        .bytes()
        .map(|b| match b {
            b'a'..=b'z' => b'a' + ((b - b'a' + 13) % 26),
            b'A'..=b'Z' => b'A' + ((b - b'A' + 13) % 26),
            _ => b,
        })
        .map(char::from)
        .collect()
}

#[wasm_bindgen]
pub fn caesar(input: &str, shift: i32) -> String {
    let shift = shift.rem_euclid(26) as u8;
    input
        .bytes()
        .map(|b| match b {
            b'a'..=b'z' => b'a' + ((b - b'a' + shift) % 26),
            b'A'..=b'Z' => b'A' + ((b - b'A' + shift) % 26),
            _ => b,
        })
        .map(char::from)
        .collect()
}

#[wasm_bindgen]
pub fn vigenere(input: &str, key: &str, decode: bool) -> Result<String, JsValue> {
    let shifts: Vec<u8> = key
        .bytes()
        .filter_map(alpha_index)
        .collect();
    if shifts.is_empty() {
        return Err(err("key must contain at least one ASCII letter"));
    }

    let mut pos = 0usize;
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        let Some(base) = alpha_base(b) else {
            out.push(char::from(b));
            continue;
        };
        let shift = shifts[pos % shifts.len()];
        let shift = if decode { (26 - shift) % 26 } else { shift };
        out.push(char::from(base + ((b - base + shift) % 26)));
        pos += 1;
    }
    Ok(out)
}

#[wasm_bindgen]
pub fn ulid(timestamp_ms: f64, random: &[u8]) -> Result<String, JsValue> {
    if random.len() < 10 {
        return Err(err("ULID needs 10 random bytes"));
    }
    let timestamp_ms = checked_u64(timestamp_ms, ULID_MAX_TIME, "ULID timestamp")?;
    let mut bytes = [0u8; 16];
    bytes[0] = (timestamp_ms >> 40) as u8;
    bytes[1] = (timestamp_ms >> 32) as u8;
    bytes[2] = (timestamp_ms >> 24) as u8;
    bytes[3] = (timestamp_ms >> 16) as u8;
    bytes[4] = (timestamp_ms >> 8) as u8;
    bytes[5] = timestamp_ms as u8;
    bytes[6..].copy_from_slice(&random[..10]);
    Ok(encode_ulid_bytes(&bytes))
}

#[wasm_bindgen]
pub fn inspect_ulid(input: &str) -> Result<String, JsValue> {
    let bytes = decode_ulid(input)?;
    let timestamp_ms = ((bytes[0] as u64) << 40)
        | ((bytes[1] as u64) << 32)
        | ((bytes[2] as u64) << 24)
        | ((bytes[3] as u64) << 16)
        | ((bytes[4] as u64) << 8)
        | bytes[5] as u64;
    Ok(format!(
        "{{\"kind\":\"ulid\",\"id\":\"{}\",\"timestamp_ms\":{},\"random\":\"{}\",\"raw\":\"{}\"}}",
        input.trim().to_uppercase(),
        timestamp_ms,
        hex(&bytes[6..]),
        hex(&bytes)
    ))
}

#[wasm_bindgen]
pub fn uuid_v7(timestamp_ms: f64, random: &[u8]) -> Result<String, JsValue> {
    if random.len() < 10 {
        return Err(err("UUIDv7 needs 10 random bytes"));
    }
    let timestamp_ms = checked_u64(timestamp_ms, ULID_MAX_TIME, "UUIDv7 timestamp")?;
    let mut bytes = [0u8; 16];
    bytes[0] = (timestamp_ms >> 40) as u8;
    bytes[1] = (timestamp_ms >> 32) as u8;
    bytes[2] = (timestamp_ms >> 24) as u8;
    bytes[3] = (timestamp_ms >> 16) as u8;
    bytes[4] = (timestamp_ms >> 8) as u8;
    bytes[5] = timestamp_ms as u8;
    bytes[6] = 0x70 | (random[0] >> 4);
    bytes[7] = (random[0] << 4) | (random[1] >> 4);
    bytes[8] = 0x80 | (random[2] & 0x3f);
    bytes[9..].copy_from_slice(&random[3..10]);
    Ok(uuid_string(&bytes))
}

#[wasm_bindgen]
pub fn inspect_uuid_v7(input: &str) -> Result<String, JsValue> {
    let bytes = decode_uuid(input)?;
    if bytes[6] >> 4 != 7 {
        return Err(err("UUID is not version 7"));
    }
    if bytes[8] >> 6 != 2 {
        return Err(err("UUID has an invalid RFC 4122/9562 variant"));
    }
    let timestamp_ms = ((bytes[0] as u64) << 40)
        | ((bytes[1] as u64) << 32)
        | ((bytes[2] as u64) << 24)
        | ((bytes[3] as u64) << 16)
        | ((bytes[4] as u64) << 8)
        | bytes[5] as u64;
    Ok(format!(
        "{{\"kind\":\"uuid_v7\",\"id\":\"{}\",\"timestamp_ms\":{},\"raw\":\"{}\"}}",
        uuid_string(&bytes),
        timestamp_ms,
        hex(&bytes)
    ))
}

#[wasm_bindgen]
pub fn ksuid(timestamp_secs: f64, payload: &[u8]) -> Result<String, JsValue> {
    if payload.len() < 16 {
        return Err(err("KSUID needs 16 payload bytes"));
    }
    let timestamp_secs = checked_u64(timestamp_secs, KSUID_MAX_TIME, "KSUID timestamp")?;
    if timestamp_secs < KSUID_EPOCH {
        return Err(err("KSUID timestamp is before epoch 1400000000"));
    }

    let corrected = (timestamp_secs - KSUID_EPOCH) as u32;
    let mut bytes = [0u8; 20];
    bytes[..4].copy_from_slice(&corrected.to_be_bytes());
    bytes[4..].copy_from_slice(&payload[..16]);
    Ok(encode_base62(&bytes, 27))
}

#[wasm_bindgen]
pub fn inspect_ksuid(input: &str) -> Result<String, JsValue> {
    let bytes = decode_base62_20(input)?;
    let corrected = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as u64;
    let timestamp_secs = KSUID_EPOCH + corrected;
    Ok(format!(
        "{{\"kind\":\"ksuid\",\"id\":\"{}\",\"timestamp_secs\":{},\"payload\":\"{}\",\"raw\":\"{}\"}}",
        input.trim(),
        timestamp_secs,
        hex(&bytes[4..]),
        hex(&bytes)
    ))
}

#[wasm_bindgen]
pub fn qr_matrix(text: &str, ecc: &str) -> Result<String, JsValue> {
    if text.is_empty() {
        return Err(err("QR content cannot be empty"));
    }
    let qr = QrCode::encode_text(text, qr_ecc(ecc)).map_err(|_| err("QR content is too long"))?;
    let size = qr.size();
    let mut modules = String::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            modules.push(if qr.get_module(x, y) { '1' } else { '0' });
        }
    }
    Ok(format!(
        "{{\"size\":{},\"modules\":\"{}\",\"ecc\":\"{}\"}}",
        size,
        modules,
        ecc.to_ascii_uppercase()
    ))
}

#[wasm_bindgen]
pub fn phone_lookup(input: &str) -> String {
    let query = input.trim();
    let digits: String = query.chars().filter(|c| c.is_ascii_digit()).collect();
    let normalized = query.to_ascii_lowercase();
    let mut results = Vec::new();
    let phone_summary = selo::phone_summary(query);

    if brazil_ddd_candidate(query, &digits) {
        let ddd_prefix = if query.starts_with("+55") && digits.len() >= 4 {
            &digits[2..4]
        } else if digits.starts_with("0055") && digits.len() >= 6 {
            &digits[4..6]
        } else if digits.starts_with("55") && digits.len() > 11 {
            &digits[2..4]
        } else {
            &digits[0..2]
        };
        if let Some(ddd) = DDD_CODES.iter().find(|item| item.code == ddd_prefix) {
            results.push(format!(
                "{{\"kind\":\"Brazil DDD\",\"iso\":\"br\",\"title\":\"DDD {} - {}\",\"subtitle\":\"{}\",\"code\":\"{}\",\"source\":\"Wikipedia DDD table\"}}",
                ddd.code, ddd.state, ddd.region, ddd.code
            ));
        }
    }

    let mut countries: Vec<&Country> = if digits.is_empty() {
        COUNTRY_CODES
            .iter()
            .filter(|country| {
                country.name.to_ascii_lowercase().contains(&normalized)
                    || country.iso == normalized.as_str()
                    || country.code == normalized.trim_start_matches('+')
            })
            .collect()
    } else {
        COUNTRY_CODES
            .iter()
            .filter(|country| digits.starts_with(country.code) || country.code.starts_with(&digits))
            .collect()
    };
    countries.sort_by(|a, b| b.code.len().cmp(&a.code.len()).then_with(|| a.name.cmp(b.name)));
    for country in countries.iter().take(12) {
        let subtitle = if country.note.is_empty() { country.iso.to_ascii_uppercase() } else { country.note.to_string() };
        results.push(format!(
            "{{\"kind\":\"Country calling code\",\"iso\":\"{}\",\"title\":\"{} +{}\",\"subtitle\":\"{}\",\"code\":\"+{}\",\"source\":\"Wikipedia telephone country codes\"}}",
            country.iso, country.name, country.code, subtitle, country.code
        ));
    }

    format!(
        "{{\"query\":\"{}\",\"formatted_query\":{},\"national_phone\":{},\"e164\":{},\"phone_uf\":{},\"results\":[{}],\"ddd_count\":{},\"country_count\":{}}}",
        json_escape(query),
        json_option(phone_summary.as_ref().map(|summary| summary.formatted.as_str())),
        json_option(phone_summary.as_ref().map(|summary| summary.national.as_str())),
        json_option(phone_summary.as_ref().map(|summary| summary.e164.as_str())),
        json_option(phone_summary.as_ref().map(|summary| summary.uf)),
        results.join(","),
        DDD_CODES.len(),
        COUNTRY_CODES.len()
    )
}

fn brazil_ddd_candidate(query: &str, digits: &str) -> bool {
    if digits.len() < 2 {
        return false;
    }
    let trimmed = query.trim();
    if trimmed.starts_with('+') {
        return digits.starts_with("55") && digits.len() >= 4;
    }
    if digits.starts_with("00") {
        return digits.starts_with("0055") && digits.len() >= 6;
    }
    if digits.starts_with("55") && digits.len() > 11 {
        return true;
    }
    digits.len() <= 11
}

#[wasm_bindgen]
pub fn br_document(
    kind: &str,
    action: &str,
    input: &str,
    uf: &str,
    random: &[u8],
) -> Result<String, JsValue> {
    selo::br_document(kind, action, input, uf, random)
}

#[wasm_bindgen]
pub fn phone_format(input: &str) -> String {
    selo::phone_summary(input)
        .map(|summary| summary.formatted)
        .unwrap_or_else(|| input.trim().to_string())
}

#[wasm_bindgen]
pub fn password_generate(
    length: u32,
    count: u32,
    lowercase: bool,
    uppercase: bool,
    numbers: bool,
    symbols: bool,
    exclude_similar: bool,
    exclude_ambiguous: bool,
    require_each: bool,
    random: &[u8],
) -> Result<String, JsValue> {
    let length = length.clamp(1, 512) as usize;
    let count = count.clamp(1, 100) as usize;
    let mut sets = Vec::new();
    if lowercase { sets.push(strip_password_chars(b"abcdefghijklmnopqrstuvwxyz", exclude_similar, exclude_ambiguous)); }
    if uppercase { sets.push(strip_password_chars(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ", exclude_similar, exclude_ambiguous)); }
    if numbers { sets.push(strip_password_chars(b"0123456789", exclude_similar, exclude_ambiguous)); }
    if symbols { sets.push(strip_password_chars(b"!@#$%^&*()-_=+[]{};:,.<>?/", exclude_similar, exclude_ambiguous)); }
    sets.retain(|set| !set.is_empty());
    if sets.is_empty() {
        return Err(err("Select at least one character set"));
    }
    if require_each && length < sets.len() {
        return Err(err("Length is too short for the required character sets"));
    }

    let pool = unique_bytes(&sets.concat());
    let mut rng = RandomCursor::new(random);
    let mut passwords = Vec::with_capacity(count);
    for _ in 0..count {
        let mut chars = Vec::with_capacity(length);
        if require_each {
            for set in &sets {
                chars.push(set[rng.index(set.len())?]);
            }
        }
        while chars.len() < length {
            chars.push(pool[rng.index(pool.len())?]);
        }
        shuffle_bytes(&mut chars, &mut rng)?;
        passwords.push(String::from_utf8(chars).expect("password alphabet is ASCII"));
    }
    Ok(passwords.join("\n"))
}

#[wasm_bindgen]
pub fn hex_encode(input: &str) -> String {
    hex_lower(input.as_bytes())
}

#[wasm_bindgen]
pub fn hex_decode(input: &str) -> Result<String, JsValue> {
    let bytes = decode_hex(input)?;
    String::from_utf8(bytes).map_err(|_| err("hex decoded bytes are not valid UTF-8"))
}

#[wasm_bindgen]
pub fn base32_encode(input: &str) -> String {
    encode_base32(input.as_bytes())
}

#[wasm_bindgen]
pub fn base32_decode(input: &str) -> Result<String, JsValue> {
    let bytes = decode_base32(input)?;
    String::from_utf8(bytes).map_err(|_| err("Base32 decoded bytes are not valid UTF-8"))
}

#[wasm_bindgen]
pub fn base58_encode(input: &str) -> String {
    encode_base_n(input.as_bytes(), BASE58_ALPHABET)
}

#[wasm_bindgen]
pub fn base58_decode(input: &str) -> Result<String, JsValue> {
    let bytes = decode_base_n(input, BASE58_ALPHABET)?;
    String::from_utf8(bytes).map_err(|_| err("Base58 decoded bytes are not valid UTF-8"))
}

#[wasm_bindgen]
pub fn base64url_encode(input: &str) -> String {
    encode_base64url(input.as_bytes())
}

#[wasm_bindgen]
pub fn base64url_decode(input: &str) -> Result<String, JsValue> {
    let bytes = decode_base64url(input)?;
    String::from_utf8(bytes).map_err(|_| err("Base64url decoded bytes are not valid UTF-8"))
}

#[wasm_bindgen]
pub fn crc32(input: &str) -> String {
    format!("{:08x}", crc32_bytes(input.as_bytes()))
}

#[wasm_bindgen]
pub fn fnv1a32(input: &str) -> String {
    let mut hash = 0x811c9dc5u32;
    for byte in input.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    format!("{hash:08x}")
}

#[wasm_bindgen]
pub fn fnv1a64(input: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[wasm_bindgen]
pub fn morse_encode(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_whitespace() {
                "/".to_string()
            } else {
                morse_for(c.to_ascii_uppercase()).unwrap_or("?").to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[wasm_bindgen]
pub fn morse_decode(input: &str) -> String {
    input
        .split_whitespace()
        .map(|token| if token == "/" { ' ' } else { char_for_morse(token).unwrap_or('?') })
        .collect()
}

#[wasm_bindgen]
pub fn leet_encode(input: &str) -> String {
    input.chars().map(|c| match c {
        'a' | 'A' => '4',
        'e' | 'E' => '3',
        'i' | 'I' => '1',
        'o' | 'O' => '0',
        's' | 'S' => '5',
        't' | 'T' => '7',
        _ => c,
    }).collect()
}

#[wasm_bindgen]
pub fn leet_decode(input: &str) -> String {
    input.chars().map(|c| match c {
        '4' => 'a',
        '3' => 'e',
        '1' => 'i',
        '0' => 'o',
        '5' => 's',
        '7' => 't',
        _ => c,
    }).collect()
}

#[wasm_bindgen]
pub fn atbash(input: &str) -> String {
    input.bytes().map(|b| match b {
        b'a'..=b'z' => (b'z' - (b - b'a')) as char,
        b'A'..=b'Z' => (b'Z' - (b - b'A')) as char,
        _ => b as char,
    }).collect()
}

#[wasm_bindgen]
pub fn polybius_encode(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            let u = if c == 'J' { 'I' } else { c.to_ascii_uppercase() };
            if !u.is_ascii_alphabetic() {
                c.to_string()
            } else {
                let idx = if u > 'J' { u as u8 - b'A' - 1 } else { u as u8 - b'A' };
                format!("{}{}", idx / 5 + 1, idx % 5 + 1)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[wasm_bindgen]
pub fn polybius_decode(input: &str) -> String {
    let mut out = String::new();
    for token in input.split_whitespace() {
        let bytes = token.as_bytes();
        if bytes.len() == 2 && (b'1'..=b'5').contains(&bytes[0]) && (b'1'..=b'5').contains(&bytes[1]) {
            let idx = (bytes[0] - b'1') * 5 + (bytes[1] - b'1');
            let letter = if idx >= 9 { b'A' + idx + 1 } else { b'A' + idx };
            out.push(letter as char);
        } else {
            if !out.is_empty() { out.push(' '); }
            out.push_str(token);
        }
    }
    out
}

fn encode_ulid_bytes(bytes: &[u8; 16]) -> String {
    let mut value = u128::from_be_bytes(*bytes);
    let mut out = [b'0'; 26];
    for slot in out.iter_mut().rev() {
        *slot = ULID_ALPHABET[(value & 31) as usize];
        value >>= 5;
    }
    String::from_utf8(out.to_vec()).expect("ULID alphabet is ASCII")
}

fn decode_ulid(input: &str) -> Result<[u8; 16], JsValue> {
    let text = input.trim();
    if text.len() != 26 {
        return Err(err("ULID must be 26 characters"));
    }

    let mut value = 0u128;
    for (idx, b) in text.bytes().enumerate() {
        let digit = ulid_value(b).ok_or_else(|| err("ULID contains an invalid character"))?;
        if idx == 0 && digit > 7 {
            return Err(err("ULID exceeds the 128-bit canonical range"));
        }
        value = (value << 5) | digit as u128;
    }
    Ok(value.to_be_bytes())
}

fn encode_base62(bytes: &[u8], width: usize) -> String {
    let mut value = bytes.to_vec();
    let mut out = vec![b'0'; width];
    let mut pos = width;

    while !value.is_empty() {
        let mut quotient = Vec::with_capacity(value.len());
        let mut remainder = 0u16;
        for byte in value {
            let n = remainder * 256 + byte as u16;
            let digit = n / 62;
            remainder = n % 62;
            if !quotient.is_empty() || digit != 0 {
                quotient.push(digit as u8);
            }
        }
        pos -= 1;
        out[pos] = KSUID_ALPHABET[remainder as usize];
        value = quotient;
    }

    String::from_utf8(out).expect("KSUID alphabet is ASCII")
}

fn decode_base62_20(input: &str) -> Result<[u8; 20], JsValue> {
    let text = input.trim();
    if text.len() != 27 {
        return Err(err("KSUID must be 27 characters"));
    }

    let mut out = [0u8; 20];
    for b in text.bytes() {
        let digit = ksuid_value(b).ok_or_else(|| err("KSUID contains an invalid character"))?;
        let mut carry = digit as u16;
        for byte in out.iter_mut().rev() {
            let n = (*byte as u16) * 62 + carry;
            *byte = (n & 0xff) as u8;
            carry = n >> 8;
        }
        if carry != 0 {
            return Err(err("KSUID exceeds the 160-bit canonical range"));
        }
    }
    Ok(out)
}

fn checked_u64(value: f64, max: u64, label: &str) -> Result<u64, JsValue> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
        return Err(err(&format!("{label} must be a non-negative integer")));
    }
    if value > max as f64 {
        return Err(err(&format!("{label} is out of range")));
    }
    Ok(value as u64)
}

fn alpha_base(b: u8) -> Option<u8> {
    match b {
        b'a'..=b'z' => Some(b'a'),
        b'A'..=b'Z' => Some(b'A'),
        _ => None,
    }
}

fn alpha_index(b: u8) -> Option<u8> {
    alpha_base(b).map(|base| b.to_ascii_uppercase() - base.to_ascii_uppercase())
}

fn ulid_value(b: u8) -> Option<u8> {
    match b.to_ascii_uppercase() {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'H' => Some(10 + b.to_ascii_uppercase() - b'A'),
        b'J'..=b'K' => Some(18 + b.to_ascii_uppercase() - b'J'),
        b'M'..=b'N' => Some(20 + b.to_ascii_uppercase() - b'M'),
        b'P'..=b'T' => Some(22 + b.to_ascii_uppercase() - b'P'),
        b'V'..=b'Z' => Some(27 + b.to_ascii_uppercase() - b'V'),
        _ => None,
    }
}

fn ksuid_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'Z' => Some(10 + b - b'A'),
        b'a'..=b'z' => Some(36 + b - b'a'),
        _ => None,
    }
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from(HEX[(b >> 4) as usize]));
        out.push(char::from(HEX[(b & 0x0f) as usize]));
    }
    out
}

fn uuid_string(bytes: &[u8; 16]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(36);
    for (idx, b) in bytes.iter().enumerate() {
        if matches!(idx, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        out.push(char::from(HEX[(b >> 4) as usize]));
        out.push(char::from(HEX[(b & 0x0f) as usize]));
    }
    out
}

fn decode_uuid(input: &str) -> Result<[u8; 16], JsValue> {
    let mut hex_chars = Vec::with_capacity(32);
    for b in input.trim().bytes() {
        if b == b'-' {
            continue;
        }
        hex_chars.push(b);
    }
    if hex_chars.len() != 32 {
        return Err(err("UUID must contain 32 hex characters"));
    }

    let mut out = [0u8; 16];
    for (idx, pair) in hex_chars.chunks_exact(2).enumerate() {
        let hi = hex_value(pair[0]).ok_or_else(|| err("UUID contains an invalid hex character"))?;
        let lo = hex_value(pair[1]).ok_or_else(|| err("UUID contains an invalid hex character"))?;
        out[idx] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + b - b'a'),
        b'A'..=b'F' => Some(10 + b - b'A'),
        _ => None,
    }
}

fn err(message: &str) -> JsValue {
    JsValue::from_str(message)
}

fn qr_ecc(value: &str) -> QrCodeEcc {
    match value.to_ascii_uppercase().as_str() {
        "L" | "LOW" => QrCodeEcc::Low,
        "Q" | "QUARTILE" => QrCodeEcc::Quartile,
        "H" | "HIGH" => QrCodeEcc::High,
        _ => QrCodeEcc::Medium,
    }
}

struct RandomCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> RandomCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn byte(&mut self) -> Result<u8, JsValue> {
        let Some(byte) = self.bytes.get(self.pos) else {
            return Err(err("not enough random bytes"));
        };
        self.pos += 1;
        Ok(*byte)
    }

    fn index(&mut self, max: usize) -> Result<usize, JsValue> {
        if max == 0 || max > u32::MAX as usize {
            return Err(err("invalid random range"));
        }
        let max = max as u32;
        let limit = u32::MAX - (u32::MAX % max);
        loop {
            let value = u32::from_le_bytes([self.byte()?, self.byte()?, self.byte()?, self.byte()?]);
            if value < limit {
                return Ok((value % max) as usize);
            }
        }
    }
}

fn strip_password_chars(chars: &[u8], exclude_similar: bool, exclude_ambiguous: bool) -> Vec<u8> {
    chars
        .iter()
        .copied()
        .filter(|byte| {
            !(exclude_similar && b"Il1O0".contains(byte))
                && !(exclude_ambiguous && b"{}[]()/\\'\"`~,;:.<>".contains(byte))
        })
        .collect()
}

fn unique_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for byte in bytes {
        if !out.contains(byte) {
            out.push(*byte);
        }
    }
    out
}

fn shuffle_bytes(bytes: &mut [u8], rng: &mut RandomCursor<'_>) -> Result<(), JsValue> {
    for i in (1..bytes.len()).rev() {
        let j = rng.index(i + 1)?;
        bytes.swap(i, j);
    }
    Ok(())
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn decode_hex(input: &str) -> Result<Vec<u8>, JsValue> {
    let clean: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if clean.len() % 2 != 0 {
        return Err(err("hex input must have an even number of digits"));
    }
    let mut out = Vec::with_capacity(clean.len() / 2);
    for pair in clean.chunks_exact(2) {
        let hi = hex_value(pair[0]).ok_or_else(|| err("hex input contains an invalid digit"))?;
        let lo = hex_value(pair[1]).ok_or_else(|| err("hex input contains an invalid digit"))?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn encode_base32(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for byte in bytes {
        buffer = (buffer << 8) | *byte as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(BASE32_ALPHABET[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(BASE32_ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    while out.len() % 8 != 0 {
        out.push('=');
    }
    out
}

fn decode_base32(input: &str) -> Result<Vec<u8>, JsValue> {
    let mut buffer = 0u32;
    let mut bits = 0u8;
    let mut out = Vec::new();
    for b in input.bytes().filter(|b| !b.is_ascii_whitespace() && *b != b'=') {
        let value = base32_value(b).ok_or_else(|| err("Base32 input contains an invalid character"))?;
        buffer = (buffer << 5) | value as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

fn base32_value(b: u8) -> Option<u8> {
    match b.to_ascii_uppercase() {
        b'A'..=b'Z' => Some(b.to_ascii_uppercase() - b'A'),
        b'2'..=b'7' => Some(26 + b - b'2'),
        _ => None,
    }
}

fn encode_base_n(bytes: &[u8], alphabet: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let base = alphabet.len() as u16;
    let mut value = bytes.to_vec();
    let zero_count = bytes.iter().take_while(|b| **b == 0).count();
    let mut out = Vec::new();
    while !value.is_empty() {
        let mut quotient = Vec::with_capacity(value.len());
        let mut remainder = 0u16;
        for byte in value {
            let n = remainder * 256 + byte as u16;
            let digit = n / base;
            remainder = n % base;
            if !quotient.is_empty() || digit != 0 {
                quotient.push(digit as u8);
            }
        }
        out.push(alphabet[remainder as usize]);
        value = quotient;
    }
    for _ in 0..zero_count {
        out.push(alphabet[0]);
    }
    out.reverse();
    String::from_utf8(out).expect("base alphabet is ASCII")
}

fn decode_base_n(input: &str, alphabet: &[u8]) -> Result<Vec<u8>, JsValue> {
    let clean: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if clean.is_empty() {
        return Ok(Vec::new());
    }
    let base = alphabet.len() as u32;
    let zero_count = clean.iter().take_while(|b| **b == alphabet[0]).count();
    let mut out = Vec::<u8>::new();
    for b in clean {
        let value = alphabet
            .iter()
            .position(|candidate| *candidate == b)
            .ok_or_else(|| err("base input contains an invalid character"))? as u32;
        let mut carry = value;
        for byte in out.iter_mut().rev() {
            let n = (*byte as u32) * base + carry;
            *byte = (n & 0xff) as u8;
            carry = n >> 8;
        }
        while carry > 0 {
            out.insert(0, (carry & 0xff) as u8);
            carry >>= 8;
        }
    }
    let mut prefixed = vec![0; zero_count];
    prefixed.extend(out);
    Ok(prefixed)
}

fn encode_base64url(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;
        out.push(BASE64URL_ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(BASE64URL_ALPHABET[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(BASE64URL_ALPHABET[((n >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(BASE64URL_ALPHABET[(n & 63) as usize] as char);
        }
    }
    out
}

fn decode_base64url(input: &str) -> Result<Vec<u8>, JsValue> {
    let clean: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace() && *b != b'=').collect();
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for b in clean {
        let value = BASE64URL_ALPHABET
            .iter()
            .position(|candidate| *candidate == b)
            .or_else(|| if b == b'+' { Some(62) } else if b == b'/' { Some(63) } else { None })
            .ok_or_else(|| err("Base64url input contains an invalid character"))? as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

fn crc32_bytes(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn morse_for(c: char) -> Option<&'static str> {
    Some(match c {
        'A' => ".-", 'B' => "-...", 'C' => "-.-.", 'D' => "-..", 'E' => ".", 'F' => "..-.",
        'G' => "--.", 'H' => "....", 'I' => "..", 'J' => ".---", 'K' => "-.-", 'L' => ".-..",
        'M' => "--", 'N' => "-.", 'O' => "---", 'P' => ".--.", 'Q' => "--.-", 'R' => ".-.",
        'S' => "...", 'T' => "-", 'U' => "..-", 'V' => "...-", 'W' => ".--", 'X' => "-..-",
        'Y' => "-.--", 'Z' => "--..", '0' => "-----", '1' => ".----", '2' => "..---",
        '3' => "...--", '4' => "....-", '5' => ".....", '6' => "-....", '7' => "--...",
        '8' => "---..", '9' => "----.", _ => return None,
    })
}

fn char_for_morse(token: &str) -> Option<char> {
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .find(|c| morse_for(*c) == Some(token))
}

fn json_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn json_option(value: Option<&str>) -> String {
    value
        .map(|text| format!("\"{}\"", json_escape(text)))
        .unwrap_or_else(|| "null".to_string())
}

const DDD_CODES: &[Ddd] = &[
    Ddd { code: "11", state: "Sao Paulo", region: "Greater Sao Paulo, Jundiai and Braganca Paulista" },
    Ddd { code: "12", state: "Sao Paulo", region: "Vale do Paraiba and northern coast" },
    Ddd { code: "13", state: "Sao Paulo", region: "Baixada Santista and Vale do Ribeira" },
    Ddd { code: "14", state: "Sao Paulo", region: "Avare, Bauru, Botucatu, Jau, Lins, Marilia and Ourinhos" },
    Ddd { code: "15", state: "Sao Paulo", region: "Itapetininga, Itapeva, Sorocaba and Tatui" },
    Ddd { code: "16", state: "Sao Paulo", region: "Araraquara, Franca, Ribeirao Preto, Sao Carlos and region" },
    Ddd { code: "17", state: "Sao Paulo", region: "Barretos, Catanduva, Fernandopolis, Sao Jose do Rio Preto and region" },
    Ddd { code: "18", state: "Sao Paulo", region: "Aracatuba, Assis, Birigui and Presidente Prudente" },
    Ddd { code: "19", state: "Sao Paulo", region: "Campinas, Limeira, Piracicaba, Rio Claro and region" },
    Ddd { code: "21", state: "Rio de Janeiro", region: "Rio de Janeiro metro area and Teresopolis" },
    Ddd { code: "22", state: "Rio de Janeiro", region: "Cabo Frio, Campos, Itaperuna, Macae and Nova Friburgo" },
    Ddd { code: "24", state: "Rio de Janeiro", region: "Angra dos Reis, Petropolis, Volta Redonda and region" },
    Ddd { code: "27", state: "Espirito Santo", region: "Vitoria metro area, Colatina and Linhares" },
    Ddd { code: "28", state: "Espirito Santo", region: "Cachoeiro de Itapemirim, Castelo, Itapemirim and Marataizes" },
    Ddd { code: "31", state: "Minas Gerais", region: "Belo Horizonte metro area, Conselheiro Lafaiete, Ipatinga and Vicosa" },
    Ddd { code: "32", state: "Minas Gerais", region: "Barbacena, Juiz de Fora, Muriae, Sao Joao del-Rei and Uba" },
    Ddd { code: "33", state: "Minas Gerais", region: "Almenara, Caratinga, Governador Valadares, Manhuacu and Teofilo Otoni" },
    Ddd { code: "34", state: "Minas Gerais", region: "Araguari, Araxa, Patos de Minas, Uberlandia and Uberaba" },
    Ddd { code: "35", state: "Minas Gerais", region: "Alfenas, Guaxupe, Lavras, Pocos de Caldas, Pouso Alegre and Varginha" },
    Ddd { code: "37", state: "Minas Gerais", region: "Bom Despacho, Divinopolis, Formiga, Itauna and Para de Minas" },
    Ddd { code: "38", state: "Minas Gerais", region: "Curvelo, Diamantina, Montes Claros, Pirapora and Unai" },
    Ddd { code: "41", state: "Parana", region: "Curitiba metro area and Parana coast" },
    Ddd { code: "42", state: "Parana", region: "Ponta Grossa and Guarapuava" },
    Ddd { code: "43", state: "Parana", region: "Apucarana and Londrina" },
    Ddd { code: "44", state: "Parana", region: "Maringa, Campo Mourao and Umuarama" },
    Ddd { code: "45", state: "Parana", region: "Cascavel and Foz do Iguacu" },
    Ddd { code: "46", state: "Parana", region: "Francisco Beltrao and Pato Branco" },
    Ddd { code: "47", state: "Santa Catarina", region: "Blumenau, Itajai, Joinville, Brusque and region" },
    Ddd { code: "48", state: "Santa Catarina", region: "Florianopolis metro area, Nova Trento, Sao Joao Batista and Criciuma" },
    Ddd { code: "49", state: "Santa Catarina", region: "Cacador, Chapeco, Concordia and Lages" },
    Ddd { code: "51", state: "Rio Grande do Sul", region: "Porto Alegre metro area, Santa Cruz do Sul and northern coast" },
    Ddd { code: "53", state: "Rio Grande do Sul", region: "Pelotas and Rio Grande" },
    Ddd { code: "54", state: "Rio Grande do Sul", region: "Caxias do Sul and Passo Fundo" },
    Ddd { code: "55", state: "Rio Grande do Sul", region: "Santa Maria, Santana do Livramento, Santo Angelo and Uruguaiana" },
    Ddd { code: "61", state: "Distrito Federal / Goias", region: "Distrito Federal and some surrounding Goias municipalities" },
    Ddd { code: "62", state: "Goias", region: "Goiania metro area, Anapolis, Niquelandia and Porangatu" },
    Ddd { code: "63", state: "Tocantins", region: "Statewide" },
    Ddd { code: "64", state: "Goias", region: "Caldas Novas, Catalao, Itumbiara and Rio Verde" },
    Ddd { code: "65", state: "Mato Grosso", region: "Cuiaba metro area" },
    Ddd { code: "66", state: "Mato Grosso", region: "Rondonopolis and Sinop" },
    Ddd { code: "67", state: "Mato Grosso do Sul", region: "Statewide" },
    Ddd { code: "68", state: "Acre", region: "Statewide" },
    Ddd { code: "69", state: "Rondonia", region: "Statewide" },
    Ddd { code: "71", state: "Bahia", region: "Salvador metro area" },
    Ddd { code: "73", state: "Bahia", region: "Eunapolis, Ilheus, Itabuna, Porto Seguro and Teixeira de Freitas" },
    Ddd { code: "74", state: "Bahia", region: "Irece, Jacobina, Juazeiro and Xique-Xique" },
    Ddd { code: "75", state: "Bahia", region: "Alagoinhas, Feira de Santana, Paulo Afonso and Valenca" },
    Ddd { code: "77", state: "Bahia", region: "Barreiras, Bom Jesus da Lapa, Guanambi and Vitoria da Conquista" },
    Ddd { code: "79", state: "Sergipe", region: "Statewide" },
    Ddd { code: "81", state: "Pernambuco", region: "Recife metro area and Caruaru" },
    Ddd { code: "82", state: "Alagoas", region: "Statewide" },
    Ddd { code: "83", state: "Paraiba", region: "Statewide" },
    Ddd { code: "84", state: "Rio Grande do Norte", region: "Statewide" },
    Ddd { code: "85", state: "Ceara", region: "Fortaleza metro area" },
    Ddd { code: "86", state: "Piaui", region: "Teresina and Greater Teresina region" },
    Ddd { code: "87", state: "Pernambuco", region: "Garanhuns, Petrolina, Salgueiro, Serra Talhada and region" },
    Ddd { code: "88", state: "Ceara", region: "Juazeiro do Norte and Sobral" },
    Ddd { code: "89", state: "Piaui", region: "Picos and Floriano" },
    Ddd { code: "91", state: "Para", region: "Belem metro area" },
    Ddd { code: "92", state: "Amazonas", region: "Manaus metro area and Parintins" },
    Ddd { code: "93", state: "Para", region: "Santarem, Altamira and Itaituba" },
    Ddd { code: "94", state: "Para", region: "Maraba" },
    Ddd { code: "95", state: "Roraima", region: "Statewide" },
    Ddd { code: "96", state: "Amapa", region: "Statewide" },
    Ddd { code: "97", state: "Amazonas", region: "Interior of Amazonas" },
    Ddd { code: "98", state: "Maranhao", region: "Sao Luis metro area" },
    Ddd { code: "99", state: "Maranhao", region: "Caxias, Codo and Imperatriz" },
];

const COUNTRY_CODES: &[Country] = &[
    Country { iso: "us", name: "United States", code: "1", note: "" },
    Country { iso: "ca", name: "Canada", code: "1", note: "" },
    Country { iso: "br", name: "Brazil", code: "55", note: "" },
    Country { iso: "mx", name: "Mexico", code: "52", note: "" },
    Country { iso: "ar", name: "Argentina", code: "54", note: "" },
    Country { iso: "cl", name: "Chile", code: "56", note: "" },
    Country { iso: "co", name: "Colombia", code: "57", note: "" },
    Country { iso: "pe", name: "Peru", code: "51", note: "" },
    Country { iso: "uy", name: "Uruguay", code: "598", note: "" },
    Country { iso: "py", name: "Paraguay", code: "595", note: "" },
    Country { iso: "bo", name: "Bolivia", code: "591", note: "" },
    Country { iso: "ve", name: "Venezuela", code: "58", note: "" },
    Country { iso: "gb", name: "United Kingdom", code: "44", note: "" },
    Country { iso: "pt", name: "Portugal", code: "351", note: "" },
    Country { iso: "es", name: "Spain", code: "34", note: "" },
    Country { iso: "fr", name: "France", code: "33", note: "" },
    Country { iso: "de", name: "Germany", code: "49", note: "" },
    Country { iso: "it", name: "Italy", code: "39", note: "" },
    Country { iso: "ru", name: "Russia", code: "7", note: "" },
    Country { iso: "cn", name: "China", code: "86", note: "" },
    Country { iso: "jp", name: "Japan", code: "81", note: "" },
    Country { iso: "kr", name: "South Korea", code: "82", note: "" },
    Country { iso: "in", name: "India", code: "91", note: "" },
    Country { iso: "au", name: "Australia", code: "61", note: "" },
    Country { iso: "nz", name: "New Zealand", code: "64", note: "" },
    Country { iso: "za", name: "South Africa", code: "27", note: "" },
    Country { iso: "ng", name: "Nigeria", code: "234", note: "" },
    Country { iso: "eg", name: "Egypt", code: "20", note: "" },
    Country { iso: "ma", name: "Morocco", code: "212", note: "" },
    Country { iso: "ao", name: "Angola", code: "244", note: "" },
    Country { iso: "mz", name: "Mozambique", code: "258", note: "" },
    Country { iso: "cv", name: "Cape Verde", code: "238", note: "" },
    Country { iso: "sg", name: "Singapore", code: "65", note: "" },
    Country { iso: "id", name: "Indonesia", code: "62", note: "" },
    Country { iso: "my", name: "Malaysia", code: "60", note: "" },
    Country { iso: "ph", name: "Philippines", code: "63", note: "" },
    Country { iso: "th", name: "Thailand", code: "66", note: "" },
    Country { iso: "vn", name: "Vietnam", code: "84", note: "" },
    Country { iso: "tr", name: "Turkey", code: "90", note: "" },
    Country { iso: "sa", name: "Saudi Arabia", code: "966", note: "" },
    Country { iso: "ae", name: "United Arab Emirates", code: "971", note: "" },
    Country { iso: "il", name: "Israel", code: "972", note: "" },
    Country { iso: "ir", name: "Iran", code: "98", note: "" },
    Country { iso: "pk", name: "Pakistan", code: "92", note: "" },
    Country { iso: "bd", name: "Bangladesh", code: "880", note: "" },
    Country { iso: "lk", name: "Sri Lanka", code: "94", note: "" },
    Country { iso: "np", name: "Nepal", code: "977", note: "" },
    Country { iso: "ch", name: "Switzerland", code: "41", note: "" },
    Country { iso: "at", name: "Austria", code: "43", note: "" },
    Country { iso: "nl", name: "Netherlands", code: "31", note: "" },
    Country { iso: "be", name: "Belgium", code: "32", note: "" },
    Country { iso: "se", name: "Sweden", code: "46", note: "" },
    Country { iso: "no", name: "Norway", code: "47", note: "" },
    Country { iso: "dk", name: "Denmark", code: "45", note: "" },
    Country { iso: "fi", name: "Finland", code: "358", note: "" },
    Country { iso: "pl", name: "Poland", code: "48", note: "" },
    Country { iso: "cz", name: "Czech Republic", code: "420", note: "" },
    Country { iso: "gr", name: "Greece", code: "30", note: "" },
    Country { iso: "ie", name: "Ireland", code: "353", note: "" },
    Country { iso: "ro", name: "Romania", code: "40", note: "" },
    Country { iso: "ua", name: "Ukraine", code: "380", note: "" },
    Country { iso: "cu", name: "Cuba", code: "53", note: "" },
    Country { iso: "do", name: "Dominican Republic", code: "1809", note: "NANP also uses +1 829 and +1 849" },
    Country { iso: "jm", name: "Jamaica", code: "1876", note: "" },
    Country { iso: "pr", name: "Puerto Rico", code: "1787", note: "NANP also uses +1 939" },
    Country { iso: "tt", name: "Trinidad and Tobago", code: "1868", note: "" },
    Country { iso: "bs", name: "Bahamas", code: "1242", note: "" },
    Country { iso: "bb", name: "Barbados", code: "1246", note: "" },
    Country { iso: "cr", name: "Costa Rica", code: "506", note: "" },
    Country { iso: "pa", name: "Panama", code: "507", note: "" },
    Country { iso: "ec", name: "Ecuador", code: "593", note: "" },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ulid_round_trip() {
        let random = [0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x10, 0x11];
        let id = ulid(1_500_000_000_000.0, &random).unwrap();
        let parts = inspect_ulid(&id).unwrap();
        assert!(parts.contains("\"timestamp_ms\":1500000000000"));
        assert!(parts.contains("\"random\":\"1234567890ABCDEF1011\""));
    }

    #[test]
    fn ksuid_matches_reference_vector() {
        let raw = decode_base62_20("0ujtsYcgvSTl8PAuAdqWYSMnLOv").unwrap();
        assert_eq!(hex(&raw), "0669F7EFB5A1CD34B5F99D1154FB6853345C9735");
        assert_eq!(encode_base62(&raw, 27), "0ujtsYcgvSTl8PAuAdqWYSMnLOv");
    }

    #[test]
    fn classic_ciphers_are_stable() {
        assert_eq!(rot13("Hello"), "Uryyb");
        assert_eq!(caesar("Abc Xyz", 2), "Cde Zab");
        assert_eq!(vigenere("ATTACKATDAWN", "LEMON", false).unwrap(), "LXFOPVEFRNHR");
        assert_eq!(vigenere("LXFOPVEFRNHR", "LEMON", true).unwrap(), "ATTACKATDAWN");
    }

    #[test]
    fn uuid_v7_sets_layout_bits() {
        let random = [0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x10, 0x11];
        let id = uuid_v7(1_500_000_000_000.0, &random).unwrap();
        let bytes = decode_uuid(&id).unwrap();
        assert_eq!(bytes[6] >> 4, 7);
        assert_eq!(bytes[8] >> 6, 2);
        let parts = inspect_uuid_v7(&id).unwrap();
        assert!(parts.contains("\"timestamp_ms\":1500000000000"));
    }

    #[test]
    fn qr_matrix_returns_modules() {
        let json = qr_matrix("https://dyammarcano.github.io", "Q").unwrap();
        assert!(json.contains("\"size\":"));
        assert!(json.contains("\"modules\":\""));
    }

    #[test]
    fn phone_lookup_finds_brazil_ddd_and_country() {
        let json = phone_lookup("+55 11 99999-0000");
        assert!(json.contains("DDD 11 - Sao Paulo"));
        assert!(json.contains("Brazil +55"));
        assert!(json.contains("\"ddd_count\":67"));
    }

    #[test]
    fn phone_lookup_keeps_foreign_international_numbers_out_of_brazil_rules() {
        let json = phone_lookup("+91 87308 44504");
        assert!(json.contains("India +91"));
        assert!(!json.contains("DDD 91"));
        assert!(json.contains("\"formatted_query\":null"));
        assert!(json.contains("\"e164\":null"));
    }

    #[test]
    fn password_policy_uses_random_bytes() {
        let random = [42u8; 1024];
        let password = password_generate(12, 2, true, true, true, false, false, false, true, &random).unwrap();
        let lines: Vec<_> = password.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line.len() == 12));
    }

    #[test]
    fn rust_encoding_hash_and_cipher_tools_work() {
        assert_eq!(hex_encode("hi"), "6869");
        assert_eq!(hex_decode("6869").unwrap(), "hi");
        assert_eq!(base32_decode(&base32_encode("hello")).unwrap(), "hello");
        assert_eq!(base58_decode(&base58_encode("hello")).unwrap(), "hello");
        assert_eq!(base64url_decode(&base64url_encode("hello")).unwrap(), "hello");
        assert_eq!(crc32("123456789"), "cbf43926");
        assert_eq!(fnv1a32("hello"), "4f9f2cab");
        assert_eq!(morse_decode(&morse_encode("SOS")), "SOS");
        assert_eq!(leet_decode(&leet_encode("test")), "test");
        assert_eq!(atbash("Abc Z"), "Zyx A");
        assert_eq!(polybius_decode(&polybius_encode("ABC")), "ABC");
    }
}
