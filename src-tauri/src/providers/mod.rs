pub mod claude;
pub mod codex;

use crate::models::{SessionProvider, SessionSource, SessionSummary};

pub struct ProviderScan {
    pub source: SessionSource,
    pub sessions: Vec<SessionSummary>,
    pub warnings: Vec<String>,
}

impl ProviderScan {
    pub fn available(
        provider: SessionProvider,
        location: String,
        sessions: Vec<SessionSummary>,
        warnings: Vec<String>,
    ) -> Self {
        let session_count = sessions.len();
        Self {
            source: SessionSource {
                provider,
                location,
                available: true,
                session_count,
                error: None,
            },
            sessions,
            warnings,
        }
    }

    pub fn unavailable(provider: SessionProvider, location: String, error: String) -> Self {
        Self {
            source: SessionSource {
                provider,
                location,
                available: false,
                session_count: 0,
                error: Some(error),
            },
            sessions: Vec::new(),
            warnings: Vec::new(),
        }
    }
}
