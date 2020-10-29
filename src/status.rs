use std::fmt;

#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
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

impl Status {
    pub fn to_str(&self) -> &str {
        let meta = match self {
            Status::Input => "Input",
            Status::Success => "Success",
            Status::SuccessEndOfSession => "Success End Of Session",
            Status::RedirectTemporary => "Redirect Temporary",
            Status::RedirectPermanent => "Redirect Permanent",
            Status::TemporaryFailure => "Temporary Failure",
            Status::ServerUnavailable => "Server Unavailable",
            Status::CGIError => "CGI Error!",
            Status::ProxyError => "Proxy Error!",
            Status::SlowDown => "Slow Down!",
            Status::PermanentFailure => "Permanent Failure",
            Status::NotFound => "Not Found!",
            Status::Gone => "Gone!",
            Status::ProxyRequestRefused => "Proxy Request Refused",
            Status::BadRequest => "Bad Request!",
            Status::ClientCertificateRequired => "Client Certificate Required",
            Status::TransientCertificateRequested => "Transient Certificate Requested",
            Status::AuthorisedCertificateRequired => "Authorised Certificate Required",
            Status::CertificateNotAccepted => "Certificate Not Accepted",
            Status::FutureCertificateRejected => "Future Certificate Rejected",
            Status::ExpiredCertificateRejected => "Expired Certificate Rejected",
        };
        return meta;
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
