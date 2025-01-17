//! Bindings and helpers for accessing libsodium functions

use anyhow::{ensure, Result};
use libsodium_sys as libsodium;
use rosenpass_constant_time::xor_into;
use rosenpass_util::attempt;
use static_assertions::const_assert_eq;
use std::os::raw::c_ulonglong;
use std::ptr::null as nullptr;

pub const NONCE0: [u8; libsodium::crypto_aead_chacha20poly1305_IETF_NPUBBYTES as usize] =
    [0u8; libsodium::crypto_aead_chacha20poly1305_IETF_NPUBBYTES as usize];
pub const NOTHING: [u8; 0] = [0u8; 0];
pub const KEY_SIZE: usize = 32;
pub const MAC_SIZE: usize = 16;

const_assert_eq!(
    KEY_SIZE,
    libsodium::crypto_aead_chacha20poly1305_IETF_KEYBYTES as usize
);
const_assert_eq!(KEY_SIZE, libsodium::crypto_generichash_BYTES as usize);

macro_rules! sodium_call {
    ($name:ident, $($args:expr),*) => { attempt!({
        ensure!(unsafe{libsodium::$name($($args),*)} > -1,
            "Error in libsodium's {}.", stringify!($name));
        Ok(())
    })};
    ($name:ident) => { sodium_call!($name, ) };
}

#[inline]
fn blake2b_flexible(out: &mut [u8], key: &[u8], data: &[u8]) -> Result<()> {
    const KEY_MIN: usize = libsodium::crypto_generichash_KEYBYTES_MIN as usize;
    const KEY_MAX: usize = libsodium::crypto_generichash_KEYBYTES_MAX as usize;
    const OUT_MIN: usize = libsodium::crypto_generichash_BYTES_MIN as usize;
    const OUT_MAX: usize = libsodium::crypto_generichash_BYTES_MAX as usize;
    assert!(key.is_empty() || (KEY_MIN <= key.len() && key.len() <= KEY_MAX));
    assert!(OUT_MIN <= out.len() && out.len() <= OUT_MAX);
    let kptr = match key.len() {
        // NULL key
        0 => nullptr(),
        _ => key.as_ptr(),
    };
    sodium_call!(
        crypto_generichash_blake2b,
        out.as_mut_ptr(),
        out.len(),
        data.as_ptr(),
        data.len() as c_ulonglong,
        kptr,
        key.len()
    )
}

// TODO: Use proper streaming hash; for mix_hash too.
#[inline]
pub fn hash_into(out: &mut [u8], data: &[u8]) -> Result<()> {
    assert!(out.len() == KEY_SIZE);
    blake2b_flexible(out, &NOTHING, data)
}

#[inline]
pub fn hash(data: &[u8]) -> Result<[u8; KEY_SIZE]> {
    let mut r = [0u8; KEY_SIZE];
    hash_into(&mut r, data)?;
    Ok(r)
}

#[inline]
pub fn mac_into(out: &mut [u8], key: &[u8], data: &[u8]) -> Result<()> {
    assert!(out.len() == KEY_SIZE);
    assert!(key.len() == KEY_SIZE);
    blake2b_flexible(out, key, data)
}

#[inline]
pub fn mac(key: &[u8], data: &[u8]) -> Result<[u8; KEY_SIZE]> {
    let mut r = [0u8; KEY_SIZE];
    mac_into(&mut r, key, data)?;
    Ok(r)
}

#[inline]
pub fn mac16(key: &[u8], data: &[u8]) -> Result<[u8; 16]> {
    assert!(key.len() == KEY_SIZE);
    let mut out = [0u8; 16];
    blake2b_flexible(&mut out, key, data)?;
    Ok(out)
}

#[inline]
pub fn hmac_into(out: &mut [u8], key: &[u8], data: &[u8]) -> Result<()> {
    // Not bothering with padding; the implementation
    // uses appropriately sized keys.
    ensure!(key.len() == KEY_SIZE);

    const IPAD: [u8; KEY_SIZE] = [0x36u8; KEY_SIZE];
    let mut temp_key = [0u8; KEY_SIZE];
    temp_key.copy_from_slice(key);
    xor_into(&mut temp_key, &IPAD);
    let outer_data = mac(&temp_key, data)?;

    const OPAD: [u8; KEY_SIZE] = [0x5Cu8; KEY_SIZE];
    temp_key.copy_from_slice(key);
    xor_into(&mut temp_key, &OPAD);
    mac_into(out, &temp_key, &outer_data)
}

#[inline]
pub fn hmac(key: &[u8], data: &[u8]) -> Result<[u8; KEY_SIZE]> {
    let mut r = [0u8; KEY_SIZE];
    hmac_into(&mut r, key, data)?;
    Ok(r)
}
