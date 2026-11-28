use base64::{
    Engine as _, engine::general_purpose::STANDARD, engine::general_purpose::URL_SAFE_NO_PAD,
};
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use friendarazzi_core::ports::OidcClientError;
use ring::{
    rand::SystemRandom,
    signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair},
};
use serde::Serialize;

#[derive(Serialize)]
struct AppleJwtHeader<'a> {
    alg: &'static str,
    kid: &'a str,
    typ: &'static str,
}

#[derive(Serialize)]
struct AppleJwtClaims<'a> {
    iss: &'a str,
    iat: i64,
    exp: i64,
    aud: &'a str,
    sub: &'a str,
}

/// Build an ES256 client-secret JWT required by Apple's OAuth token endpoint.
pub(crate) fn build_apple_client_secret(
    team_id: &str,
    client_id: &str,
    key_id: &str,
    private_key_pem: &str,
    audience: &str,
    now: DateTime<Utc>,
) -> Result<String, Report<OidcClientError>> {
    let der = decode_pkcs8_pem(private_key_pem)?;
    let rng = SystemRandom::new();
    let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &der, &rng)
        .map_err(|_| Report::new(OidcClientError::CryptoError))?;

    let iat = now.timestamp();
    let header_json = serde_json::to_vec(&AppleJwtHeader {
        alg: "ES256",
        kid: key_id,
        typ: "JWT",
    })
    .change_context(OidcClientError::CryptoError)?;
    let claims_json = serde_json::to_vec(&AppleJwtClaims {
        iss: team_id,
        iat,
        exp: iat + 300,
        aud: audience,
        sub: client_id,
    })
    .change_context(OidcClientError::CryptoError)?;

    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header_json),
        URL_SAFE_NO_PAD.encode(claims_json)
    );
    let sig = key_pair
        .sign(&rng, signing_input.as_bytes())
        .map_err(|_| Report::new(OidcClientError::CryptoError))?;
    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(sig.as_ref())
    ))
}

fn decode_pkcs8_pem(pem: &str) -> Result<Vec<u8>, Report<OidcClientError>> {
    let b64: String = pem
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("-----"))
        .collect();
    if b64.is_empty() {
        return Err(Report::new(OidcClientError::CryptoError));
    }
    STANDARD
        .decode(b64)
        .map_err(|_| Report::new(OidcClientError::CryptoError))
}

#[cfg(test)]
mod tests {
    use ring::{
        pkcs8::Document,
        signature::{ECDSA_P256_SHA256_FIXED, KeyPair, UnparsedPublicKey},
    };

    use super::*;

    fn test_p256_pem(doc: &Document) -> String {
        let body = STANDARD.encode(doc.as_ref());
        format!("-----BEGIN PRIVATE KEY-----\n{body}\n-----END PRIVATE KEY-----\n")
    }

    #[test_log::test]
    fn apple_client_secret_signs_and_verifies_with_es256() {
        let rng = SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng)
                .unwrap();
        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

        let jwt = build_apple_client_secret(
            "TEAM123",
            "app.friendarazzi.web",
            "KEY456",
            &test_p256_pem(&pkcs8),
            "https://appleid.apple.com",
            now,
        )
        .unwrap();

        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let sig = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        let pub_key = UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, key_pair.public_key());
        pub_key.verify(signing_input.as_bytes(), &sig).unwrap();

        let header: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
        assert_eq!(header["alg"], "ES256");
        assert_eq!(header["kid"], "KEY456");

        let claims: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        assert_eq!(claims["iss"], "TEAM123");
        assert_eq!(claims["sub"], "app.friendarazzi.web");
        assert_eq!(claims["aud"], "https://appleid.apple.com");
    }
}
