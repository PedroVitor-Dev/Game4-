//! Explicit routing endpoint policy shared by export, discovery and payload sessions.
use super::*;
pub fn normalize(values: &[String]) -> Result<Vec<String>> {
    if values.len() > 16 || values.iter().map(String::len).sum::<usize>() > 4096 {
        return Err(limit());
    }
    let mut result = Vec::new();
    for value in values {
        if value.is_empty()
            || value.len() > 2048
            || value.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(library::invalid(ErrorField::Request));
        }
        let u = url::Url::parse(value).map_err(|_| library::invalid(ErrorField::Request))?;
        if !matches!(u.scheme(), "http" | "https" | "udp")
            || u.host_str().is_none()
            || !u.username().is_empty()
            || u.password().is_some()
            || u.fragment().is_some()
            || u.port() == Some(0)
            || (u.scheme() == "udp" && (u.port().is_none() || u.query().is_some()))
        {
            return Err(library::invalid(ErrorField::Request));
        }
        let value = u.to_string();
        if !result.contains(&value) {
            result.push(value);
        }
    }
    Ok(result)
}
pub fn magnet(hash: &str, trackers: &[String]) -> String {
    let mut u = url::Url::parse(&format!("magnet:?xt=urn:btih:{hash}")).expect("validated hash");
    for t in trackers {
        u.query_pairs_mut().append_pair("tr", t);
    }
    u.to_string()
}
