use crate::error::Error;
use crate::Result;
use dotenvy::dotenv;
use ed25519_dalek::{Signer as _, SigningKey, SECRET_KEY_LENGTH};
use reqwest::header::{HeaderMap, HeaderValue};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

pub const HEADER_API_KEY: &str = "X-Api-Key";
pub const HEADER_REQUEST_SIGN: &str = "X-Request-Sign";
pub const HEADER_SIGN_DATE: &str = "X-Sign-Date";

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

    pub fn generate_headers(&self, method: &str, url: &Url, body: &str) -> Result<HeaderMap> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let path_and_query = url.path().to_string() + url.query().unwrap_or_default();

        // Step 1: Build non-signed string
        let unsigned_string = format!("{method}{path_and_query}{body}{timestamp}");

        // Step 2: Sign the string
        let signature = self.signing_key.sign(unsigned_string.as_bytes());

        // Step 3: Encode the result string with hex
        let signature_hex = hex::encode(signature.to_bytes());

        // Step 4: Prepare headers
        let mut headers = HeaderMap::new();
        headers.insert(
            HEADER_API_KEY,
            HeaderValue::from_str(&self.api_key)
                .map_err(|e| Error::InvalidHeader(e.to_string()))?,
        );
        headers.insert(
            HEADER_REQUEST_SIGN,
            HeaderValue::from_str(&signature_hex)
                .map_err(|e| Error::InvalidHeader(e.to_string()))?,
        );
        headers.insert(HEADER_SIGN_DATE, HeaderValue::from(timestamp));

        Ok(headers)
    }
}

fn create_signing_key(secret_key: &str) -> Result<SigningKey> {
    let secret_key_bytes = hex::decode(secret_key)?;
    if secret_key_bytes.len() != SECRET_KEY_LENGTH {
        return Err(Error::SigningKey("Invalid secret key length".into()));
    }
    let key_array: [u8; SECRET_KEY_LENGTH] = secret_key_bytes
        .try_into()
        .map_err(|_| Error::SigningKey("Failed to convert secret key to array".into()))?;

    Ok(SigningKey::from_bytes(&key_array))
}
