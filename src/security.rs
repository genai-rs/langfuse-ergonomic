//! Security utilities for protecting sensitive information
//!
//! This module provides types and utilities for handling sensitive data
//! such as API keys, tokens, and other secrets in a secure manner.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A string that contains sensitive information and is automatically redacted in Debug/Display
#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    /// Create a new SecretString
    pub fn new(value: impl Into<String>) -> Self {
        SecretString(value.into())
    }
    
    /// Get the actual value (use with caution)
    /// 
    /// # Security Warning
    /// Only use this method when you need to actually use the secret value.
    /// Never log or display the result of this method.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
    
    /// Check if the secret is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    
    /// Get the length of the secret
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretString(***REDACTED***)")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***REDACTED***")
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        SecretString::new(value)
    }
}

impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        SecretString::new(value)
    }
}

/// A redactor that can identify and redact sensitive information in strings
pub struct Redactor {
    patterns: Vec<RedactionPattern>,
}

#[derive(Debug, Clone)]
struct RedactionPattern {
    #[allow(dead_code)]
    name: String,
    regex: regex::Regex,
    replacement: String,
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Redactor {
    /// Create a new Redactor with default patterns
    pub fn new() -> Self {
        let patterns = vec![
            // API Keys and tokens (various formats)
            RedactionPattern {
                name: "api_key".to_string(),
                regex: regex::Regex::new(r"(?i)(api[_-]?key|apikey|api_secret|secret[_-]?key)[\s:=]+([A-Za-z0-9\-_]{20,})").unwrap(),
                replacement: "$1=***REDACTED***".to_string(),
            },
            // Bearer tokens
            RedactionPattern {
                name: "bearer_token".to_string(),
                regex: regex::Regex::new(r"(?i)Bearer\s+([A-Za-z0-9\-_.~+/]+=*)").unwrap(),
                replacement: "Bearer ***REDACTED***".to_string(),
            },
            // AWS Access Key ID
            RedactionPattern {
                name: "aws_access_key".to_string(),
                regex: regex::Regex::new(r"(?i)(AKIA[0-9A-Z]{16})").unwrap(),
                replacement: "***AWS_ACCESS_KEY_REDACTED***".to_string(),
            },
            // AWS Secret Access Key
            RedactionPattern {
                name: "aws_secret_key".to_string(),
                regex: regex::Regex::new(r"(?i)(aws[_-]?secret[_-]?access[_-]?key|aws[_-]?secret)[\s:=]+([A-Za-z0-9/+=]{40})").unwrap(),
                replacement: "$1=***REDACTED***".to_string(),
            },
            // JWT tokens
            RedactionPattern {
                name: "jwt".to_string(),
                regex: regex::Regex::new(r"eyJ[A-Za-z0-9-_]+\.eyJ[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+").unwrap(),
                replacement: "***JWT_REDACTED***".to_string(),
            },
            // Generic secrets in key=value format
            RedactionPattern {
                name: "secret_value".to_string(),
                regex: regex::Regex::new(r"(?i)(password|passwd|pwd|secret|token|auth|key)[\s:=]+([^\s,;]+)").unwrap(),
                replacement: "$1=***REDACTED***".to_string(),
            },
            // URLs with embedded credentials
            RedactionPattern {
                name: "url_credentials".to_string(),
                regex: regex::Regex::new(r"(https?://)([^:]+):([^@]+)@").unwrap(),
                replacement: "$1***:***@".to_string(),
            },
            // Email addresses (optional, can be disabled)
            RedactionPattern {
                name: "email".to_string(),
                regex: regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
                replacement: "***EMAIL_REDACTED***".to_string(),
            },
            // Credit card numbers (basic pattern)
            RedactionPattern {
                name: "credit_card".to_string(),
                regex: regex::Regex::new(r"\b(?:\d[ -]*?){13,16}\b").unwrap(),
                replacement: "***CARD_REDACTED***".to_string(),
            },
            // Social Security Numbers (US)
            RedactionPattern {
                name: "ssn".to_string(),
                regex: regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
                replacement: "***SSN_REDACTED***".to_string(),
            },
        ];
        
        Redactor { patterns }
    }
    
    /// Add a custom redaction pattern
    pub fn add_pattern(&mut self, name: impl Into<String>, pattern: &str, replacement: impl Into<String>) -> Result<(), regex::Error> {
        let regex = regex::Regex::new(pattern)?;
        self.patterns.push(RedactionPattern {
            name: name.into(),
            regex,
            replacement: replacement.into(),
        });
        Ok(())
    }
    
    /// Redact sensitive information from a string
    pub fn redact(&self, text: &str) -> String {
        let mut result = text.to_string();
        for pattern in &self.patterns {
            result = pattern.regex.replace_all(&result, pattern.replacement.as_str()).to_string();
        }
        result
    }
    
    /// Redact sensitive information from a JSON value
    pub fn redact_json(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => serde_json::Value::String(self.redact(s)),
            serde_json::Value::Object(map) => {
                let mut redacted_map = serde_json::Map::new();
                for (key, val) in map {
                    // Check if the key name suggests sensitive data
                    let is_sensitive_key = key.to_lowercase().contains("secret") ||
                                         key.to_lowercase().contains("password") ||
                                         key.to_lowercase().contains("token") ||
                                         key.to_lowercase().contains("key") ||
                                         key.to_lowercase().contains("auth");
                    
                    if is_sensitive_key && val.is_string() {
                        redacted_map.insert(key.clone(), serde_json::Value::String("***REDACTED***".to_string()));
                    } else {
                        redacted_map.insert(key.clone(), self.redact_json(val));
                    }
                }
                serde_json::Value::Object(redacted_map)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.redact_json(v)).collect())
            }
            other => other.clone(),
        }
    }
}

