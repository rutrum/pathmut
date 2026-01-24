// experiment:
// build a data structure that deserialize and re-serialize any URL or Path
// without losing any information

use typed_path::{Utf8TypedPath, Utf8UnixComponent, Utf8WindowsComponent, Utf8WindowsPrefix};

use url::{Host, ParseError, Url};

use std::borrow::Borrow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKind {
    Domain(Vec<String>), // Split by dots: ["api", "example", "com"]
    Ipv4(String),        // "192.168.1.1"
    Ipv6(String),        // "[::1]" (with brackets for serialization)
}

#[derive(Debug, Clone)]
pub struct Path {
    segments: Vec<Segment>,
    kind: PathKind,
}

#[derive(Debug, Clone)]
enum PathKind {
    Unix {
        root: bool,
    },
    Windows {
        prefix: Option<WindowsPrefix>,
        root: bool,
    },
    Url {
        scheme: Option<String>,
        username: Option<String>,
        password: Option<String>,
        host: HostKind,
        port: Option<u16>,
        query_params: Vec<String>,
        fragment: Option<String>,
    },
}

#[derive(Debug, Clone)]
enum WindowsPrefix {
    Verbatim(String),
    VerbatimUNC(String, String),
    VerbatimDisk(char),
    DeviceNS(String),
    Unc(String, String),
    Disk(char),
}

impl std::fmt::Display for WindowsPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowsPrefix::Verbatim(path) => write!(f, r"\\?\{}", path),
            WindowsPrefix::VerbatimUNC(server, share) => write!(f, r"\\?\UNC\{}\{}", server, share),
            WindowsPrefix::VerbatimDisk(drive) => write!(f, r"\\?\{}:", drive),
            WindowsPrefix::DeviceNS(name) => write!(f, r"\\.\{}", name),
            WindowsPrefix::Unc(server, share) => write!(f, r"\\{}\{}", server, share),
            WindowsPrefix::Disk(drive) => write!(f, "{}:", drive),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Segment(String);

/// Any string that's delimited by periods
impl Segment {
    pub fn name(&self) -> String {
        self.0.clone()
    }

    pub fn set_name(&mut self, new_value: &str) {
        self.0 = new_value.into();
    }

    pub fn stem(&self) -> String {
        if self.len() > 1 {
            self.0
                .split(".")
                .take(self.len() - 1)
                .collect::<Vec<&str>>()
                .join(".")
        } else {
            self.0.clone()
        }
    }

    pub fn set_stem(&mut self, new_value: &str) {
        self.0 = if self.len() > 1 {
            format!("{}.{}", new_value, self.extension())
        } else {
            format!("{}.{}", new_value, self.0)
        };
    }

    pub fn extension(&self) -> String {
        if self.len() > 1 {
            self.0.split(".").last().unwrap().into()
        } else {
            String::new()
        }
    }

    pub fn set_extension(&mut self, new_value: &str) {
        self.0 = if self.len() > 1 {
            format!("{}.{}", self.stem(), new_value)
        } else {
            format!("{}.{}", self.0, new_value)
        };
    }

    pub fn prefix(&self) -> String {
        // this is probably not correct, see https://doc.rust-lang.org/std/path/struct.Path.html#method.file_prefix
        self.0
            .split(".")
            .next()
            .map(|s| s.into())
            .unwrap_or_default()
    }

    pub fn set_prefix(&mut self, new_value: &str) {
        self.0 = if self.len() > 1 {
            format!(
                "{}.{}",
                new_value,
                self.0.split(".").skip(1).collect::<Vec<&str>>().join(".")
            )
        } else {
            // this might not be correct
            format!("{}.{}", new_value, self.0)
        };
    }

