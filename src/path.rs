// experiment:
// build a data structure that deserialize and re-serialize any URL or Path
// without losing any information

use typed_path::{Utf8TypedPath, Utf8UnixComponent, Utf8WindowsComponent, Utf8WindowsPrefix};

use url::{Host, ParseError, Url};

use std::borrow::Borrow;

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
        host: Vec<String>,
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
    UNC(String, String),
    Disk(char),
}

impl WindowsPrefix {
    fn to_string(&self) -> String {
        match self {
            WindowsPrefix::Verbatim(path) => format!(r"\\?\{}", path),
            WindowsPrefix::VerbatimUNC(server, share) => format!(r"\\?\UNC\{}\{}", server, share),
            WindowsPrefix::VerbatimDisk(drive) => format!(r"\\?\{}:", drive),
            WindowsPrefix::DeviceNS(name) => format!(r"\\.\{}", name),
            WindowsPrefix::UNC(server, share) => format!(r"\\{}\{}", server, share),
            WindowsPrefix::Disk(drive) => format!("{}:", drive),
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
            .unwrap_or(String::new())
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
            WindowsPrefix::UNC(server.to_string(), share.to_string())
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
                .filter(|x| x.len() > 0)
                .map(Segment)
                .collect(),
            Option::None => Vec::new(),
        };
        let username = if url.username().is_empty() {
            None
        } else {
            Some(url.username().to_owned())
        };
        Ok((
            Path {
                segments,
                kind: PathKind::Url {
                    scheme,
                    username,
                    password: url.password().map(|s| s.to_owned()),
                    host: url
                        .host()
                        .map(|h| match h {
                            Host::Domain(d) => d.split(".").map(|s| s.to_string()).collect(),
                            _ => Vec::new(),
                        })
                        .unwrap_or(Vec::new()),
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
                                    WindowsPrefix::UNC(s.to_string(), t.to_string())
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
                    format!("{}{}{}", p.to_string(), left, right)
                } else {
                    format!("{}{}", left, right)
                }
            }
            PathKind::Url {
                scheme,
                username,
                password,
                host,
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
                ss.push(host.join("."));
                if self.segments.len() > 0 {
                    ss.push("/".to_owned());
                    ss.push(self.segments.join("/"));
                }
                if query_params.len() > 0 {
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
    Prefix,

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
}

impl Path {
    pub fn get(&self, c: Component) -> String {
        use Component::*;
        use PathKind::*;
        match (c, &self.kind) {
            // windows
            (
                Prefix,
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
                use WindowsPrefix::*;
                match p {
                    VerbatimDisk(disk) => (*disk).into(),
                    Disk(disk) => (*disk).into(),
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
                (Some(u), None) => format!("{u}"),
                (None, None) => format!(""),
            },
            (Host, Url { host, .. }) => host.join("."),
            (Tld, Url { host, .. }) => match host.len() {
                0 | 1 => "".into(),
                _ => host[host.len() - 1].clone(),
            },
            (
                Queries,
                Url {
                    query_params: qps, ..
                },
            ) if qps.len() > 0 => format!("?{}", qps.join("&")),
            (
                Fragment,
                Url {
                    fragment: Some(s), ..
                },
            ) => s.to_string(),
            // Segments
            (Extension, _) if self.segments.len() > 0 => self.segments.last().unwrap().extension(),
            (Stem, _) if self.segments.len() > 0 => self.segments.last().unwrap().stem(),
            (Name, _) if self.segments.len() > 0 => self.segments.last().unwrap().clone().0,
            (FilePrefix, _) if self.segments.len() > 0 => self.segments.last().unwrap().prefix(),
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
            (Prefix, Windows { .. }) => {
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
                    *prefix = Some(WindowsPrefix::Disk(disk_char));
                }
            }
            (Extension, _) => {
                if self.segments.len() > 0 {
                    self.segments.last_mut().unwrap().set_extension(new_value);
                }
            }
            (Stem, _) => {
                if self.segments.len() > 0 {
                    self.segments.last_mut().unwrap().set_stem(new_value);
                }
            }
            (Name, _) if self.segments.len() > 0 => {
                self.segments.last_mut().unwrap().set_name(new_value);
            }
            (FilePrefix, _) => {
                if self.segments.len() > 0 {
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
                    *host = new_value.split('.').map(|s| s.to_string()).collect();
                }
            }
            (Tld, Url { .. }) => {
                if let PathKind::Url { host, .. } = &mut self.kind {
                    if host.len() > 0 {
                        let len = host.len();
                        host[len - 1] = new_value.to_string();
                    }
                }
            }
            (Queries, Url { .. }) => {
                if let PathKind::Url { query_params, .. } = &mut self.kind {
                    *query_params = if new_value.starts_with('?') {
                        new_value[1..].split('&').map(|s| s.to_string()).collect()
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
                let path_str = if new_value.starts_with('/') {
                    &new_value[1..]
                } else {
                    new_value
                };
                self.segments = path_str
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .map(|s| Segment(s.to_string()))
                    .collect();
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
                if self.segments.len() > 0 {
                    let last = self.segments.last_mut().unwrap();
                    if last.len() > 1 {
                        last.0 = last.stem();
                    }
                }
            }
            Stem => {
                if self.segments.len() > 0 {
                    let last = self.segments.last_mut().unwrap();
                    if last.len() > 1 {
                        last.0 = last.extension();
                    } else {
                        last.0 = String::new();
                    }
                }
            }
            Name => {
                if self.segments.len() > 0 {
                    self.segments.pop();
                }
            }
            FilePrefix => {
                if self.segments.len() > 0 {
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
            Prefix => {
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
                    *host = Vec::new();
                }
            }
            Tld => {
                if let PathKind::Url { host, .. } = &mut self.kind {
                    if host.len() > 0 {
                        host.pop();
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
    #[case("././../dir/../file.ext")]
    // windows
    #[case(r"\file.stem.ext")]
    #[case(r"\dir\file.stem.ext")]
    #[case(r"\file.ext")]
    #[case(r"\dir\file.ext")]
    // windows prefix
    #[case(r"Z:\dir\file.ext")]
    #[case(r"Z:dir\file.ext")]
    #[case(r"\\server\share")]
    #[case(r"\\server\share\file.ext")]
    #[case(r"\\?\dir\file.ext")]
    #[case(r"\\?\UNC\server\share")]
    #[case(r"\\?\Z:dir\file.ext")]
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
    fn path_variations(#[case] path: &str) {}

    #[rstest]
    // UNIX PATHS

    // root, path segments, file parts
    #[case("/path/to/file.txt")]
    #[case("file")]
    #[case("file.txt")]
    #[case("/")]
    #[case("/folder")]
    #[case("/path/to/file.tar.gz")]
    // relative paths
    #[case(".")]
    #[case("./folder")]
    #[case("..")]
    #[case("../folder")]
    #[case("../path/to/file.txt")]
    #[case("in/../in/../in")]
    // redundant current directory: see note below
    //#[case("one/./././two")]
    // double separators: these are _very_ deep in the typed path module
    // there is no way to modify this behavior.  To support this, I would need
    // to write my own parser
    //#[case("//mnt")]
    //#[case("usr//bin//bash")]

    // WINDOWS PATHS
    #[case(r"\path\to\file.txt")]
    #[case(r"\file.txt")]
    #[case(r"\folder")]
    #[case(r"\")]
    #[case(r"path\to\file.txt")] // this may not be parsed as windows
    // relative
    #[case(r".\folder")]
    #[case(r"..\folder")]
    #[case(r"..\path\to\file.txt")]
    #[case(r"in\..\in\..\in")]
    // prefixes
    // these may not be parsing correctly, but it may not matter
    // not sure I want to provide a way to change these, besides disk
    #[case(r"C:\folder")]
    #[case(r"C:folder")]
    #[case(r"\\?\folder")]
    #[case(r"\\?\UNC\server\share")]
    #[case(r"\\?\C:")]
    #[case(r"\\.\COM42")]
    #[case(r"\\server\share")]
    // URLS
    // domains
    #[case("github.com")]
    #[case("rutrum.github.io")]
    #[case("machine.left-right.ts.net")]
    // schemes
    #[case("https://github.com")]
    #[case("smtp://gmail.com")]
    // authority
    #[case("ssh://user:pass@github.com")]
    #[case("ssh://user@github.com")]
    #[case("ssh://:pass@github.com")]
    // paths
    #[case("file:///path/to/file")]
    #[case("file:///file.tar.gz")]
    // query_params, fragments
    #[case("http://example.com/path/file.csv#a=5")]
    #[case("http://example.com/path/file.csv?a=5")]
    #[case("http://example.com/path/file.csv?a=5&b=2")]
    #[case("http://example.com/path/file.csv?a=5&b=2#f=3")]
    #[case("/path/file.csv#a=5")]
    #[case("/path/file.csv?a=5")]
    #[case("/path/file.csv?a=5&b=2")]
    #[case("/path/file.csv?a=5&b=2#f=3")]
    fn identity(#[case] path: &str) {
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
                assert_eq!(p.get(Component::Prefix), r"\\?\UNC\server\share", "{:?}", p);
            } else {
                assert_eq!(p.get(Component::Prefix), r"\\server\share", "{:?}", p);
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
            p.set(Component::Prefix, new_prefix);
            assert_eq!(p.get(Component::Prefix), new_prefix, "{:?}", p);
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
            p.delete(Component::Prefix);
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
            assert!(p.has(Component::Prefix), "{:?}", p);
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

    #[rstest]
    fn can_set_extension(
        #[values("scheme://user:pass@sub.domain.tld/dir/file.ext?key=value#fragment")] path: &str,
    ) {
        let q = Path::parse(path);
        let new_value = "APPLE";

        if path.contains("ext") {
            let mut p = q.clone();
            p.set(Component::Extension, new_value);
            assert_eq!(p.get(Component::Extension), new_value, "{:?}", p);
            let new_path = p.clone().serialize();
            assert!(!new_path.contains("ext"), "{}", new_path);
        }
        if path.contains("file") {
            let mut p = q.clone();
            p.set(Component::FilePrefix, new_value);
            assert_eq!(p.get(Component::FilePrefix), new_value, "{:?}", p);
            let new_path = p.clone().serialize();
            assert!(!new_path.contains("file"), "{}", new_path);

            let mut p = q.clone();
            p.set(Component::Stem, new_value);
            assert_eq!(p.get(Component::Stem), new_value, "{:?}", p);
            let new_path = p.clone().serialize();
            assert!(!new_path.contains("file"), "{}", new_path);

            let mut p = q.clone();
            p.set(Component::Name, new_value);
            assert_eq!(p.get(Component::Name), new_value, "{:?}", p);
            let new_path = p.clone().serialize();
            assert!(!new_path.contains("file"), "{}", new_path);
        }
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
        fn test_url_scheme_get_set() {
            let mut path = Path::parse("https://example.com");
            assert_eq!(path.get(Component::Scheme), "https");
            path.set(Component::Scheme, "http");
            assert_eq!(path.serialize(), "http://example.com");
        }

        #[test]
        fn test_url_host_get_set() {
            let mut path = Path::parse("https://api.github.com");
            assert_eq!(path.get(Component::Host), "api.github.com");
            path.set(Component::Host, "gitlab.com");
            assert_eq!(path.serialize(), "https://gitlab.com");
        }

        #[test]
        fn test_url_tld_get_set() {
            let mut path = Path::parse("https://example.com");
            assert_eq!(path.get(Component::Tld), "com");
            path.set(Component::Tld, "org");
            assert_eq!(path.serialize(), "https://example.org");
        }

        #[test]
        fn test_url_queries_get_set() {
            let mut path = Path::parse("https://example.com?page=1&limit=10");
            assert_eq!(path.get(Component::Queries), "?page=1&limit=10");
            path.set(Component::Queries, "page=2");
            assert_eq!(path.get(Component::Queries), "?page=2");
        }

        #[test]
        fn test_url_fragment_get_set() {
            let mut path = Path::parse("https://example.com#section");
            assert_eq!(path.get(Component::Fragment), "section");
            path.set(Component::Fragment, "header");
            assert_eq!(path.serialize(), "https://example.com#header");
        }

        #[test]
        fn test_url_username_password() {
            let mut path = Path::parse("https://user:pass@example.com");
            assert_eq!(path.get(Component::Username), "user");
            assert_eq!(path.get(Component::Password), "pass");
            path.set(Component::Username, "admin");
            assert_eq!(path.get(Component::Authority), "admin:pass");
        }

        #[test]
        fn test_url_delete_components() {
            let mut path = Path::parse("https://user:pass@example.com/path?query#fragment");
            path.delete(Component::Fragment);
            assert_eq!(path.get(Component::Fragment), "");
            path.delete(Component::Queries);
            assert_eq!(path.get(Component::Queries), "");
            path.delete(Component::Username);
            assert_eq!(path.get(Component::Username), "");
        }

        #[test]
        fn test_url_path_segments() {
            let mut path = Path::parse("https://example.com/api/v1/users");
            assert_eq!(path.get(Component::Name), "users");
            path.set(Component::Name, "repos");
            assert_eq!(path.serialize(), "https://example.com/api/v1/repos");
        }

        #[test]
        fn test_url_extension_on_path() {
            let mut path = Path::parse("https://example.com/file.json");
            assert_eq!(path.get(Component::Extension), "json");
            path.set(Component::Extension, "xml");
            assert_eq!(path.serialize(), "https://example.com/file.xml");
        }

        #[test]
        fn test_url_authority_parsing() {
            let mut path = Path::parse("https://example.com");
            path.set(Component::Authority, "user:password");
            assert_eq!(path.get(Component::Username), "user");
            assert_eq!(path.get(Component::Password), "password");

            path.set(Component::Authority, "user");
            assert_eq!(path.get(Component::Username), "user");
            assert_eq!(path.get(Component::Password), "");
        }

        #[test]
        fn test_url_replace() {
            let mut path = Path::parse("https://example.com");
            path.replace(Component::Fragment, "section");
            assert_eq!(path.get(Component::Fragment), ""); // replace only works if exists

            path.set(Component::Fragment, "intro");
            path.replace(Component::Fragment, "conclusion");
            assert_eq!(path.get(Component::Fragment), "conclusion");
        }

        #[test]
        fn test_url_has() {
            let path = Path::parse("https://user:pass@example.com/path?query#fragment");
            assert!(path.has(Component::Scheme));
            assert!(path.has(Component::Username));
            assert!(path.has(Component::Password));
            assert!(path.has(Component::Host));
            assert!(path.has(Component::Queries));
            assert!(path.has(Component::Fragment));

            let minimal = Path::parse("https://example.com");
            assert!(!minimal.has(Component::Username));
            assert!(!minimal.has(Component::Fragment));
        }

        #[test]
        fn test_url_path_component() {
            let mut path = Path::parse("https://example.com/old/path");
            path.set(Component::Path, "/new/path/here");
            assert_eq!(path.serialize(), "https://example.com/new/path/here");
        }

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
}
