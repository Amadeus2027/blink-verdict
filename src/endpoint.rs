/// Small, deliberately restricted service URL parser. No URL/IDNA runtime is needed.
/// Unicode hosts must be entered as punycode; paths may use percent encoding.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug)]
pub(crate) struct Endpoint<'a> {
    pub host: &'a str,
    pub port: u16,
    pub path: &'a str,
    pub secure: bool,
    pub loopback: bool,
}

impl<'a> Endpoint<'a> {
    pub fn parse(value: &'a str) -> Result<Self, String> {
        let invalid =
            || "API 地址无效：请使用 ASCII 域名 / IP，且不含账号、查询参数、片段或空白".to_string();
        if !value.is_ascii()
            || value
                .bytes()
                .any(|c| c <= b' ' || c == 127 || b"\\?#".contains(&c))
        {
            return Err(invalid());
        }
        let (secure, rest) = if let Some(rest) = value.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = value.strip_prefix("http://") {
            (false, rest)
        } else {
            return Err(invalid());
        };
        let (authority, path) = rest
            .find('/')
            .map_or((rest, "/"), |i| (&rest[..i], &rest[i..]));
        let (host, port_text) = if let Some(ipv6) = authority.strip_prefix('[') {
            let end = ipv6.find(']').ok_or_else(invalid)?;
            let host = &ipv6[..end];
            host.parse::<std::net::Ipv6Addr>().map_err(|_| invalid())?;
            let tail = &ipv6[end + 1..];
            (
                host,
                if tail.is_empty() {
                    None
                } else {
                    Some(tail.strip_prefix(':').ok_or_else(invalid)?)
                },
            )
        } else {
            let (host, port) = authority
                .split_once(':')
                .map_or((authority, None), |(h, p)| (h, Some(p)));
            if host.is_empty()
                || !host
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-".contains(&c))
            {
                return Err(invalid());
            }
            (host, port)
        };
        let port = match port_text {
            Some(p) if !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()) => {
                p.parse::<u16>().map_err(|_| invalid())?
            }
            Some(_) => return Err(invalid()),
            None => {
                if secure {
                    443
                } else {
                    80
                }
            }
        };
        if port == 0 {
            return Err(invalid());
        }
        let loopback = host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback());
        if !secure && !loopback {
            return Err("API 地址须使用 HTTPS；仅本机回环地址可使用 HTTP".into());
        }
        Ok(Self {
            host,
            port,
            path,
            secure,
            loopback,
        })
    }
}
