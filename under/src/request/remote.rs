use crate::HttpEntity;
use std::borrow::Cow;
use std::net::IpAddr;

/// A type that helps retrieve the remote address of a request.
///
/// Because the process is incredibly complex, we can't just have a single
/// function that returns a single IP address - the _source_ of that IP address
/// is incredibly important, and we need to know what sources the application
/// in question trusts.
///
/// The basic idea is this: there are multiple potential sources to load an IP
/// address from.  All of these potential sources must be derivable from the
/// request itself.  The application must then specify which sources it trusts
/// to be able to load an IP address from.  Those sources themselves can have
/// their own configurations that allow them to be more fine-grained.  We do
/// not suggest a default - or, rather, disencourage it.  Instead, we make it
/// easier to specify.
///
/// For more information, see <https://adam-p.ca/blog/2022/03/x-forwarded-for/>.
///
/// # Examples
/// ```rust
/// # use under::*;
/// # use std::net::IpAddr;
/// # let mut request = Request::get("/").unwrap().with_local_addr();
/// request.set_header("X-Forwarded-For", "1.1.1.1, 2.2.2.2, 3.3.3.3");
/// let ip = request.remote_address()
///     .trust_cloudflare_header()
///     .trust_forwarded_for(-1)
///     .trust_peer_address()
///     .apply();
/// assert_eq!(ip, Some(IpAddr::from([3, 3, 3, 3])));
/// ```
#[derive(Debug, Clone)]
pub struct RemoteAddress<'r> {
    /// The request itself.  We borrow the request in here so that in the end
    /// we can just simply have the application pull the IP address from here.
    request: &'r super::Request,
    /// The sources that are trusted to load an IP address from.
    trusted_sources: Vec<RemoteAddressSource>,
}

impl<'r> RemoteAddress<'r> {
    pub(crate) fn new(request: &'r super::Request) -> Self {
        Self {
            request,
            trusted_sources: vec![],
        }
    }
}

