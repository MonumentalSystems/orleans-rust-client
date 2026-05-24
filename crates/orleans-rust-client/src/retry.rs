//! Conservative, opt-in retry policy.
//!
//! Retries are **disabled by default**. Grain calls are not assumed to be
//! idempotent, so automatic retries can only be safe when the bridge reports
//! an error as `retryable` (for example a placement rejection that never
//! reached the grain). Enable retries explicitly via
//! [`crate::OrleansClientBuilder::retry_policy`].

use std::time::Duration;

/// Exponential-backoff retry configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retries after the initial attempt. Zero disables
    /// retries.
    pub max_retries: u32,
    /// Backoff before the first retry.
    pub initial_backoff: Duration,
    /// Upper bound on backoff between retries.
    pub max_backoff: Duration,
    /// Multiplier applied to the backoff after each attempt.
    pub backoff_multiplier: f64,
}

impl RetryPolicy {
    /// No retries. This is the default.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            max_retries: 0,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
            backoff_multiplier: 1.0,
        }
    }

    /// A deliberately small policy: at most two retries with capped backoff,
    /// suitable only for errors the bridge has flagged as retryable.
    #[must_use]
    pub fn conservative() -> Self {
        Self {
            max_retries: 2,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(2),
            backoff_multiplier: 2.0,
        }
    }

    /// Whether any retries are permitted.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.max_retries > 0
    }

    /// Backoff before the retry numbered `attempt` (1-based).
    #[must_use]
    pub fn backoff_for(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }
        let exp = self.backoff_multiplier.powi((attempt - 1) as i32);
        let millis = self.initial_backoff.as_secs_f64() * 1000.0 * exp;
        let capped = millis.min(self.max_backoff.as_secs_f64() * 1000.0);
        Duration::from_millis(capped as u64)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::disabled()
    }
}