    pub fn len(&self) -> usize {
        self.0.split(".").count()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Segment> for String {
    fn from(segment: Segment) -> String {
        segment.0
    }
}

impl Borrow<str> for Segment {
    fn borrow(&self) -> &str {
        &self.0
    }
}

// Helper function to convert typed_path's prefix to our owned version
fn convert_prefix_to_owned(prefix: Utf8WindowsPrefix) -> WindowsPrefix {
    match prefix {
        Utf8WindowsPrefix::Verbatim(s) => WindowsPrefix::Verbatim(s.to_string()),
        Utf8WindowsPrefix::VerbatimUNC(server, share) => {
            WindowsPrefix::VerbatimUNC(server.to_string(), share.to_string())
        }
        Utf8WindowsPrefix::VerbatimDisk(disk) => WindowsPrefix::VerbatimDisk(disk),
        Utf8WindowsPrefix::DeviceNS(name) => WindowsPrefix::DeviceNS(name.to_string()),
        Utf8WindowsPrefix::UNC(server, share) => {
            WindowsPrefix::Unc(server.to_string(), share.to_string())
        }
        Utf8WindowsPrefix::Disk(disk) => WindowsPrefix::Disk(disk),
    }
}

impl Path {
    pub fn is_unix(&self) -> bool {
        matches!(self.kind, PathKind::Unix { .. })
    }

    pub fn is_windows(&self) -> bool {
        matches!(self.kind, PathKind::Windows { .. })
    }

    pub fn is_url(&self) -> bool {
        matches!(self.kind, PathKind::Url { .. })
    }

    pub fn parse(path_str: &str) -> Path {
        let as_url = Path::parse_as_url(path_str);
        let as_path = Path::parse_as_typed_path(path_str);
        match (as_url, &as_path.kind) {
            (Ok((_, cannot_be_base)), PathKind::Windows { .. }) if cannot_be_base => as_path,
            (Ok((url, _)), _) => url,
            _ => as_path,
        }
    }

    pub fn parse_as_url(path_str: &str) -> Result<(Path, bool), ParseError> {
        let url = Url::parse(path_str)?;
        let scheme = Some(url.scheme().to_owned());
        let segments = match url.path_segments() {
            Some(segs) => segs
                .map(|s| s.to_owned())
                .filter(|x| !x.is_empty())
                .map(Segment)
                .collect(),
            Option::None => Vec::new(),
        };
        let username = if url.username().is_empty() {
            None
        } else {
            Some(url.username().to_owned())
        };
        let host = match url.host() {
            Some(Host::Domain(d)) => {
                HostKind::Domain(d.split('.').map(|s| s.to_string()).collect())
            }
            Some(Host::Ipv4(ipv4)) => HostKind::Ipv4(ipv4.to_string()),
            Some(Host::Ipv6(ipv6)) => {
                // Store with brackets for serialization consistency
                HostKind::Ipv6(format!("[{}]", ipv6))
            }
            None => HostKind::Domain(Vec::new()),
        };
        let port = url.port();

        Ok((
            Path {
                segments,
                kind: PathKind::Url {
                    scheme,
                    username,
                    password: url.password().map(|s| s.to_owned()),
                    host,
                    port,
                    query_params: url.query_pairs().map(|(a, b)| format!("{a}={b}")).collect(),
                    fragment: url.fragment().map(|s| s.to_owned()),
                },
            },
            url.cannot_be_a_base(),
        ))
    }

    pub fn parse_as_typed_path(path_str: &str) -> Path {
        let path = Utf8TypedPath::derive(path_str);
        let mut segments = Vec::new();
        match path {
            Utf8TypedPath::Unix(path) => {
                let mut root = false;
                for component in path.components() {
                    use Utf8UnixComponent::*;
                    let mut segment = None;
                    match component {
                        RootDir => root = true,
                        CurDir => segment = Some("."),
                        ParentDir => segment = Some(".."),
                        Normal(s) => segment = Some(s),
                    };
                    if let Some(s) = segment {
                        segments.push(Segment(s.to_string()))
                    }
                }
                Path {
                    segments,
                    kind: PathKind::Unix { root },
                }
            }
            Utf8TypedPath::Windows(path) => {
                let mut prefix = None;
                let mut root = false;
                for component in path.components() {
                    use Utf8WindowsComponent::*;
                    let mut segment = None;
                    match component {
                        RootDir => root = true,
                        CurDir => segment = Some("."),
                        ParentDir => segment = Some(".."),
                        Normal(s) => segment = Some(s),
                        Prefix(p) => {
                            prefix = Some(match p.kind() {
                                Utf8WindowsPrefix::Verbatim(s) => {
                                    WindowsPrefix::Verbatim(s.to_string())
                                }
                                Utf8WindowsPrefix::VerbatimUNC(s, t) => {
                                    WindowsPrefix::VerbatimUNC(s.to_string(), t.to_string())
                                }
                                Utf8WindowsPrefix::VerbatimDisk(s) => {
                                    WindowsPrefix::VerbatimDisk(s)
                                }
                                Utf8WindowsPrefix::DeviceNS(s) => {
                                    WindowsPrefix::DeviceNS(s.to_string())
                                }
                                Utf8WindowsPrefix::UNC(s, t) => {
                                    WindowsPrefix::Unc(s.to_string(), t.to_string())
                                }
                                Utf8WindowsPrefix::Disk(s) => WindowsPrefix::Disk(s),
                            })
                        }
                    };
                    if let Some(s) = segment {
                        segments.push(Segment(s.to_string()))
                    }
                }
                Path {
                    segments,
                    kind: PathKind::Windows { root, prefix },
                }
            }
        }
    }

    pub fn serialize(&self) -> String {
        match &self.kind {
            PathKind::Unix { root } => {
                format!(
                    "{}{}",
                    if *root { "/" } else { "" },
                    self.segments.join("/")
                )
            }
            PathKind::Windows { root, prefix } => {
                let left = if *root { r"\" } else { "" };
                let right = String::from_utf8(self.segments.join(r"\").into()).unwrap();
                if let Some(p) = prefix {
                    format!("{}{}{}", p, left, right)
                } else {
                    format!("{}{}", left, right)
                }
            }
            PathKind::Url {
                scheme,
                username,
                password,
                host,
                port,
                query_params,
                fragment,
            } => {
                let mut ss: Vec<String> = Vec::new();
                if let Some(s) = scheme {
                    ss.push(s.clone());
                    ss.push("://".to_owned());
                }
                let userpass = match (username, password) {
                    (Some(u), Some(p)) => format!("{u}:{p}@"),
                    (Some(u), None) => format!("{u}@"),
                    (None, Some(p)) => format!(":{p}@"),
                    (None, None) => String::new(),
                };
                ss.push(userpass);
                let host_str = match host {
                    HostKind::Domain(parts) => parts.join("."),
                    HostKind::Ipv4(addr) => addr.clone(),
                    HostKind::Ipv6(addr) => addr.clone(),
                };
                ss.push(host_str);
                if let Some(p) = port {
                    ss.push(format!(":{}", p));
                }
                if !self.segments.is_empty() {
                    ss.push("/".to_owned());
                    ss.push(self.segments.join("/"));
                }
                if !query_params.is_empty() {
                    ss.push("?".to_owned());
                    ss.push(query_params.join("&"));
                }
                if let Some(f) = fragment {
                    ss.push(format!("#{f}"));
                }
                ss.join("")
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Component {
    // Segment
    Extension,
    Stem,
    Name,
    FilePrefix,

    // Windows
    Disk,
    WindowsPrefix,

    // URL
    Scheme,
    Username,
    Password,
    Authority,
    Host,
    Tld,
    Path,
    Queries,
    Fragment,

    // URL - New components
    Port,   // Explicit port only
    IP,     // IPv4 or IPv6 (whichever exists)
    IPv4,   // IPv4 address (empty if not IPv4)
    IPv6,   // IPv6 address (empty if not IPv6)
    Origin, // scheme://host:port
}

impl TryFrom<&str> for Component {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        use Component::*;
        match s {
            "ext" | "extension" => Ok(Extension),
            "stem" => Ok(Stem),
            "name" => Ok(Name),
            "prefix" | "fileprefix" => Ok(FilePrefix),
            "disk" => Ok(Disk),
            "winprefix" => Ok(WindowsPrefix),
            // URL components
            "scheme" => Ok(Scheme),
            "username" | "user" => Ok(Username),
            "password" | "pass" => Ok(Password),
            "authority" | "auth" => Ok(Authority),
            "host" => Ok(Host),
            "tld" => Ok(Tld),
            "path" => Ok(Path),
            "queries" | "query" => Ok(Queries),
            "fragment" | "frag" => Ok(Fragment),
            "port" => Ok(Port),
            "ip" => Ok(IP),
            "ipv4" => Ok(IPv4),
            "ipv6" => Ok(IPv6),
            "origin" => Ok(Origin),
            _ => Err(format!("unknown component: {}", s)),
        }
    }
}

impl Path {
    pub fn get(&self, c: Component) -> String {
        use Component::*;
        use PathKind::*;
        match (c, &self.kind) {
            // windows
            (
                WindowsPrefix,
                Windows {
                    prefix: Some(p), ..
                },
            ) => p.to_string(),
            (
                Disk,
                Windows {
                    prefix: Some(p), ..
                },
            ) => {
                use self::WindowsPrefix::*;
                match p {
                    VerbatimDisk(disk) => (*disk).into(),
                    self::WindowsPrefix::Disk(disk) => (*disk).into(),
                    _ => String::new(),
                }
            }
            // Url
            (
                Scheme,
                Url {
                    scheme: Some(s), ..
                },
            ) => s.to_string(),
            (
                Username,
                Url {
                    username: Some(s), ..
                },
            ) => s.to_string(),
            (
                Password,
                Url {
                    password: Some(s), ..
                },
            ) => s.to_string(),
            (
                Authority,
                Url {
                    password, username, ..
                },
            ) => match (username, password) {
                (Some(u), Some(p)) => format!("{u}:{p}"),
                (None, Some(p)) => format!(":{p}"),
                (Some(u), None) => u.to_string(),
                (None, None) => String::new(),
            },
            (Host, Url { host, .. }) => match host {
                HostKind::Domain(parts) => parts.join("."),
                HostKind::Ipv4(addr) => addr.clone(),
                HostKind::Ipv6(addr) => addr.clone(),
            },
            (Tld, Url { host, .. }) => match host {
                HostKind::Domain(parts) if !parts.is_empty() => parts.last().unwrap().clone(),
                _ => String::new(), // Empty for IPv4/IPv6
            },
            (
                Queries,
                Url {
                    query_params: qps, ..
                },
            ) if !qps.is_empty() => format!("?{}", qps.join("&")),
            (
                Fragment,
                Url {
                    fragment: Some(s), ..
                },
            ) => s.to_string(),
            // URL - New components
            (Port, Url { port, .. }) => port.map(|p| p.to_string()).unwrap_or_default(),
            (IPv4, Url { host, .. }) => match host {
                HostKind::Ipv4(addr) => addr.clone(),
                _ => String::new(),
            },
            (IPv6, Url { host, .. }) => match host {
                HostKind::Ipv6(addr) => addr.clone(),
                _ => String::new(),
            },
            (IP, Url { host, .. }) => match host {
                HostKind::Ipv4(addr) => addr.clone(),
                HostKind::Ipv6(addr) => addr.clone(),
                HostKind::Domain(_) => String::new(),
            },
            (
                Origin,
                Url {
                    scheme, host, port, ..
                },
            ) => {
                let scheme_str = scheme
                    .as_ref()
                    .map(|s| format!("{}://", s))
                    .unwrap_or_default();

                let host_str = match host {
                    HostKind::Domain(parts) => parts.join("."),
                    HostKind::Ipv4(addr) => addr.clone(),
                    HostKind::Ipv6(addr) => addr.clone(),
                };

                let port_str = port.map(|p| format!(":{}", p)).unwrap_or_default();

                format!("{}{}{}", scheme_str, host_str, port_str)
            }
            // Segments
            (Extension, _) if !self.segments.is_empty() => {
                self.segments.last().unwrap().extension()
            }
            (Stem, _) if !self.segments.is_empty() => self.segments.last().unwrap().stem(),
            (Name, _) if !self.segments.is_empty() => self.segments.last().unwrap().clone().0,
            (FilePrefix, _) if !self.segments.is_empty() => self.segments.last().unwrap().prefix(),
            _ => "".into(),
        }
    }

    pub fn has(&self, c: Component) -> bool {
        !self.get(c).is_empty()
    }

    pub fn set(&mut self, c: Component, new_value: &str) {
        use Component::*;
        use PathKind::*;
        match (c, &self.kind) {
            // Windows
            (WindowsPrefix, Windows { .. }) => {
                if let PathKind::Windows { prefix, .. } = &mut self.kind {
                    // Use typed_path to parse the prefix, then convert to owned
                    let typed_path = Utf8TypedPath::derive(new_value);
                    if let Utf8TypedPath::Windows(win) = typed_path {
                        // Extract prefix from typed_path
                        *prefix = win.components().next().and_then(|c| {
                            if let Utf8WindowsComponent::Prefix(p) = c {
                                Some(convert_prefix_to_owned(p.kind()))
                            } else {
                                None
                            }
                        });
                    }
                }
            }
            (Disk, Windows { .. }) => {
                if let PathKind::Windows { prefix, .. } = &mut self.kind {
                    let disk_char = new_value.chars().next().unwrap_or('C');
                    *prefix = Some(self::WindowsPrefix::Disk(disk_char));
                }
            }
            (Extension, _) => {
                if !self.segments.is_empty() {
                    self.segments.last_mut().unwrap().set_extension(new_value);
                }
            }
            (Stem, _) => {
                if !self.segments.is_empty() {
                    self.segments.last_mut().unwrap().set_stem(new_value);
                }
            }
            (Name, _) if !self.segments.is_empty() => {
                self.segments.last_mut().unwrap().set_name(new_value);
            }
            (FilePrefix, _) => {
                if !self.segments.is_empty() {
                    self.segments.last_mut().unwrap().set_prefix(new_value);
                }
            }
            // URL component setters
            (Scheme, Url { .. }) => {
                if let PathKind::Url { scheme, .. } = &mut self.kind {
                    *scheme = Some(new_value.to_string());
                }
            }
            (Username, Url { .. }) => {
                if let PathKind::Url { username, .. } = &mut self.kind {
                    *username = if new_value.is_empty() {
                        None
                    } else {
                        Some(new_value.to_string())
                    };
                }
            }
            (Password, Url { .. }) => {
                if let PathKind::Url { password, .. } = &mut self.kind {
                    *password = if new_value.is_empty() {
                        None
                    } else {
                        Some(new_value.to_string())
                    };
                }
            }
            (Authority, Url { .. }) => {
                if let PathKind::Url {
                    username, password, ..
                } = &mut self.kind
                {
                    if let Some((user, pass)) = new_value.split_once(':') {
                        *username = if user.is_empty() {
                            None
                        } else {
                            Some(user.to_string())
                        };
                        *password = if pass.is_empty() {
                            None
                        } else {
                            Some(pass.to_string())
                        };
                    } else {
                        *username = if new_value.is_empty() {
                            None
                        } else {
                            Some(new_value.to_string())
                        };
                        *password = None;
                    }
                }
            }
            (Host, Url { .. }) => {
                if let PathKind::Url { host, .. } = &mut self.kind {
                    // Infer type from string format
                    *host = if new_value.contains(':') && new_value.starts_with('[') {
                        // IPv6 with brackets
                        HostKind::Ipv6(new_value.to_string())
                    } else if new_value.split('.').count() == 4
                        && new_value.split('.').all(|p| p.parse::<u8>().is_ok())
                    {
                        // IPv4 (all parts are 0-255)
                        HostKind::Ipv4(new_value.to_string())
                    } else {
                        // Domain
                        HostKind::Domain(new_value.split('.').map(|s| s.to_string()).collect())
                    };
                }
            }
            (Tld, Url { .. }) => {
                if let PathKind::Url { host, .. } = &mut self.kind {
                    match host {
                        HostKind::Domain(parts) if !parts.is_empty() => {
                            let len = parts.len();
                            parts[len - 1] = new_value.to_string();
                        }
                        _ => { /* No-op for IPv4/IPv6 */ }
                    }
                }
            }
            (Queries, Url { .. }) => {
                if let PathKind::Url { query_params, .. } = &mut self.kind {
                    *query_params = if let Some(stripped) = new_value.strip_prefix('?') {
                        stripped.split('&').map(|s| s.to_string()).collect()
                    } else if new_value.is_empty() {
                        Vec::new()
                    } else {
                        new_value.split('&').map(|s| s.to_string()).collect()
                    };
                }
            }
            (Fragment, Url { .. }) => {
                if let PathKind::Url { fragment, .. } = &mut self.kind {
                    *fragment = if new_value.is_empty() {
                        None
                    } else {
                        Some(new_value.to_string())
                    };
                }
            }
            (Path, Url { .. }) => {
                let path_str = if let Some(stripped) = new_value.strip_prefix('/') {
                    stripped
                } else {
                    new_value
                };
                self.segments = path_str
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .map(|s| Segment(s.to_string()))
                    .collect();
            }
            // URL - New components
            (Port, Url { .. }) => {
                if let PathKind::Url { port, .. } = &mut self.kind {
                    *port = if new_value.is_empty() {
                        None
                    } else {
                        new_value.parse::<u16>().ok()
                    };
                }
            }
            (IPv4, Url { .. }) => {
                if let PathKind::Url { host, .. } = &mut self.kind {
                    match host {
                        HostKind::Ipv4(_) => {
                            *host = HostKind::Ipv4(new_value.to_string());
                        }
                        _ => { /* No-op for Domain/IPv6 */ }
                    }
                }
            }
            (IPv6, Url { .. }) => {
                if let PathKind::Url { host, .. } = &mut self.kind {
                    match host {
                        HostKind::Ipv6(_) => {
                            // Ensure brackets for serialization
                            let addr = if new_value.starts_with('[') {
                                new_value.to_string()
                            } else {
                                format!("[{}]", new_value)
                            };
                            *host = HostKind::Ipv6(addr);
                        }
                        _ => { /* No-op for Domain/IPv4 */ }
                    }
                }
            }
            (IP, Url { .. }) => {
                // IP is read-only - it returns either IPv4 or IPv6
                // Setting it doesn't make sense, so this is a no-op
            }
            (Origin, Url { .. }) => {
                if let PathKind::Url {
                    scheme, host, port, ..
                } = &mut self.kind
                {
                    // Parse origin string: scheme://host:port
                    if let Some((scheme_part, rest)) = new_value.split_once("://") {
                        *scheme = Some(scheme_part.to_string());

                        // Parse host:port - handle IPv6 brackets
                        let (host_part, port_part) = if rest.starts_with('[') {
                            // IPv6 - find the closing bracket
                            if let Some(bracket_end) = rest.find(']') {
                                let ipv6_part = &rest[..=bracket_end];
                                let after_bracket = &rest[bracket_end + 1..];
                                if let Some(stripped) = after_bracket.strip_prefix(':') {
                                    (ipv6_part, Some(stripped))
                                } else {
                                    (ipv6_part, None)
                                }
                            } else {
                                (rest, None)
                            }
                        } else {
                            // Domain or IPv4 - split on last colon
                            if let Some((h, p)) = rest.rsplit_once(':') {
                                (h, Some(p))
                            } else {
                                (rest, None)
                            }
                        };

                        // Set port
                        *port = port_part.and_then(|p| p.parse::<u16>().ok());

                        // Set host (will infer type)
                        *host = if host_part.contains(':') && host_part.starts_with('[') {
                            HostKind::Ipv6(host_part.to_string())
                        } else if host_part.split('.').count() == 4
                            && host_part.split('.').all(|p| p.parse::<u8>().is_ok())
                        {
                            HostKind::Ipv4(host_part.to_string())
                        } else {
                            HostKind::Domain(host_part.split('.').map(|s| s.to_string()).collect())
                        };
                    }
                }
            }
            _ => {
                // Invalid operation (e.g., Scheme on Unix path) - no-op
            }
        }
    }

    pub fn replace(&mut self, c: Component, new_value: &str) {
        if !self.get(c).is_empty() {
            self.set(c, new_value);
        }
    }

    pub fn delete(&mut self, c: Component) {
        use Component::*;
        match c {
            // Segment operations - delete by setting to empty
            Extension => {
                if !self.segments.is_empty() {
                    let last = self.segments.last_mut().unwrap();
                    if last.len() > 1 {
                        last.0 = last.stem();
                    }
                }
            }
            Stem => {
                if !self.segments.is_empty() {
                    let last = self.segments.last_mut().unwrap();
                    if last.len() > 1 {
                        last.0 = last.extension();
                    } else {
                        last.0 = String::new();
                    }
                }
            }
            Name => {
                if !self.segments.is_empty() {
                    self.segments.pop();
                }
            }
            FilePrefix => {
                if !self.segments.is_empty() {
                    let last = self.segments.last_mut().unwrap();
                    let after = if last.len() > 1 {
                        format!(
                            ".{}",
                            last.0.split('.').skip(1).collect::<Vec<_>>().join(".")
                        )
                    } else {
                        String::new()
                    };
                    last.0 = after;
                }
            }

            // Windows operations
            Disk => {
                if let PathKind::Windows { prefix, .. } = &mut self.kind {
                    *prefix = None;
                }
            }
            WindowsPrefix => {
                if let PathKind::Windows { prefix, .. } = &mut self.kind {
                    *prefix = None;
                }
            }

            // URL operations
            Scheme => {
                if let PathKind::Url { scheme, .. } = &mut self.kind {
                    *scheme = None;
                }
            }
            Username => {
                if let PathKind::Url { username, .. } = &mut self.kind {
                    *username = None;
                }
            }
            Password => {
                if let PathKind::Url { password, .. } = &mut self.kind {
                    *password = None;
                }
            }
            Authority => {
                if let PathKind::Url {
                    username, password, ..
                } = &mut self.kind
                {
                    *username = None;
                    *password = None;
                }
            }
            Host => {
                if let PathKind::Url { host, .. } = &mut self.kind {
                    *host = HostKind::Domain(Vec::new());
                }
            }
            Tld => {
                if let PathKind::Url { host, .. } = &mut self.kind {
                    match host {
                        HostKind::Domain(parts) if !parts.is_empty() => {
                            parts.pop();
                        }
                        _ => { /* No-op for IPv4/IPv6 */ }
                    }
                }
            }
            Path => {
                self.segments.clear();
            }
            Queries => {
                if let PathKind::Url { query_params, .. } = &mut self.kind {
                    *query_params = Vec::new();
                }
            }
            Fragment => {
                if let PathKind::Url { fragment, .. } = &mut self.kind {
                    *fragment = None;
                }
            }
            // URL - New components
            Port => {
                if let PathKind::Url { port, .. } = &mut self.kind {
                    *port = None;
                }
            }
            IPv4 | IPv6 | IP => {
                // Delete host entirely
                if let PathKind::Url { host, .. } = &mut self.kind {
                    *host = HostKind::Domain(Vec::new());
                }
            }
            Origin => {
                // Clear scheme, host, port
                if let PathKind::Url {
                    scheme, host, port, ..
                } = &mut self.kind
                {
                    *scheme = None;
                    *host = HostKind::Domain(Vec::new());
                    *port = None;
                }
            }
        }
    }
}

#[cfg(test)]
use rstest_reuse;

#[cfg(test)]
mod test {
    use super::*;

    use rstest::rstest;
    use rstest_reuse::{apply, template};

    // Reusable template for path variations - used across multiple operation tests
    #[template]
    #[rstest]
    // unix
    #[case("file.stem.ext")]
    #[case("dir/file.stem.ext")]
    #[case("/file.stem.ext")]
    #[case("/dir/file.stem.ext")]
    #[case("file.ext")]
    #[case("dir/file.ext")]
    #[case("/file.ext")]
    #[case("/dir/file.ext")]
    // relative paths
    #[case(".")]
    #[case("..")]
    #[case("./dir/file.ext")]
    #[case("../dir/file.ext")]
    // unix - basic paths
    #[case("/")]
    // windows
    #[case(r"\file.stem.ext")]
    #[case(r"\dir\file.stem.ext")]
    #[case(r"\file.ext")]
    #[case(r"\dir\file.ext")]
    // windows - basic paths
    #[case(r"\")]
    // windows prefix
    #[case(r"Z:\dir\file.ext")]
    #[case(r"Z:dir\file.ext")]
    #[case(r"\\server\share")]
    #[case(r"\\server\share\file.ext")]
    #[case(r"\\?\dir\file.ext")]
    #[case(r"\\?\UNC\server\share")]
    #[case(r"\\?\Z:dir\file.ext")]
    #[case(r"\\?\Z:\dir\file.ext")]
    // windows prefix - simple variants
    #[case(r"\\.\COM42")] // DeviceNS
    #[case(r"C:\folder")]
    #[case(r"\\?\folder")]
    #[case(r"\\?\C:")]
    #[case(r"\\server\share\folder")]
    // path traversal edge cases
    #[case("in/../in/../in")]
    #[case(r"in\..\in\..\in")]
    // Future support for redundant path separators (commented for now):
    // #[case("one/./././two")]
    // #[case("//mnt")]
    // #[case("usr//bin//bash")]
    // urls - no suffix
    #[case("scheme://sub.domain.tld/dir/file.ext")]
    #[case("scheme://user@sub.domain.tld/dir/file.ext")]
    #[case("scheme://:pass@sub.domain.tld/dir/file.ext")]
    #[case("scheme://user:pass@sub.domain.tld/dir/file.ext")]
    // fragment
    #[case("scheme://sub.domain.tld/dir/file.ext#fragment")]
    #[case("scheme://user@sub.domain.tld/dir/file.ext#fragment")]
    #[case("scheme://:pass@sub.domain.tld/dir/file.ext#fragment")]
    #[case("scheme://user:pass@sub.domain.tld/dir/file.ext#fragment")]
    // query
    #[case("scheme://sub.domain.tld/dir/file.ext?key=value")]
    #[case("scheme://user@sub.domain.tld/dir/file.ext?key=value")]
    #[case("scheme://:pass@sub.domain.tld/dir/file.ext?key=value")]
    #[case("scheme://user:pass@sub.domain.tld/dir/file.ext?key=value")]
    // fragment and query
    #[case("scheme://sub.domain.tld/dir/file.ext?key=value#fragment")]
    #[case("scheme://user@sub.domain.tld/dir/file.ext?key=value#fragment")]
    #[case("scheme://:pass@sub.domain.tld/dir/file.ext?key=value#fragment")]
    #[case("scheme://user:pass@sub.domain.tld/dir/file.ext?key=value#fragment")]
    // URLs with IPv4 addresses (http needed for IP recognition)
    // TODO: write my own parser, I guess...
    #[case("http://192.168.1.1")]
    #[case("http://192.168.1.1:8080")]
    // URLs with IPv6 addresses (http needed for IP recognition)
    #[case("http://[::1]")]
    #[case("http://[::1]:8080")]
    // URLs with domain names and ports
    #[case("scheme://sub.domain.tld:8080")]
    // URLs with complex combinations
    #[case("http://user:pass@192.168.1.1:8080/dir/file.ext?key=value#fragment")]
    #[case("http://user@[::1]:8080/dir/file.ext?key=value#fragment")]
    #[case("scheme://user:pass@sub.domain.tld:8080/dir/file.ext?key=value#fragment")]
    fn path_variations(#[case] path: &str) {}

    #[apply(path_variations)]
    fn identity(path: &str) {
        let p = Path::parse(path);
        assert_eq!(path.to_string(), p.clone().serialize(), "{:?}", p);
    }

    #[apply(path_variations)]
    fn can_get(path: &str) {
        let p = Path::parse(path);
        // segments
        if path.contains("ext") {
            assert_eq!(p.get(Component::Extension), "ext", "{:?}", p);
        }
        if path.contains("stem") {
            assert_eq!(p.get(Component::Stem), "file.stem", "{:?}", p);
        }
        if path.contains("file") {
            if path.contains("stem") {
                assert_eq!(p.get(Component::Name), "file.stem.ext", "{:?}", p);
            } else {
                assert_eq!(p.get(Component::Name), "file.ext", "{:?}", p);
            }
            assert_eq!(p.get(Component::FilePrefix), "file", "{:?}", p);
        }
        // windows prefix
        if path.contains("Z") {
            assert_eq!(p.get(Component::Disk), "Z", "{:?}", p);
        }
        if path.contains("server") {
            if path.contains("UNC") {
                assert_eq!(
                    p.get(Component::WindowsPrefix),
                    r"\\?\UNC\server\share",
                    "{:?}",
                    p
                );
            } else {
                assert_eq!(
                    p.get(Component::WindowsPrefix),
                    r"\\server\share",
                    "{:?}",
                    p
                );
            }
        }

        // url
        if path.contains("scheme") {
            assert_eq!(p.get(Component::Scheme), "scheme");
        }
        if path.contains("user") {
            assert_eq!(p.get(Component::Username), "user");
        }
        if path.contains("pass") {
            assert_eq!(p.get(Component::Password), "pass");
        }
        match (path.contains("user"), path.contains("pass")) {
            (true, true) => assert_eq!(p.get(Component::Authority), "user:pass"),
            (true, false) => assert_eq!(p.get(Component::Authority), "user"),
            (false, true) => assert_eq!(p.get(Component::Authority), ":pass"),
            (false, false) => assert_eq!(p.get(Component::Authority), ""),
        }
        if path.contains("domain") {
            assert!(p.get(Component::Host).contains("domain"));
        }
        if path.contains("tld") {
            assert_eq!(p.get(Component::Tld), "tld", "{:?}", p);
        }
        if path.contains("key") {
            assert_eq!(p.get(Component::Queries), "?key=value");
        }
        if path.contains("fragment") {
            assert_eq!(p.get(Component::Fragment), "fragment");
        }
        // Port assertions
        if path.contains(":8080") {
            assert_eq!(p.get(Component::Port), "8080", "{:?}", p);
        }
        // IPv4 assertions
        if path.contains("192.168.1.1") {
            assert_eq!(p.get(Component::IPv4), "192.168.1.1", "{:?}", p);
            assert_eq!(p.get(Component::Host), "192.168.1.1", "{:?}", p);
            assert_eq!(p.get(Component::Tld), "", "{:?}", p); // No TLD for IPs
        }
        // IPv6 assertions
        if path.contains("[::1]") {
            assert_eq!(p.get(Component::IPv6), "[::1]", "{:?}", p);
            assert_eq!(p.get(Component::Host), "[::1]", "{:?}", p);
            assert_eq!(p.get(Component::Tld), "", "{:?}", p); // No TLD for IPs
        }
        // Origin assertions (scheme + host + port)
        if path.contains("scheme://sub.domain.tld:8080") {
            assert_eq!(
                p.get(Component::Origin),
                "scheme://sub.domain.tld:8080",
                "{:?}",
                p
            );
        }
    }

    #[apply(path_variations)]
    fn can_set(path: &str) {
        let original = Path::parse(path);

        // Test segment components
        if path.contains("ext") {
            let mut p = original.clone();
            p.set(Component::Extension, "NEW");
            assert_eq!(p.get(Component::Extension), "NEW", "{:?}", p);
            assert!(
                !p.serialize().contains("ext"),
                "Should not contain 'ext': {}",
                p.serialize()
            );
        }

        if path.contains("stem") {
            let mut p = original.clone();
            p.set(Component::Stem, "NEWSTEM");
            assert_eq!(p.get(Component::Stem), "NEWSTEM", "{:?}", p);
        }

        if path.contains("file") {
            let mut p = original.clone();
            p.set(Component::Name, "NEWNAME");
            assert_eq!(p.get(Component::Name), "NEWNAME", "{:?}", p);

            let mut p = original.clone();
            p.set(Component::FilePrefix, "NEWPREFIX");
            assert_eq!(p.get(Component::FilePrefix), "NEWPREFIX", "{:?}", p);
        }

        // Test Windows components
        if path.contains("Z:") || path.contains("Z") {
            let mut p = original.clone();
            p.set(Component::Disk, "X");
            assert_eq!(p.get(Component::Disk), "X", "{:?}", p);
        }

        if path.contains("server") {
            let mut p = original.clone();
            let new_prefix = if path.contains("UNC") {
                r"\\?\UNC\newserver\newshare"
            } else {
                r"\\newserver\newshare"
            };
            p.set(Component::WindowsPrefix, new_prefix);
            assert_eq!(p.get(Component::WindowsPrefix), new_prefix, "{:?}", p);
        }

        // Test URL components
        if path.contains("scheme") {
            let mut p = original.clone();
            p.set(Component::Scheme, "ftp");
            assert_eq!(p.get(Component::Scheme), "ftp", "{:?}", p);
            assert!(
                !p.serialize().contains("scheme://"),
                "Should not contain 'scheme://': {}",
                p.serialize()
            );
        }

        if path.contains("user") {
            let mut p = original.clone();
            p.set(Component::Username, "admin");
            assert_eq!(p.get(Component::Username), "admin", "{:?}", p);
        }

        if path.contains("pass") {
            let mut p = original.clone();
            p.set(Component::Password, "newpass");
            assert_eq!(p.get(Component::Password), "newpass", "{:?}", p);
        }

        if path.contains("domain") {
            let mut p = original.clone();
            p.set(Component::Host, "newhost.com");
            assert_eq!(p.get(Component::Host), "newhost.com", "{:?}", p);
        }

        if path.contains("tld") {
            let mut p = original.clone();
            p.set(Component::Tld, "org");
            assert_eq!(p.get(Component::Tld), "org", "{:?}", p);
        }

        if path.contains("key") {
            let mut p = original.clone();
            p.set(Component::Queries, "new=query");
            assert_eq!(p.get(Component::Queries), "?new=query", "{:?}", p);
        }

        if path.contains("fragment") {
            let mut p = original.clone();
            p.set(Component::Fragment, "newfragment");
            assert_eq!(p.get(Component::Fragment), "newfragment", "{:?}", p);
        }
    }

    #[apply(path_variations)]
    fn can_delete(path: &str) {
        let original = Path::parse(path);

        // Test segment components
        // For segments, deleting a component doesn't mean the result won't have that component type
        // Instead, verify the delete operation changes the path
        if path.contains("ext") {
            let mut p = original.clone();
            let before = p.serialize();
            p.delete(Component::Extension);
            let after = p.serialize();
            // Deletion should change the path
            assert_ne!(before, after, "Delete should change path for {:?}", p);
            assert!(
                !after.contains(".ext"),
                "Should not contain '.ext': {}",
                after
            );
        }

        if path.contains("file") {
            let mut p = original.clone();
            let before = p.serialize();
            p.delete(Component::Name);
            let after = p.serialize();
            // Deleting name should change the path
            if !before.ends_with('/') && !before.ends_with('\\') {
                // Only check if it's not already a directory path
                assert_ne!(
                    before, after,
                    "Delete name should change path for {:?}",
                    original
                );
            }
        }

        // Test Windows components - these should actually clear when deleted
        if path.contains("Z:") || path.contains("Z") {
            let mut p = original.clone();
            p.delete(Component::Disk);
            assert_eq!(p.get(Component::Disk), "", "{:?}", p);
        }

        if path.contains("server") {
            let mut p = original.clone();
            p.delete(Component::WindowsPrefix);
            // After deleting Windows prefix, it should be gone
            let serialized = p.serialize();
            assert!(
                !serialized.contains("server"),
                "Should not contain 'server': {}",
                serialized
            );
        }

        // Test URL components - these SHOULD return empty after deletion
        if path.contains("scheme") {
            let mut p = original.clone();
            p.delete(Component::Scheme);
            assert_eq!(p.get(Component::Scheme), "", "{:?}", p);
        }

        if path.contains("user") {
            let mut p = original.clone();
            p.delete(Component::Username);
            assert_eq!(p.get(Component::Username), "", "{:?}", p);
        }

        if path.contains("pass") {
            let mut p = original.clone();
            p.delete(Component::Password);
            assert_eq!(p.get(Component::Password), "", "{:?}", p);
        }

        if path.contains("domain") {
            let mut p = original.clone();
            let before_host = p.get(Component::Host);
            p.delete(Component::Host);
            let after_host = p.get(Component::Host);
            // Deleting host should change it (but might not make it empty if there are path segments)
            assert_ne!(
                before_host, after_host,
                "Delete should change host for {:?}",
                p
            );
        }

        if path.contains("tld") {
            let mut p = original.clone();
            let before_tld = p.get(Component::Tld);
            p.delete(Component::Tld);
            let after_tld = p.get(Component::Tld);
            // Deleting TLD from "sub.domain.tld" gives "sub.domain" which has TLD "domain"
            // So we verify it changed, not that it's empty
            assert_ne!(
                before_tld, after_tld,
                "Delete should change TLD for {:?}",
                p
            );
        }

        if path.contains("key") {
            let mut p = original.clone();
            p.delete(Component::Queries);
            assert_eq!(p.get(Component::Queries), "", "{:?}", p);
        }

        if path.contains("fragment") {
            let mut p = original.clone();
            p.delete(Component::Fragment);
            assert_eq!(p.get(Component::Fragment), "", "{:?}", p);
            assert!(
                !p.serialize().contains("#fragment"),
                "Should not contain '#fragment': {}",
                p.serialize()
            );
        }
    }

    #[apply(path_variations)]
    fn can_replace(path: &str) {
        let original = Path::parse(path);

        // Test replace succeeds when component exists
        if path.contains("ext") {
            let mut p = original.clone();
            p.replace(Component::Extension, "REPLACED");
            assert_eq!(p.get(Component::Extension), "REPLACED", "{:?}", p);
        }

        if path.contains("stem") {
            let mut p = original.clone();
            p.replace(Component::Stem, "REPLACED");
            assert_eq!(p.get(Component::Stem), "REPLACED", "{:?}", p);
        }

        if path.contains("file") {
            let mut p = original.clone();
            p.replace(Component::Name, "REPLACED");
            assert_eq!(p.get(Component::Name), "REPLACED", "{:?}", p);
        }

        if path.contains("Z:") || path.contains("Z") {
            let mut p = original.clone();
            p.replace(Component::Disk, "X");
            assert_eq!(p.get(Component::Disk), "X", "{:?}", p);
        }

        if path.contains("scheme") {
            let mut p = original.clone();
            p.replace(Component::Scheme, "ftp");
            assert_eq!(p.get(Component::Scheme), "ftp", "{:?}", p);
        }

        if path.contains("user") {
            let mut p = original.clone();
            p.replace(Component::Username, "admin");
            assert_eq!(p.get(Component::Username), "admin", "{:?}", p);
        }

        if path.contains("fragment") {
            let mut p = original.clone();
            p.replace(Component::Fragment, "REPLACED");
            assert_eq!(p.get(Component::Fragment), "REPLACED", "{:?}", p);
        }

        // Test replace is no-op when component doesn't exist
        // For unix/windows paths without URLs, URL components should not be added
        if !path.contains("scheme") && !path.contains("://") {
            let mut p = original.clone();
            let before = p.serialize();
            p.replace(Component::Scheme, "SHOULD_NOT_APPEAR");
            assert_eq!(
                p.serialize(),
                before,
                "Replace should be no-op when component missing"
            );
            assert_eq!(p.get(Component::Scheme), "", "{:?}", p);
        }

        if !path.contains("fragment") {
            let mut p = original.clone();
            let before = p.serialize();
            p.replace(Component::Fragment, "SHOULD_NOT_APPEAR");
            assert_eq!(
                p.serialize(),
                before,
                "Replace should be no-op when component missing"
            );
            assert_eq!(p.get(Component::Fragment), "", "{:?}", p);
        }
    }

    #[apply(path_variations)]
    fn can_has(path: &str) {
        let p = Path::parse(path);

        // Verify has() returns true for present components
        // Extension exists if path contains ".ext" or ".stem.ext"
        if path.contains("ext") {
            assert!(p.has(Component::Extension), "{:?}", p);
        }

        // Stem/Name/FilePrefix exist for most file paths except "." and ".."
        // Just verify they work correctly, don't assert they should be missing
        if path != "." && path != ".." {
            // Most paths have a name component
            if path.contains("file")
                || path.ends_with(".ext")
                || path.contains(r"\")
                || path.contains("/")
            {
                // If it looks like it has file components, verify has() works
                let has_name = p.has(Component::Name);
                let get_name = p.get(Component::Name);
                assert_eq!(
                    has_name,
                    !get_name.is_empty(),
                    "has() and get() should be consistent for Name in {:?}",
                    p
                );
            }
        }

        // Windows components
        if path.contains("Z:") || path.contains("Z") {
            assert!(p.has(Component::Disk), "{:?}", p);
        }

        if path.contains("server") {
            assert!(p.has(Component::WindowsPrefix), "{:?}", p);
        }

        // URL components
        if path.contains("scheme") {
            assert!(p.has(Component::Scheme), "{:?}", p);
        }

        if path.contains("user") {
            assert!(p.has(Component::Username), "{:?}", p);
        }

        if path.contains("pass") {
            assert!(p.has(Component::Password), "{:?}", p);
        }

        if path.contains("domain") {
            assert!(p.has(Component::Host), "{:?}", p);
        }

        if path.contains("tld") {
            assert!(p.has(Component::Tld), "{:?}", p);
        }

        if path.contains("key") {
            assert!(p.has(Component::Queries), "{:?}", p);
        }

        if path.contains("fragment") {
            assert!(p.has(Component::Fragment), "{:?}", p);
        }

        // Test the relationship: has() should be true iff get() is non-empty
        // Do this for all components to ensure consistency
        assert_eq!(
            p.has(Component::Extension),
            !p.get(Component::Extension).is_empty(),
            "has(Extension) should match !get(Extension).is_empty() for {:?}",
            p
        );
        assert_eq!(
            p.has(Component::Scheme),
            !p.get(Component::Scheme).is_empty(),
            "has(Scheme) should match !get(Scheme).is_empty() for {:?}",
            p
        );
        assert_eq!(
            p.has(Component::Fragment),
            !p.get(Component::Fragment).is_empty(),
            "has(Fragment) should match !get(Fragment).is_empty() for {:?}",
            p
        );
    }

    #[ignore]
    #[test]
    fn hueristic() {
        // if it contains ://, it's a url
        assert!(Path::parse("s://asdf").is_url());
        // if it contains a back slash and no forward slashes, assume windows
        assert!(Path::parse(r"dir\dir2").is_windows());
        // first segment contains dots, and isn't relative, assume URL
        assert!(Path::parse(r"github.com/rutrum").is_url());
        // if the first segment has no schema/authority and is relative, assume unix
        assert!(Path::parse(r"../rutrum").is_unix());
        assert!(Path::parse(r"./rutrum").is_unix());
    }

    // URL manipulation tests
    mod url_tests {
        use super::*;

        #[test]
        fn test_url_empty_components() {
            let mut path = Path::parse("https://user@example.com");
            assert_eq!(path.get(Component::Username), "user");
            assert_eq!(path.get(Component::Password), "");

            path.set(Component::Username, "");
            assert_eq!(path.get(Component::Username), "");
        }

        #[test]
        fn test_url_multiple_operations() {
            let mut path = Path::parse("http://api.example.com/v1/users?page=1#section");

            path.set(Component::Scheme, "https");
            path.set(Component::Tld, "org");
            path.set(Component::Queries, "limit=50");
            path.delete(Component::Fragment);

            let result = path.serialize();
            assert!(result.starts_with("https://"));
            assert!(result.contains(".org"));
            assert!(result.contains("limit=50"));
            assert!(!result.contains("section"));
        }
    }

    mod url_component_tests {
        use super::*;

        #[test]
        fn tld_empty_for_ipv4() {
            let p = Path::parse("http://192.168.1.1:8080/path");
            assert_eq!(p.get(Component::Tld), "");
            assert_eq!(p.get(Component::IPv4), "192.168.1.1");
            assert_eq!(p.get(Component::Port), "8080");
        }

        #[test]
        fn tld_empty_for_ipv6() {
            let p = Path::parse("http://[::1]:8080");
            assert_eq!(p.get(Component::Tld), "");
            assert_eq!(p.get(Component::IPv6), "[::1]");
            assert_eq!(p.get(Component::Port), "8080");
        }

        #[test]
        fn port_explicit_only() {
            let p1 = Path::parse("scheme://sub.domain.tld"); // No port
            assert_eq!(p1.get(Component::Port), "");

            let p2 = Path::parse("scheme://sub.domain.tld:8080"); // Explicit port
            assert_eq!(p2.get(Component::Port), "8080");
        }

        #[test]
        fn ip_component_returns_either_type() {
            let p1 = Path::parse("http://192.168.1.1");
            assert_eq!(p1.get(Component::IP), "192.168.1.1");

            let p2 = Path::parse("http://[::1]");
            assert_eq!(p2.get(Component::IP), "[::1]");

            let p3 = Path::parse("scheme://sub.domain.tld");
            assert_eq!(p3.get(Component::IP), "");
        }

        #[test]
        fn origin_combines_scheme_host_port() {
            let p = Path::parse("scheme://sub.domain.tld:8080/dir/file.ext?key=value#fragment");
            assert_eq!(p.get(Component::Origin), "scheme://sub.domain.tld:8080");
        }
    }
}
