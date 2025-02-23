#![allow(dead_code)]
use std::error::Error;

#[derive(Debug)]
pub enum GeminiResponse {
    Input {
        kind: InputKind,
        prompt: String,
    },
    Success {
        body: String,
    },
    Redirection {
        kind: RedirectionKind,
        to: String,
    },
    TemporaryFailure {
        kind: TemporaryFailureKind,
        msg: String,
    },
    PermanentFailure {
        kind: PermanentFailureKind,
        msg: String,
    },
    ClientCertificate {
        kind: CertificateErrorKind,
        msg: String,
    },
}

#[derive(Debug)]
pub struct GeminiResponseParseError {}
impl std::fmt::Display for GeminiResponseParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Couldn't parse gemini response")
    }
}

impl Error for GeminiResponseParseError {}

impl GeminiResponse {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, GeminiResponseParseError> {
        let err = Err(GeminiResponseParseError {});
        let mut crlf = false;
        if bytes.len() < 2 {
            return err;
        }
        let code = if let Ok(c) = std::str::from_utf8(&bytes[..2]) {
            if let Ok(c) = c.parse::<i32>() {
                c
            } else {
                return err;
            }
        } else {
            return err;
        };
        let mut i = 3;
        let mut body_start = -1;
        let mut response_data = String::new();
        while i < bytes.len() {
            let b = bytes[i];
            if b == '\r' as u8 {
                if i > 3 {
                    response_data = if let Ok(s) = String::from_utf8(Vec::from(&bytes[3..i])) {
                        s
                    } else {
                        return err;
                    }
                }
                if let Some(&lf) = bytes.get(i + 1) {
                    if lf == '\n' as u8 {
                        crlf = true;
                        i += 1;
                    } else {
                        return err;
                    }
                } else {
                    return err;
                }
            } else if crlf {
                body_start = i as i32;
                break;
            }
            i += 1;
        }

        let body_data = if body_start < 0 {
            String::new()
        } else {
            if let Ok(s) = String::from_utf8(Vec::from(&bytes[body_start as usize..])) {
                s
            } else {
                return err;
            }
        };
        let res = {
            if code >= 10 && code <= 19 {
                Self::Input {
                    kind: if code == 11 {
                        InputKind::Sensitive
                    } else {
                        InputKind::Basic
                    },
                    prompt: response_data,
                }
            } else if code >= 20 && code <= 29 {
                Self::Success { body: body_data }
            } else if code >= 30 && code <= 39 {
                Self::Redirection {
                    kind: if code == 31 {
                        RedirectionKind::Permanent
                    } else {
                        RedirectionKind::Temporary
                    },
                    to: response_data,
                }
            } else if code >= 40 && code <= 49 {
                Self::TemporaryFailure {
                    kind: if code == 41 {
                        TemporaryFailureKind::ServerUnavailable
                    } else if code == 42 {
                        TemporaryFailureKind::CGIError
                    } else if code == 43 {
                        TemporaryFailureKind::ProxyError
                    } else if code == 44 {
                        TemporaryFailureKind::SlowDown
                    } else {
                        TemporaryFailureKind::Unspecified
                    },
                    msg: response_data,
                }
            } else if code >= 50 && code <= 59 {
                Self::PermanentFailure {
                    kind: if code == 51 {
                        PermanentFailureKind::NotFound
                    } else if code == 52 {
                        PermanentFailureKind::Gone
                    } else if code == 53 {
                        PermanentFailureKind::ProxyRequestRefused
                    } else if code == 59 {
                        PermanentFailureKind::BadRequest
                    } else {
                        PermanentFailureKind::General
                    },
                    msg: response_data,
                }
            } else if code >= 60 && code <= 69 {
                Self::ClientCertificate {
                    kind: if code == 61 {
                        CertificateErrorKind::CertificateNotAuthorized
                    } else if code == 62 {
                        CertificateErrorKind::CertificateNotValid
                    } else {
                        CertificateErrorKind::CertificateRequired
                    },
                    msg: response_data,
                }
            } else {
                return err;
            }
        };

        Ok(res)
    }
}

#[derive(Debug)]
pub enum InputKind {
    Basic,     // 10
    Sensitive, // 11
}

#[derive(Debug)]
pub enum RedirectionKind {
    Temporary, // 30
    Permanent, // 31
}

#[derive(Debug)]
pub enum TemporaryFailureKind {
    Unspecified,       // 40
    ServerUnavailable, // 41
    CGIError,          // 42
    ProxyError,        // 43
    SlowDown,          // 44
}

#[derive(Debug)]
pub enum PermanentFailureKind {
    General,             // 50
    NotFound,            // 51
    Gone,                // 52
    ProxyRequestRefused, // 53
    BadRequest,          // 59
}

#[derive(Debug)]
pub enum CertificateErrorKind {
    CertificateRequired,      // 60
    CertificateNotAuthorized, // 61
    CertificateNotValid,      // 62
}
