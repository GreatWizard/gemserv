use url::form_urlencoded;

pub fn url_decode(url: &[u8]) -> String {
    let decoded: String = form_urlencoded::parse(url)
        .map(|(key, val)| [key, val].concat())
        .collect();
    return decoded
}
