//! Authentication, session management, and rate limiting for Studio Web.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

use argon2::Argon2;
use chrono::{DateTime, Utc};
use password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

/// Maximum failed login attempts before lockout.
const MAX_LOGIN_ATTEMPTS: u32 = 5;
/// Duration of lockout window.
const LOCKOUT_DURATION: Duration = Duration::from_secs(15 * 60); // 15 minutes
/// Sudo grace period for destructive actions (10 minutes).
const SUDO_DURATION: Duration = Duration::from_secs(10 * 60);
/// Session idle expiration (24 hours).
const SESSION_TTL: Duration = Duration::from_secs(24 * 3600);

/// Hash a plaintext password with Argon2id and a random salt.
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Lỗi băm mật khẩu: {e}"))?;
    Ok(hash.to_string())
}

/// Verify a plaintext password against an Argon2 PHC string.
pub fn verify_password(password: &str, hash_str: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(hash_str) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// An active web session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub last_sudo: Option<DateTime<Utc>>,
}

impl Session {
    pub fn new(token: String) -> Self {
        let now = Utc::now();
        Self {
            token,
            created_at: now,
            last_active: now,
            last_sudo: Some(now), // Login grants initial 10-minute sudo
        }
    }

    /// Check if session is within the 10-minute sudo grace period.
    pub fn is_sudo(&self) -> bool {
        if let Some(sudo_time) = self.last_sudo {
            let elapsed = Utc::now() - sudo_time;
            elapsed.num_seconds() >= 0 && elapsed.num_seconds() <= SUDO_DURATION.as_secs() as i64
        } else {
            false
        }
    }

    /// Refresh sudo grace period.
    pub fn refresh_sudo(&mut self) {
        self.last_sudo = Some(Utc::now());
    }

    /// Touch last active timestamp.
    pub fn touch(&mut self) {
        self.last_active = Utc::now();
    }
}

/// In-memory session store.
#[derive(Debug, Default)]
pub struct SessionStore {
    sessions: RwLock<HashMap<String, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Create and store a new session.
    pub fn create_session(&self) -> Session {
        let token = uuid::Uuid::new_v4().simple().to_string();
        let session = Session::new(token.clone());
        let mut map = self.sessions.write().expect("lock poisoned");
        // Prune expired sessions
        let now = Utc::now();
        map.retain(|_, s| {
            let age = now - s.last_active;
            age.num_seconds() < SESSION_TTL.as_secs() as i64
        });
        map.insert(token, session.clone());
        session
    }

    /// Validate token and touch session.
    pub fn validate(&self, token: &str) -> Option<Session> {
        let mut map = self.sessions.write().expect("lock poisoned");
        if let Some(sess) = map.get_mut(token) {
            let age = Utc::now() - sess.last_active;
            if age.num_seconds() < SESSION_TTL.as_secs() as i64 {
                sess.touch();
                return Some(sess.clone());
            }
        }
        None
    }

    /// Refresh sudo for an active session.
    pub fn refresh_sudo(&self, token: &str) -> bool {
        let mut map = self.sessions.write().expect("lock poisoned");
        if let Some(sess) = map.get_mut(token) {
            sess.refresh_sudo();
            true
        } else {
            false
        }
    }

    /// Invalidate a session (logout).
    pub fn remove(&self, token: &str) {
        let mut map = self.sessions.write().expect("lock poisoned");
        map.remove(token);
    }
}

#[derive(Debug)]
struct AttemptRecord {
    count: u32,
    first_attempt: Instant,
    locked_until: Option<Instant>,
}

/// IP-based rate limiter for login attempts.
#[derive(Debug, Default)]
pub struct LoginRateLimiter {
    attempts: Mutex<HashMap<IpAddr, AttemptRecord>>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
        }
    }

    /// Check if IP is currently locked out. Returns remaining lockout seconds if locked.
    pub fn check_lockout(&self, ip: IpAddr) -> Option<u64> {
        let mut map = self.attempts.lock().expect("lock poisoned");
        if let Some(rec) = map.get_mut(&ip) {
            if let Some(locked_until) = rec.locked_until {
                let now = Instant::now();
                if now < locked_until {
                    let rem = locked_until.duration_since(now).as_secs();
                    return Some(rem.max(1));
                } else {
                    // Lockout expired, reset
                    map.remove(&ip);
                }
            }
        }
        None
    }

    /// Record a failed login attempt. Returns true if locked out now.
    pub fn record_failure(&self, ip: IpAddr) -> (bool, u64) {
        let mut map = self.attempts.lock().expect("lock poisoned");
        let now = Instant::now();
        let rec = map.entry(ip).or_insert(AttemptRecord {
            count: 0,
            first_attempt: now,
            locked_until: None,
        });

        // Reset if window has passed
        if now.duration_since(rec.first_attempt) > LOCKOUT_DURATION {
            rec.count = 0;
            rec.first_attempt = now;
            rec.locked_until = None;
        }

        rec.count += 1;
        if rec.count >= MAX_LOGIN_ATTEMPTS {
            rec.locked_until = Some(now + LOCKOUT_DURATION);
            (true, LOCKOUT_DURATION.as_secs())
        } else {
            (false, 0)
        }
    }

    /// Record successful login (clears failures).
    pub fn record_success(&self, ip: IpAddr) {
        let mut map = self.attempts.lock().expect("lock poisoned");
        map.remove(&ip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_argon2_hashing_and_verification() {
        let pass = "my_owner_secret_password_123";
        let hash = hash_password(pass).expect("hash success");
        assert!(verify_password(pass, &hash));
        assert!(!verify_password("wrong_password", &hash));
    }

    #[test]
    fn test_session_lifecycle_and_sudo() {
        let store = SessionStore::new();
        let session = store.create_session();
        assert!(session.is_sudo());

        let validated = store.validate(&session.token).expect("valid token");
        assert_eq!(validated.token, session.token);

        store.remove(&session.token);
        assert!(store.validate(&session.token).is_none());
    }

    #[test]
    fn test_rate_limiter_lockout() {
        let limiter = LoginRateLimiter::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

        assert_eq!(limiter.check_lockout(ip), None);
        for _ in 0..4 {
            let (locked, _) = limiter.record_failure(ip);
            assert!(!locked);
            assert_eq!(limiter.check_lockout(ip), None);
        }

        // 5th attempt locks out
        let (locked, rem) = limiter.record_failure(ip);
        assert!(locked);
        assert!(rem > 0);
        assert!(limiter.check_lockout(ip).is_some());

        // Success on another IP does not clear this IP
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101));
        limiter.record_success(ip2);
        assert!(limiter.check_lockout(ip).is_some());

        // Success on this IP clears it
        limiter.record_success(ip);
        assert_eq!(limiter.check_lockout(ip), None);
    }
}
