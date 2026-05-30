use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use bs58;
use std::{fs, path::PathBuf};

pub struct Identity {
    pub public_key: String,
    pub private_key: String,
}

impl Identity {
    pub fn generate() -> Self {
        let mut csprng = OsRng;

        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        let public_key = bs58::encode(verifying_key.to_bytes()).into_string();
        let private_key = bs58::encode(signing_key.to_bytes()).into_string();

        Self {
            public_key,
            private_key,
        }
    }

    pub fn load_or_create(force: bool) -> Self {
        let path = Self::path();

        if path.exists() && !force {
            return Self::load();
        }

        let identity = Self::generate();
        identity.save();
        identity
    }

    fn save(&self) {
        let path = Self::path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();

        let data = serde_json::json!({
            "public_key": self.public_key,
            "private_key": self.private_key
        });

        fs::write(&path, data.to_string()).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).ok();
        }
    }

    fn load() -> Self {
        let path = Self::path();
        let data = fs::read_to_string(path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&data).unwrap();

        Self {
            public_key: json["public_key"].as_str().unwrap().to_string(),
            private_key: json["private_key"].as_str().unwrap().to_string(),
        }
    }

    fn path() -> PathBuf {
        dirs::home_dir().unwrap().join(".deping/id.json")
    }
}
