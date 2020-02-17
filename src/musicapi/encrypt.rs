//
// encrypt.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
#[allow(unused)]
use super::model::*;
use crate::model::{Errors, NCMResult};
use num::bigint::BigUint;
use openssl::{
    hash::{hash, MessageDigest},
    symm::{encrypt, Cipher},
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Serialize;
use serde_urlencoded;

static MODULUS:&str = "00e0b509f6259df8642dbc35662901477df22677ec152b5ff68ace615bb7b725152b3ab17a876aea8a5aa76d2e417629ec4ee341f56135fccf695280104e0312ecbda92557c93870114af6c9d05c4f7f0c3685b7a46bee255932575cce10b424d813cfe4875d3e82047b97ddef52741d546b8e289dc6935b3ece0462db0a22b8e7";
static NONCE: &str = "0CoJUm6Qyw8W8jud";
static PUBKEY: &str = "010001";

pub(crate) struct Encrypt;

#[allow(unused)]
impl Encrypt {
    pub(crate) fn encrypt_id(id: String) -> NCMResult<String> {
        let magic = b"3go8&$8*3*3h0k(2)2";
        let magic_len = magic.len();
        let id = id;
        let mut song_id = id.clone().into_bytes();
        id.as_bytes().iter().enumerate().for_each(|(i, sid)| {
            song_id[i] = *sid ^ magic[i % magic_len];
        });
        let result = hash(MessageDigest::md5(), &song_id)?;
        Ok(base64::encode_config(&hex::encode(result), base64::URL_SAFE)
            .replace("/", "_")
            .replace("+", "-"))
    }

    pub(crate) fn encrypt_request(text: impl Serialize + std::fmt::Debug) -> NCMResult<String> {
        let data = serde_json::to_string(&text)?;
        let secret = Self.create_key(16);
        let params = Self.aes(Self.aes(data, NONCE)?, &secret)?;
        #[allow(non_snake_case)]
        let encSecKey = Self.rsa(secret)?;
        let meal = &[("params", params), ("encSecKey", encSecKey)];
        Ok(serde_urlencoded::to_string(&meal)?)
    }

    fn aes(&self, text: String, key: &str) -> NCMResult<String> {
        let pad = 16 - text.len() % 16;
        let p = pad as u8 as char;
        let mut text = text;
        for _ in 0..pad {
            text.push(p);
        }
        let text = text.as_bytes();
        let cipher = Cipher::aes_128_cbc();
        let ciphertext = encrypt(cipher, key.as_bytes(), Some(b"0102030405060708"), text)?;
        Ok(base64::encode(&ciphertext))
    }

    fn rsa(&self, text: String) -> NCMResult<String> {
        let text = text.chars().rev().collect::<String>();
        let text = BigUint::parse_bytes(hex::encode(text).as_bytes(), 16).ok_or(Errors::NoneError)?;
        let pubkey = BigUint::parse_bytes(PUBKEY.as_bytes(), 16).ok_or(Errors::NoneError)?;
        let modulus = BigUint::parse_bytes(MODULUS.as_bytes(), 16).ok_or(Errors::NoneError)?;
        let pow = text.modpow(&pubkey, &modulus);
        Ok(pow.to_str_radix(16))
    }

    fn create_key(&self, len: usize) -> String {
        return hex::encode(thread_rng().sample_iter(&Alphanumeric).take(len).collect::<String>())[..16].to_string();
    }
}
