use crate::error::Error;
use crate::Result;
use chrono::Utc;
use ed25519_dalek::{SecretKey, Signer as _, SigningKey};
use reqwest::header::{HeaderMap, HeaderValue};
use std::env;
use url::Url;

pub const HEADER_API_KEY: &str = "X-Api-Key";
pub const HEADER_REQUEST_SIGN: &str = "X-Request-Sign";
pub const HEADER_SIGN_DATE: &str = "X-Sign-Date";
pub const SIGNATURE_PREFIX: &str = "dmar ed25519 ";

pub struct Signer {
    signing_key: SigningKey,
    api_key: String,
}

impl Signer {
    pub fn new() -> Result<Self> {
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
        let timestamp = Utc::now().timestamp().to_string();
        let path_and_query = url.path().to_string() + url.query().unwrap_or_default();

        // Step 1: Build non-signed string
        let unsigned_string = format!("{method}{path_and_query}{body}{timestamp}");

        // Step 2: Sign the string
        let signature = self.signing_key.sign(unsigned_string.as_bytes());

        // Step 3: Specify signature type and encode the signature with hex
        let signature_hex = SIGNATURE_PREFIX.to_string() + &hex::encode(signature.to_bytes());

        // Step 4: Prepare headers
        let mut headers = HeaderMap::new();
        headers.insert(HEADER_API_KEY, HeaderValue::from_str(&self.api_key)?);
        headers.insert(HEADER_REQUEST_SIGN, HeaderValue::from_str(&signature_hex)?);
        headers.insert(HEADER_SIGN_DATE, HeaderValue::from_str(&timestamp)?);

        Ok(headers)
    }
}

fn create_signing_key(secret_key: &str) -> Result<SigningKey> {
    let bytes: SecretKey = hex::decode(secret_key)?
        .try_into()
        .map_err(|_| Error::SigningKey("Couldn't turn secret key str into a SecretKey".into()))?;

    Ok(SigningKey::from_bytes(&bytes))
}
