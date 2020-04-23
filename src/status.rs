use std::fmt;

#[repr(u8)]
#[derive(Debug)]
pub enum Status {
    Input = 10,
    Success = 20,
    SuccessEndOfSession = 21,
    RedirectTemporary = 30,
    RedirectPermanent = 31,
    TemporaryFailure = 40,
    ServerUnavailable = 41,
    CGIError = 42,
    ProxyError = 43,
    SlowDown = 44,
    PermanentFailure = 50,
    NotFound = 51,
    Gone = 52,
    ProxyRequestRefused = 53,
    BadRequest = 59,
    ClientCertificateRequired = 60,
    TransientCertificateRequested = 61,
    AuthorisedCertificateRequired = 62,
    CertificateNotAccepted = 63,
    FutureCertificateRejected = 64,
    ExpiredCertificateRejected = 65,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Status::Success => "20\t",
            _ => "",
        }
    }
}
