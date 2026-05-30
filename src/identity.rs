use ed25519_dalek::{SigningKey, Signature, Signer};
use rand::rngs::OsRng;
use std::{fs, path::PathBuf};
use hex;

pub struct Identity {
    pub public_key: String,
    pub private_key: String, // Stored as 64-character hex string (32-byte seed + 32-byte public key)
}

impl Identity {
 pub fn sign(&self, message: &[u8]) -> Vec<u8> {
    let bytes = hex::decode(&self.private_key).expect("Invalid hex private key");

    // Convert the 32-byte seed slice into a fixed-size array
    let seed: &[u8; 32] = bytes[..32].try_into().expect("Invalid key length");

    // In ed25519-dalek 2.x, we use SigningKey::from_bytes
    // Note: The crate interprets 32-byte arrays as the secret key/seed
    let signing_key = SigningKey::from_bytes(seed);

    let signature: Signature = signing_key.sign(message);
    signature.to_vec()
}

    pub fn public_key_hex(&self) -> String {
        self.public_key.clone()
    }

    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        Self {
            // verifying_key() produces the 32-byte public key
            public_key: hex::encode(signing_key.verifying_key().to_bytes()),
            // to_bytes() produces 64 bytes (seed + public key)
            private_key: hex::encode(signing_key.to_bytes()),
        }
    }

    pub fn load_or_create(force: bool) -> Self {
        let path = Self::path();
        if path.exists() && !force { return Self::load(); }
        let identity = Self::generate();
        identity.save();
        identity
    }

    fn save(&self) {
        let path = Self::path();
        fs::create_dir_all(path.parent().unwrap()).expect("Failed to create config dir");
        let data = serde_json::json!({
            "public_key": self.public_key,
            "private_key": self.private_key
        });
        fs::write(&path, data.to_string()).expect("Failed to save identity");
    }

    fn load() -> Self {
        let data = fs::read_to_string(Self::path()).expect("Failed to read identity");
        let json: serde_json::Value = serde_json::from_str(&data).expect("Malformed identity file");
        Self {
            public_key: json["public_key"].as_str().unwrap().to_string(),
            private_key: json["private_key"].as_str().unwrap().to_string(),
        }
    }

    fn path() -> PathBuf {
        dirs::home_dir().expect("Home dir not found").join(".deping/id.json")
    }
}
