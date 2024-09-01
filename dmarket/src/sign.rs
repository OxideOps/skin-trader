use crate::error::Error;
use crate::Result;
use dotenvy::dotenv;
use ed25519_dalek::{Signer as _, SigningKey, SECRET_KEY_LENGTH};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

pub struct Signer {
    signing_key: SigningKey,
    api_key: String,
}

impl Signer {
    pub fn new() -> Result<Self> {
        dotenv().ok();

        let secret_key = env::var("DMARKET_SECRET_KEY")
            .map_err(|_| Error::Config("DMARKET_SECRET_KEY not found in environment".into()))?;

        let api_key = env::var("DMARKET_API_KEY")
            .map_err(|_| Error::Config("DMARKET_API_KEY not found in environment".into()))?;

        let signing_key = create_signing_key(&secret_key)?;

        Ok(Self {
            signing_key,
            api_key,
        })
    }

    pub fn generate_headers(
        &self,
        method: &str,
        url: &str,
        body: &str,
    ) -> Result<Vec<(String, String)>> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let parsed_url = Url::parse(url)?;
        let path_and_query = parsed_url.path().to_string() + parsed_url.query().unwrap_or("");

        // Step 1: Build non-signed string
        let unsigned_string = format!("{}{}{}{}", method, path_and_query, body, timestamp);

        // Step 2: Sign the string
        let signature = self.signing_key.sign(unsigned_string.as_bytes());

        // Step 3: Encode the result string with hex
        let signature_hex = hex::encode(signature.to_bytes());

        // Step 4: Prepare headers
        Ok(vec![
            ("X-Api-Key".to_string(), self.api_key.clone()),
            ("X-Request-Sign".to_string(), signature_hex),
            ("X-Sign-Date".to_string(), timestamp.to_string()),
        ])
    }
}

fn create_signing_key(secret_key: &str) -> Result<SigningKey> {
    let secret_key_bytes = hex::decode(secret_key)?;
    if secret_key_bytes.len() != SECRET_KEY_LENGTH {
        return Err(Error::SigningKey("Invalid secret key length".into()));
    }
    let key_array: [u8; 32] = secret_key_bytes
        .try_into()
        .map_err(|_| Error::SigningKey("Failed to convert secret key to array".into()))?;

    Ok(SigningKey::from_bytes(&key_array))
}
