//! # Password Hashing – Argon2
//!
//! Hash và verify password sử dụng Argon2id (recommended bởi OWASP).
//! Argon2id kết hợp chống side-channel attack (Argon2i) và GPU attack (Argon2d).

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::core::errors::AppError;

/// Hash password bằng Argon2id với random salt.
///
/// Output format: `$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Password hashing failed: {e}")))?;

    Ok(hash.to_string())
}

/// Verify password against stored hash.
///
/// Returns `true` nếu password khớp, `false` nếu không.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash format: {e}")))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
