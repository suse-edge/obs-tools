use std::{
    io::{BufRead, BufReader},
    sync::Arc,
};

use cookie_store::{Cookie, CookieResult, CookieStore};
use reqwest_cookie_store::CookieStoreRwLock;
use time::OffsetDateTime;
use url::Url;

fn parse_line(line: String) -> Option<CookieResult<'static>> {
    let line = match line.strip_prefix("Set-Cookie3: ") {
        Some(v) => v,
        None => return None,
    };
    Some(parse_cookie(line))
}

fn parse_cookie(line: &str) -> Result<Cookie<'static>, cookie_store::CookieError> {
    let attributes: Vec<(&str, Option<&str>)> = line
        .split(';')
        .map(|attr| match attr.find('=') {
            Some(i) => (
                attr[..i].trim(),
                Some(attr[(i + 1)..].trim().trim_matches('"')),
            ),
            None => (attr.trim(), None),
        })
        .collect();
    let line = attributes
        .iter()
        .map(|(k, v)| match v {
            Some(v) => format!("{}={}", k, v),
            None => k.to_string(),
        })
        .collect::<Vec<String>>()
        .join(";");

    let expires = attributes
        .iter()
        .find_map(|(k, v)| match (&*k.to_ascii_lowercase(), v) {
            ("expires", Some(v)) => {
                match OffsetDateTime::parse(
                    &v.replace(' ', "T"),
                    &time::format_description::well_known::Iso8601::PARSING,
                ) {
                    Ok(v) => Some(v),
                    Err(_) => None,
                }
            }
            (_, _) => None,
        });
    let mut raw_cookie: cookie_store::RawCookie<'static> = cookie_store::RawCookie::parse(line)?;
    if let Some(e) = expires {
        raw_cookie.set_expires(e);
    }
    let url = get_url_from_cookie(&raw_cookie)?;
    let cookie = Cookie::try_from_raw_cookie(&raw_cookie, &url)?;
    Ok(cookie)
}

fn get_url_from_cookie(cookie: &cookie_store::RawCookie) -> Result<Url, cookie_store::CookieError> {
    let scheme = match cookie.secure() {
        Some(true) => "https",
        _ => "http",
    };
    let domain = cookie
        .domain()
        .ok_or(cookie_store::CookieError::UnspecifiedDomain)?;
    let path = cookie.path().unwrap_or("/");
    Url::parse(&format!("{}://{}{}", scheme, domain, path))
        .map_err(|_| cookie_store::CookieError::Parse)
}

pub(crate) fn parse_lwp_cookiejar(
    reader: impl BufRead,
) -> Result<CookieStore, cookie_store::CookieError> {
    let mut lines = reader.lines();
    let line = lines.next().unwrap().unwrap();
    if !line.starts_with("#LWP-Cookies-") {
        return Err(cookie_store::CookieError::Parse);
    }
    CookieStore::from_cookies(lines.map_while(Result::ok).filter_map(parse_line), false)
}

pub fn get_osc_cookiejar() -> Result<Arc<CookieStoreRwLock>, cookie_store::CookieError> {
    if let Ok(xdg_paths) = xdg::BaseDirectories::with_prefix("osc") {
        if let Some(path) = xdg_paths.find_state_file("cookiejar") {
            if let Ok(cookiejar_file) = std::fs::File::open(path) {
                let cookie_jar_reader = BufReader::new(cookiejar_file);
                let cookie_jar = parse_lwp_cookiejar(cookie_jar_reader)?;
                return Ok(Arc::new(reqwest_cookie_store::CookieStoreRwLock::new(
                    cookie_jar,
                )));
            }
        }
    }

    let cookie_jar = CookieStore::new(None);
    Ok(Arc::new(reqwest_cookie_store::CookieStoreRwLock::new(
        cookie_jar,
    )))
}
