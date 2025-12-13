use sha1::{Sha1, Digest};
use bitflags::bitflags;

pub enum AuthPlugin {
    NativePassword,
    CachingSha2Password,
    ClearPassword,
}

pub struct Authenticator {
    plugin: AuthPlugin,
}

impl Authenticator {
    pub fn new(plugin: AuthPlugin) -> Self {
        Self { plugin }
    }

    /// Implement mysql_native_password check
    /// challenge = SHA1(SHA1(password)) XOR SHA1(salt + SHA1(SHA1(password)))
    /// client_response = SHA1(password) XOR SHA1(salt + SHA1(SHA1(password)))
    /// EQUATION:
    /// server_side: hash1 = SHA1(password) -> stored in DB
    ///              hash2 = SHA1(hash1)    -> stored in DB (in mysql.user)
    /// client_sends = hash1 XOR SHA1(salt + hash2)
    /// verification = SHA1(salt + hash2) XOR client_sends == hash1
    /// Check if SHA1(verification) == hash2
    pub fn verify_native_password(&self, response: &[u8], salt: &[u8], stored_hash_sha1_sha1: &[u8]) -> bool {
         if response.len() != 20 { return false; }
         
         // In real DB we have stored_hash_sha1_sha1 (double hash).
         // We compute: token = SHA1(salt + stored_hash_sha1_sha1)
         // client_response = hash1 XOR token
         // therefore hash1 = client_response XOR token
         // then we check SHA1(hash1) == stored_hash_sha1_sha1

         let mut hasher = Sha1::new();
         hasher.update(salt);
         hasher.update(stored_hash_sha1_sha1);
         let token = hasher.finalize();

         let mut hash1 = [0u8; 20];
         for i in 0..20 {
             hash1[i] = response[i] ^ token[i];
         }

         let mut hasher2 = Sha1::new();
         hasher2.update(hash1);
         let check = hasher2.finalize();

         check.as_slice() == stored_hash_sha1_sha1
    }
}
