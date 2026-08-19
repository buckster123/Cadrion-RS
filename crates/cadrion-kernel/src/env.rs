//! Environment helpers. New vars are `CADRION_*`; `CADRE_*` still works.

/// Read `name` (`CADRION_*`), then the matching `CADRE_*` legacy key.
pub fn env_var(name: &str) -> Result<String, std::env::VarError> {
    match std::env::var(name) {
        Ok(v) => Ok(v),
        Err(e) => {
            if let Some(rest) = name.strip_prefix("CADRION_") {
                std::env::var(format!("CADRE_{rest}"))
            } else {
                Err(e)
            }
        }
    }
}

/// True when `got` equals `want`, or the Cadre-era `cadre.` form of a `cadrion.` schema.
pub fn schema_matches(got: &str, want: &str) -> bool {
    if got == want {
        return true;
    }
    want.strip_prefix("cadrion.")
        .is_some_and(|rest| got == format!("cadre.{rest}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_accepts_legacy_cadre_prefix() {
        assert!(schema_matches("cadrion.dfm_profile", "cadrion.dfm_profile"));
        assert!(schema_matches("cadre.dfm_profile", "cadrion.dfm_profile"));
        assert!(!schema_matches("other.dfm_profile", "cadrion.dfm_profile"));
    }
}
