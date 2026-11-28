use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use error_stack::Report;
use friendarazzi_core::domain::EmailAdresse;
use friendarazzi_core::domain::konto::{CLOCK_SKEW, OidcProviderId, VerifiedOidcIdentity};
use friendarazzi_core::ports::OidcClientError;
use ring::signature::{
    ECDSA_P256_SHA256_FIXED, RSA_PKCS1_2048_8192_SHA256, RsaPublicKeyComponents, UnparsedPublicKey,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct JwkSet {
    pub keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Jwk {
    pub kty: String,
    #[serde(default)]
    pub kid: Option<String>,
    #[serde(default)]
    pub alg: Option<String>,
    #[serde(default)]
    pub crv: Option<String>,
    #[serde(default)]
    pub n: Option<String>,
    #[serde(default)]
    pub e: Option<String>,
    #[serde(default)]
    pub x: Option<String>,
    #[serde(default)]
    pub y: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JwtHeader {
    alg: String,
    #[serde(default)]
    kid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IdTokenClaims {
    iss: String,
    sub: String,
    aud: AudienceClaim,
    exp: i64,
    #[serde(default)]
    nbf: Option<i64>,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    email_verified: Option<EmailVerifiedClaim>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AudienceClaim {
    Single(String),
    Multiple(Vec<String>),
}

impl AudienceClaim {
    fn contains(&self, client_id: &str) -> bool {
        match self {
            Self::Single(s) => s == client_id,
            Self::Multiple(list) => list.iter().any(|s| s == client_id),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EmailVerifiedClaim {
    Bool(bool),
    String(String),
}

impl EmailVerifiedClaim {
    fn is_verified(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::String(s) => s.eq_ignore_ascii_case("true"),
        }
    }
}

pub(crate) enum VerifyStatus {
    Verified(VerifiedOidcIdentity),
    UnknownKid,
}

pub(crate) fn verify_id_token(
    id_token: &str,
    jwks: &JwkSet,
    provider_id: &OidcProviderId,
    expected_issuer: &str,
    expected_client_id: &str,
    expected_nonce: &str,
    now: DateTime<Utc>,
) -> Result<VerifyStatus, Report<OidcClientError>> {
    let mut parts = id_token.split('.');
    let (Some(header_b64), Some(claims_b64), Some(sig_b64), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Report::new(OidcClientError::InvalidIdToken));
    };

    let header_bytes = URL_SAFE_NO_PAD
        .decode(header_b64)
        .map_err(|_| Report::new(OidcClientError::InvalidIdToken))?;
    let header: JwtHeader = serde_json::from_slice(&header_bytes)
        .map_err(|_| Report::new(OidcClientError::InvalidIdToken))?;

    if header.alg != "RS256" && header.alg != "ES256" {
        return Err(Report::new(OidcClientError::InvalidIdToken));
    }

    let sig_bytes = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|_| Report::new(OidcClientError::InvalidIdToken))?;
    let signing_input = format!("{header_b64}.{claims_b64}");

    let candidates: Vec<&Jwk> = match header.kid.as_deref() {
        Some(kid) => {
            let matching: Vec<&Jwk> = jwks
                .keys
                .iter()
                .filter(|k| k.kid.as_deref() == Some(kid))
                .collect();
            if matching.is_empty() {
                return Ok(VerifyStatus::UnknownKid);
            }
            matching
        }
        None => jwks.keys.iter().collect(),
    };

    let verified_sig = candidates
        .into_iter()
        .any(|jwk| verify_with_jwk(&header.alg, jwk, signing_input.as_bytes(), &sig_bytes).is_ok());
    if !verified_sig {
        return Err(Report::new(OidcClientError::InvalidIdToken));
    }

    let claims_bytes = URL_SAFE_NO_PAD
        .decode(claims_b64)
        .map_err(|_| Report::new(OidcClientError::InvalidIdToken))?;
    let claims: IdTokenClaims = serde_json::from_slice(&claims_bytes)
        .map_err(|_| Report::new(OidcClientError::InvalidIdToken))?;

    if claims.iss.trim_end_matches('/') != expected_issuer.trim_end_matches('/') {
        return Err(Report::new(OidcClientError::InvalidIdToken));
    }
    if !claims.aud.contains(expected_client_id) {
        return Err(Report::new(OidcClientError::InvalidIdToken));
    }
    let skew = CLOCK_SKEW.num_seconds();
    let now_ts = now.timestamp();
    if claims.exp + skew <= now_ts {
        return Err(Report::new(OidcClientError::InvalidIdToken));
    }
    if let Some(nbf) = claims.nbf
        && nbf - skew > now_ts
    {
        return Err(Report::new(OidcClientError::InvalidIdToken));
    }
    if claims.nonce.as_deref() != Some(expected_nonce) {
        return Err(Report::new(OidcClientError::NonceMismatch));
    }
    if claims.sub.trim().is_empty() {
        return Err(Report::new(OidcClientError::InvalidIdToken));
    }
    let email_verified = claims
        .email_verified
        .as_ref()
        .is_some_and(EmailVerifiedClaim::is_verified);
    if !email_verified {
        return Err(Report::new(OidcClientError::EmailNotVerified));
    }
    let raw_email = claims
        .email
        .as_deref()
        .ok_or_else(|| Report::new(OidcClientError::InvalidEmail))?;
    let email = EmailAdresse::normalisiert(raw_email)
        .map_err(|_| Report::new(OidcClientError::InvalidEmail))?;

    Ok(VerifyStatus::Verified(VerifiedOidcIdentity {
        provider_id: provider_id.clone(),
        subject: claims.sub,
        email,
        display_name: claims.name,
    }))
}

fn verify_with_jwk(alg: &str, jwk: &Jwk, signing_input: &[u8], sig: &[u8]) -> Result<(), ()> {
    if let Some(jwk_alg) = jwk.alg.as_deref()
        && jwk_alg != alg
    {
        return Err(());
    }
    match alg {
        "RS256" => {
            if jwk.kty != "RSA" {
                return Err(());
            }
            let n = URL_SAFE_NO_PAD
                .decode(jwk.n.as_deref().ok_or(())?)
                .map_err(|_| ())?;
            let e = URL_SAFE_NO_PAD
                .decode(jwk.e.as_deref().ok_or(())?)
                .map_err(|_| ())?;
            RsaPublicKeyComponents { n: &n, e: &e }
                .verify(&RSA_PKCS1_2048_8192_SHA256, signing_input, sig)
                .map_err(|_| ())
        }
        "ES256" => {
            if jwk.kty != "EC" || jwk.crv.as_deref() != Some("P-256") {
                return Err(());
            }
            let x = URL_SAFE_NO_PAD
                .decode(jwk.x.as_deref().ok_or(())?)
                .map_err(|_| ())?;
            let y = URL_SAFE_NO_PAD
                .decode(jwk.y.as_deref().ok_or(())?)
                .map_err(|_| ())?;
            if x.len() > 32 || y.len() > 32 {
                return Err(());
            }
            let mut point = [0u8; 65];
            point[0] = 0x04;
            point[1 + (32 - x.len())..33].copy_from_slice(&x);
            point[33 + (32 - y.len())..65].copy_from_slice(&y);
            UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, &point)
                .verify(signing_input, sig)
                .map_err(|_| ())
        }
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use ring::{
        rand::SystemRandom,
        signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair},
    };
    use serde_json::json;

    use super::*;

    fn es256_jwk_and_key() -> (EcdsaKeyPair, Jwk) {
        let rng = SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        let pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng)
            .unwrap();
        let pub_bytes = pair.public_key().as_ref();
        assert_eq!(pub_bytes.len(), 65);
        assert_eq!(pub_bytes[0], 0x04);
        let jwk = Jwk {
            kty: "EC".to_owned(),
            kid: Some("kid-1".to_owned()),
            alg: Some("ES256".to_owned()),
            crv: Some("P-256".to_owned()),
            n: None,
            e: None,
            x: Some(URL_SAFE_NO_PAD.encode(&pub_bytes[1..33])),
            y: Some(URL_SAFE_NO_PAD.encode(&pub_bytes[33..65])),
        };
        (pair, jwk)
    }

    fn sign_es256(pair: &EcdsaKeyPair, kid: &str, claims: serde_json::Value) -> String {
        let header = json!({ "alg": "ES256", "kid": kid, "typ": "JWT" });
        let h_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
        let c_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
        let signing_input = format!("{h_b64}.{c_b64}");
        let rng = SystemRandom::new();
        let sig = pair.sign(&rng, signing_input.as_bytes()).unwrap();
        format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(sig.as_ref()))
    }

    #[test_log::test]
    fn verifies_valid_es256_token_and_enforces_nonce_and_email_verified() {
        let (pair, jwk) = es256_jwk_and_key();
        let jwks = JwkSet { keys: vec![jwk] };
        let provider = OidcProviderId::parse("apple").unwrap();
        let now = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

        let valid_token = sign_es256(
            &pair,
            "kid-1",
            json!({
                "iss": "https://appleid.apple.com",
                "sub": "apple-sub-1",
                "aud": "app.friendarazzi.web",
                "exp": 1_700_000_600,
                "nonce": "nonce-123",
                "email": "relay@privaterelay.appleid.com",
                "email_verified": "true",
            }),
        );
        let VerifyStatus::Verified(verified) = verify_id_token(
            &valid_token,
            &jwks,
            &provider,
            "https://appleid.apple.com",
            "app.friendarazzi.web",
            "nonce-123",
            now,
        )
        .unwrap() else {
            panic!("expected verified");
        };
        assert_eq!(verified.subject, "apple-sub-1");
        assert_eq!(verified.email.as_ref(), "relay@privaterelay.appleid.com");

        let unverified_token = sign_es256(
            &pair,
            "kid-1",
            json!({
                "iss": "https://appleid.apple.com",
                "sub": "apple-sub-1",
                "aud": "app.friendarazzi.web",
                "exp": 1_700_000_600,
                "nonce": "nonce-123",
                "email": "user@example.at",
                "email_verified": false,
            }),
        );
        let err = verify_id_token(
            &unverified_token,
            &jwks,
            &provider,
            "https://appleid.apple.com",
            "app.friendarazzi.web",
            "nonce-123",
            now,
        )
        .err()
        .unwrap();
        assert_eq!(err.current_context(), &OidcClientError::EmailNotVerified);

        let wrong_nonce = sign_es256(
            &pair,
            "kid-1",
            json!({
                "iss": "https://appleid.apple.com",
                "sub": "apple-sub-1",
                "aud": "app.friendarazzi.web",
                "exp": 1_700_000_600,
                "nonce": "other-nonce",
                "email": "user@example.at",
                "email_verified": true,
            }),
        );
        let err = verify_id_token(
            &wrong_nonce,
            &jwks,
            &provider,
            "https://appleid.apple.com",
            "app.friendarazzi.web",
            "nonce-123",
            now,
        )
        .err()
        .unwrap();
        assert_eq!(err.current_context(), &OidcClientError::NonceMismatch);
    }
}
