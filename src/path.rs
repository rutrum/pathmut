// experiment:
// build a data structure that deserialize and re-serialize any URL or Path
// without losing any information

use typed_path::{
    Utf8TypedPath, Utf8UnixComponent, Utf8WindowsComponent, Utf8WindowsPrefix,
    Utf8WindowsPrefixComponent,
};

use url::{Host, ParseError, Url};

use std::borrow::Borrow;

#[derive(Debug, Clone)]
pub struct Path<'s> {
    segments: Vec<Segment>,
    kind: PathKind<'s>,
}

#[derive(Debug, Clone)]
enum PathKind<'s> {
    Unix {
        root: bool,
    },
    Windows {
        prefix: Option<WindowsPrefix<'s>>, // TODO: make my own prefix type
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
enum WindowsPrefix<'s> {
    Verbatim(&'s str),
    VerbatimUNC(&'s str, &'s str),
    VerbatimDisk(char),
    DeviceNS(&'s str),
    UNC(&'s str, &'s str),
    Disk(char),
}

impl WindowsPrefix<'_> {
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

impl Path<'_> {
    pub fn is_unix(self) -> bool {
        matches!(self.kind, PathKind::Unix { .. })
    }

    pub fn is_windows(self) -> bool {
        matches!(self.kind, PathKind::Windows { .. })
    }

    pub fn is_url(self) -> bool {
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

    pub fn parse_as_url<'a>(path_str: &'a str) -> Result<(Path<'a>, bool), ParseError> {
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
                                Utf8WindowsPrefix::Verbatim(s) => WindowsPrefix::Verbatim(s),
                                Utf8WindowsPrefix::VerbatimUNC(s, t) => {
                                    WindowsPrefix::VerbatimUNC(s, t)
                                }
                                Utf8WindowsPrefix::VerbatimDisk(s) => {
                                    WindowsPrefix::VerbatimDisk(s)
                                }
                                Utf8WindowsPrefix::DeviceNS(s) => WindowsPrefix::DeviceNS(s),
                                Utf8WindowsPrefix::UNC(s, t) => WindowsPrefix::UNC(s, t),
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

impl Path<'_> {
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
            (
                Prefix,
                Windows {
                    prefix: Some(p), ..
                },
            ) => {
                // somehow parse new_value as a prefix
            }
            (
                Disk,
                Windows {
                    prefix: Some(p),
                    root,
                },
            ) => {
                self.kind = Windows {
                    // redo this too
                    // should disk be restricted to a character?
                    prefix: Some(WindowsPrefix::Disk(new_value.chars().next().unwrap())),
                    root: *root,
                };
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
                    self.segments.last_mut().unwrap().set_stem(new_value);
                }
            }
            _ => todo!(),
        }
    }

    pub fn replace(&mut self, c: Component, new_value: &str) {
        if !self.get(c).is_empty() {
            self.set(c, new_value);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use rstest::rstest;

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
    // #[case(r"dir\file.stem.ext")]  // parsed as unix
    #[case(r"\file.stem.ext")]
    #[case(r"\dir\file.stem.ext")]
    // #[case(r"dir\file.ext")] // parsed as unix
    #[case(r"\file.ext")]
    #[case(r"\dir\file.ext")]
    // these get parsed as unix paths, shouldn't they be urls?  I can check for . in top folder and not root
    //#[case("sub.domain.tld")]
    //#[case("sub.domain.tld/file.ext")]
    //#[case("sub.domain.tld/dir/file.ext")]
    // windows prefix
    #[case(r"Z:\dir\file.ext")]
    #[case(r"Z:dir\file.ext")]
    #[case(r"\\server\share")]
    #[case(r"\\server\share\file.ext")]
    #[case(r"\\?\dir\file.ext")]
    #[case(r"\\?\UNC\server\share")]
    #[case(r"\\?\Z:dir\file.ext")]
    // todo: DeviceNS, that's weird though, since it depends on fixed number of devices
    // I dont really want to support that
    // no suffix
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
    fn can_get(#[case] path: &str) {
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
}