impl RemoteAddress<'_> {
    /// Adds a source that loads from the X-Forwarded-For header.  The index
    /// here specifies _which_ entry in the X-Forwarded-For header to use.
    /// This is useful for load balancing applications that use multiple
    /// load balancers, or have multiple proxies between the application or
    /// the user.  Ideally, the application would specify the right-most
    /// specific source, not including the load balancers.
    ///
    /// This source correctly parses multiple `X-Forwarded-For` headers,
    /// implicitly concatenating them.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use std::net::IpAddr;
    /// # let mut request = Request::get("/").unwrap();
    /// request.set_header("X-Forwarded-For", "1.1.1.1, 2.2.2.2, 3.3.3.3");
    /// let ip = request.remote_address()
    ///     .trust_forwarded_for(0)
    ///     .apply();
    /// assert_eq!(ip, Some(IpAddr::from([1, 1, 1, 1])));
    /// let ip = request.remote_address()
    ///     .trust_forwarded_for(-1)
    ///     .apply();
    /// assert_eq!(ip, Some(IpAddr::from([3, 3, 3, 3])));
    /// ```
    pub fn trust_forwarded_for(&mut self, index: isize) -> &mut Self {
        self.trusted_sources
            .push(RemoteAddressSource::XForwardedFor(index));
        self
    }

    /// Adds a source that loads from the Forwarded header.  The index here
    /// specifies _which_ entry in the Forwarded header to use.  This is
    /// useful for load balancing applications that use multiple load
    /// balancers, or have multiple proxies between the application or the
    /// user.  Ideally, the application would specify the right-most
    /// specific source, not including the load balancers.
    ///
    /// This source correctly parses multiple `Forwarded` headers,
    /// implicitly concatenating them.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use std::net::IpAddr;
    /// # let mut request = Request::get("/").unwrap();
    /// request.set_header("Forwarded", "for=1.1.1.1, for=2.2.2.2, for=3.3.3.3");
    /// let ip = request.remote_address()
    ///    .trust_forwarded(0)
    ///    .apply();
    /// assert_eq!(ip, Some(IpAddr::from([1, 1, 1, 1])));
    /// let ip = request.remote_address()
    ///   .trust_forwarded(-1)
    ///   .apply();
    /// assert_eq!(ip, Some(IpAddr::from([3, 3, 3, 3])));
    /// ```
    pub fn trust_forwarded(&mut self, index: isize) -> &mut Self {
        self.trusted_sources
            .push(RemoteAddressSource::Forwarded(index));
        self
    }

    /// Adds a source that loads from a specific header.  This is useful for
    /// load balancing applications where the load balancer adds a header to
    /// the request that contains the IP address of the client.  Note that,
    /// however, if the load balancer is not trusted, or that the application
    /// can ever be accessed without the load balancer, this will not work.
    /// This source iterates over all possible header values, from top to
    /// bottom, and returns the first one that parses as an IP address.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use std::net::IpAddr;
    /// # let mut request = Request::get("/").unwrap();
    /// request.set_header("X-Real-IP", "1.1.1.1");
    /// request.add_header("X-Real-IP", "2.2.2.2");
    /// let ip = request.remote_address().trust_header("X-Real-IP").apply();
    /// assert_eq!(ip, Some(IpAddr::from([1, 1, 1, 1])));
    /// ```
    pub fn trust_header(&mut self, header: impl Into<Cow<'static, str>>) -> &mut Self {
        self.trusted_sources
            .push(RemoteAddressSource::Header(header.into()));
        self
    }

    /// Adds a source that loads from the header `"CF-Connecting-IP"`.  This
    /// uses the same logic as [`Self::trust_header`] to parse the header.
    pub fn trust_cloudflare_header(&mut self) -> &mut Self {
        self.trust_header("CF-Connecting-IP")
    }

    /// Adds a source that loads from the header `"X-Real-IP"`.  This uses
    /// the same logic as [`Self::trust_header`] to parse the header.
    pub fn trust_real_ip_header(&mut self) -> &mut Self {
        self.trust_header("X-Real-IP")
    }

    /// Adds a source that loads from the header `"True-Client-IP"`.  This
    /// uses the same logic as [`Self::trust_header`] to parse the header.
    pub fn trust_client_ip_header(&mut self) -> &mut Self {
        self.trust_header("True-Client-IP")
    }

    /// Adds a source that loads from the peer address of the TCP connection.
    /// There generally will always be a peer address, but if the application
    /// is behind a reverse proxy, this peer address will be the address of
    /// the reverse proxy, instead of the user's machine.  As such, this is
    /// most likely not what you want.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use std::net::IpAddr;
    /// # let mut request = Request::get("/").unwrap().with_local_addr();
    /// let ip = request.remote_address()
    ///     .trust_peer_address()
    ///     .apply();
    /// assert_eq!(ip, Some(IpAddr::from([127, 0, 0, 1])));
    /// ```
    pub fn trust_peer_address(&mut self) -> &mut Self {
        self.trusted_sources.push(RemoteAddressSource::PeerAddress);
        self
    }

    /// Applies the sources to the request, extracting the IP address.  Since
    /// all sources are fallible, this will return `None` if all of the sources
    /// fail.  All sources are evaluated from the first source added to the
    /// last.
    ///
    /// # Examples
    /// ```rust
    /// # use under::*;
    /// # use std::net::IpAddr;
    /// # let mut request = Request::get("/").unwrap().with_local_addr();
    /// request.set_header("X-Forwarded-For", "1.1.1.1, 2.2.2.2, 3.3.3.3");
    /// let ip = request.remote_address()
    ///     .trust_cloudflare_header()
    ///     .trust_forwarded_for(-1)
    ///     .trust_peer_address()
    ///     .apply();
    /// assert_eq!(ip, Some(IpAddr::from([3, 3, 3, 3])));
    /// ```
    #[must_use = "you probably don't intend to discard this value"]
    pub fn apply(&self) -> Option<IpAddr> {
        for source in &self.trusted_sources {
            if let Some(ip) = source.apply(self.request) {
                return Some(ip);
            }
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RemoteAddressSource {
    /// Loads the X-Forwarded-For header directly from the request.  The number
    /// here indicates which IP address to use; the first one is 0, the second
    /// is 1, etc.; if the number is negative, then it is the nth address from
    /// the right; so -1 is the last IP, -2 is the second-to-last, etc.
    XForwardedFor(isize),
    /// Loads the Forwarded header directly from the request.  The number here
    /// indicates which IP address to use; the first one is 0, the second is 1,
    /// etc.; if the number is negative, then it is the nth address from the
    /// right; so -1 is the last IP, -2 is the second-to-last, etc.
    ///
    /// The Forwarded header can contain more information than just an IP
    /// address, such as a secret provided by a proxy down the line; it is
    /// possible to include this as a parameter to the header.  Unfortunately,
    /// I do not know of any use cases for this yet, so I have not implemented
    /// it yet (feel free to open an Issue/PR!).
    Forwarded(isize),
    /// Pulls the IP address straight from the specified header.  Ideally this
    /// header would be set by a trusted proxy (overriding any previous
    /// headers), but if there is no trusted proxy, then it could be set by a
    /// client.
    Header(Cow<'static, str>),
    /// Pulls the IP address straight from the TCP/IP peer.  This is the most
    /// reliable source, but since the application's infrastructure may have
    /// (reverse) proxies in front of the user, it is not guaranteed to be
    /// accurate (and, in fact, if there is a trusted proxy, this may return
    /// the IP of the trusted proxy instead).
    PeerAddress,
}

impl RemoteAddressSource {
    pub fn apply(&self, request: &super::Request) -> Option<IpAddr> {
        match self {
            RemoteAddressSource::XForwardedFor(index) => x_forwarded_for_header(request, *index),
            RemoteAddressSource::Forwarded(index) => forwarded_header(request, *index),
            RemoteAddressSource::Header(name) => request
                .header_all(&**name)
                .into_iter()
                .filter_map(|v| v.to_str().ok())
                .find_map(|v| v.parse().ok()),
            RemoteAddressSource::PeerAddress => request.peer_addr().map(|v| v.ip()),
        }
    }
}

fn x_forwarded_for_header(request: &super::Request, index: isize) -> Option<IpAddr> {
    let mut ip = request
        .header_all("X-Forwarded-For")
        .into_iter()
        .filter_map(|s| s.to_str().ok())
        .flat_map(|s| s.split(','))
        .map(str::trim);

    if index < 0 {
        #[allow(clippy::cast_sign_loss)]
        let index = (index.checked_abs()? as usize).checked_sub(1)?;
        ip.nth_back(index).and_then(|s| s.parse().ok())
    } else if index >= 0 {
        #[allow(clippy::cast_sign_loss)]
        ip.nth(index as usize).and_then(|s| s.parse().ok())
    } else {
        None
    }
}

lazy_static::lazy_static! {
    static ref FOR_WORD: regex::Regex = regex::Regex::new(r"(?i)^for$").unwrap();
    static ref SPECIAL_TOKEN: regex::Regex = regex::Regex::new(r#"^"[(.+)]"$"#).unwrap();
}

// How is this even more unreliable than x-forwarded-for?  If it's not utf-8,
// or doesn't match key-value parsing pairs, than it'll ignore whole sections.
// Not sure this is a good thing.
fn forwarded_header(request: &super::Request, index: isize) -> Option<IpAddr> {
    fn parse_key_value(s: &str) -> Option<(&str, &str)> {
        let (key, value) = s.split_once('=')?;
        Some((key, value))
    }

    fn parse_ip(s: &str) -> Option<IpAddr> {
        let s = s.trim();
        if let Some(cap) = SPECIAL_TOKEN.captures(s) {
            cap[1].parse().ok()
        } else {
            s.parse().ok()
        }
    }

    let ip = request
        .header_all("Forwarded")
        .into_iter()
        .filter_map(|s| s.to_str().ok())
        .flat_map(|s| s.split(','))
        .map(str::trim)
        .map(|s| {
            s.split(';')
                .filter_map(|s| parse_key_value(s.trim()))
                .collect::<Vec<_>>()
        });

    // FOR_WORD is a requirement here because the standard says `for` is
    // case insensitive.  We _could_ try to lowercase it, but...
    let mut ffor = ip.filter_map(|v| {
        v.iter()
            .find(|(k, _)| FOR_WORD.is_match(k))
            .map(|(_, v)| *v)
    });

    if index < 0 {
        #[allow(clippy::cast_sign_loss)]
        let index = (index.checked_abs()? as usize).checked_sub(1)?;
        ffor.nth_back(index).and_then(parse_ip)
    } else if index >= 0 {
        #[allow(clippy::cast_sign_loss)]
        ffor.nth(index as usize).and_then(parse_ip)
    } else {
        None
    }
}