/// Trait for types that can redact their sensitive information
pub trait Redactable {
    /// Return a redacted version of self suitable for logging/display
    fn redacted(&self) -> String;
}

impl Redactable for String {
    fn redacted(&self) -> String {
        let redactor = Redactor::new();
        redactor.redact(self)
    }
}

impl Redactable for str {
    fn redacted(&self) -> String {
        let redactor = Redactor::new();
        redactor.redact(self)
    }
}

impl Redactable for serde_json::Value {
    fn redacted(&self) -> String {
        let redactor = Redactor::new();
        let redacted_value = redactor.redact_json(self);
        serde_json::to_string_pretty(&redacted_value).unwrap_or_else(|_| "***REDACTION_ERROR***".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_secret_string_redaction() {
        let secret = SecretString::new("my-super-secret-key");
        
        // Debug should be redacted
        let debug_str = format!("{:?}", secret);
        assert_eq!(debug_str, "SecretString(***REDACTED***)");
        
        // Display should be redacted
        let display_str = format!("{}", secret);
        assert_eq!(display_str, "***REDACTED***");
        
        // expose_secret should return actual value
        assert_eq!(secret.expose_secret(), "my-super-secret-key");
    }
    
    #[test]
    fn test_redactor_api_keys() {
        let redactor = Redactor::new();
        
        let text = "My API_KEY=sk-1234567890abcdefghijklmnop and it's secret";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("sk-1234567890"));
    }
    
    #[test]
    fn test_redactor_bearer_token() {
        let redactor = Redactor::new();
        
        let text = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("Bearer ***REDACTED***"));
        assert!(!redacted.contains("eyJ"));
    }
    
    #[test]
    fn test_redactor_url_credentials() {
        let redactor = Redactor::new();
        
        let text = "Connect to https://user:password123@example.com/api";
        let redacted = redactor.redact(text);
        assert!(redacted.contains("https://***:***@"));
        assert!(!redacted.contains("password123"));
    }
    
    #[test]
    fn test_redactor_json() {
        let redactor = Redactor::new();
        
        let json = serde_json::json!({
            "name": "test",
            "api_key": "sk-secret123",
            "password": "mypassword",
            "data": {
                "token": "bearer-token-xyz",
                "public": "this is public"
            }
        });
        
        let redacted = redactor.redact_json(&json);
        let redacted_str = serde_json::to_string(&redacted).unwrap();
        
        assert!(redacted_str.contains("***REDACTED***"));
        assert!(!redacted_str.contains("sk-secret123"));
        assert!(!redacted_str.contains("mypassword"));
        assert!(!redacted_str.contains("bearer-token-xyz"));
        assert!(redacted_str.contains("this is public"));
    }
    
    #[test]
    fn test_redactable_trait() {
        let text = "My secret is password=supersecret123".to_string();
        let redacted = text.redacted();
        assert!(redacted.contains("***REDACTED***"));
        assert!(!redacted.contains("supersecret123"));
    }
}