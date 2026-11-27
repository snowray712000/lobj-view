//! 資料夾名稱簡單遮蔽（可還原），用來把病患姓名之類的資料夾名換成 a-z2-7。
//! 與 `no_user_info.py` 的演算法完全相容（同密碼互相可還原）：
//! 金鑰 = PBKDF2-HMAC-SHA256(密碼, "no_user_info", 100000 次, 32 bytes)；
//! 名稱以 UTF-16BE 編碼；驗證碼 iv = HMAC(金鑰, 原文)[..3]；
//! 密文 = 原文 XOR HMAC(金鑰, iv ‖ 計數器(4 bytes, 大端)) 串流；輸出 = base32(iv ‖ 密文) 小寫、去掉 `=`。
//! 這只是遮蔽，不是強加密。

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const IV_LEN: usize = 3;
const B32: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

fn hmac(key: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut m = HmacSha256::new_from_slice(key).expect("HMAC 接受任意長度金鑰");
    for p in parts {
        m.update(p);
    }
    m.finalize().into_bytes().into()
}

/// 從密碼導出金鑰（PBKDF2-HMAC-SHA256，輸出剛好一個區塊 32 bytes）。
pub fn derive_key(password: &str) -> [u8; 32] {
    let pw = password.as_bytes();
    let mut u = hmac(pw, &[b"no_user_info", &1u32.to_be_bytes()]);
    let mut out = u;
    for _ in 1..100_000 {
        u = hmac(pw, &[&u]);
        for (o, x) in out.iter_mut().zip(u) {
            *o ^= x;
        }
    }
    out
}

fn xor_stream(key: &[u8; 32], iv: &[u8], data: &[u8]) -> Vec<u8> {
    let mut ks = Vec::with_capacity(data.len() + 32);
    let mut i = 0u32;
    while ks.len() < data.len() {
        ks.extend_from_slice(&hmac(key, &[iv, &i.to_be_bytes()]));
        i += 1;
    }
    data.iter().zip(ks).map(|(a, b)| a ^ b).collect()
}

fn b32_encode(data: &[u8]) -> String {
    let (mut out, mut buf, mut bits) = (String::new(), 0u32, 0u32);
    for &b in data {
        buf = (buf << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(B32[((buf >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(B32[((buf << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn b32_decode(s: &str) -> Option<Vec<u8>> {
    let (mut out, mut buf, mut bits) = (Vec::new(), 0u32, 0u32);
    for c in s.to_ascii_lowercase().bytes() {
        let v = B32.iter().position(|&x| x == c)? as u32;
        buf = (buf << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

pub fn encode(name: &str, key: &[u8; 32]) -> String {
    let raw: Vec<u8> = name.encode_utf16().flat_map(u16::to_be_bytes).collect();
    let iv = &hmac(key, &[&raw])[..IV_LEN];
    let mut data = iv.to_vec();
    data.extend(xor_stream(key, iv, &raw));
    b32_encode(&data)
}

pub fn decode(s: &str, key: &[u8; 32]) -> Result<String, String> {
    let data = b32_decode(s).filter(|d| d.len() > IV_LEN).ok_or("名稱不是遮蔽產生的")?;
    let (iv, ct) = data.split_at(IV_LEN);
    let raw = xor_stream(key, iv, ct);
    if hmac(key, &[&raw])[..IV_LEN] != *iv {
        return Err("密碼錯誤或名稱不是遮蔽產生的".into());
    }
    if raw.len() % 2 != 0 {
        return Err("名稱不是遮蔽產生的".into());
    }
    let units: Vec<u16> = raw.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
    String::from_utf16(&units).map_err(|_| "名稱不是遮蔽產生的".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 向量由 no_user_info.py（密碼 cadcam）產生，確保兩邊相容
    const VECTORS: [(&str, &str); 4] = [
        ("王小明", "zahg7gro5qep7ca"),
        ("abc", "fnckfoogfrkfsdq"),
        ("ACom_230914_121", "hdkxp4hpmcd2tipbdstsbhqcoqvf2sf3n6t3v243ezjic4ef6pihw"),
        ("李 Bob", "brx5zt2j7hm735ym2skwo"),
    ];

    #[test]
    fn compatible_with_python() {
        let key = derive_key("cadcam");
        for (plain, enc) in VECTORS {
            assert_eq!(encode(plain, &key), enc);
            assert_eq!(decode(enc, &key).unwrap(), plain);
        }
    }

    #[test]
    fn wrong_password_rejected() {
        let enc = encode("王小明", &derive_key("cadcam"));
        assert!(decode(&enc, &derive_key("other")).is_err());
        assert!(decode("not*base32", &derive_key("cadcam")).is_err());
    }
}
