use url::form_urlencoded;

pub fn url_decode(url: &[u8]) -> String {
    let decoded: String = form_urlencoded::parse(url)
        .map(|(key, val)| [key, val].concat())
        .collect();
    return decoded
}

pub fn fingerhex(x509: &openssl::x509::X509) -> String {
    let finger = match x509.digest(openssl::hash::MessageDigest::sha256()) {
        Ok(f) => f,
        _ => return "".to_string(),
    };
    let mut hex: String = String::from("SHA256:");
    for f in finger.as_ref() {
        hex.push_str(&format!("{:02X}", f));
    }
    hex
}
