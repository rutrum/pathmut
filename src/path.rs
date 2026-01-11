// experiment:
// build a data structure that deserialize and re-serialize any URL or Path
// without losing any information

use typed_path::{
    Utf8TypedPath, Utf8UnixComponent, Utf8WindowsComponent, Utf8WindowsPrefixComponent,
};

use url::{Host, ParseError, Url};

#[derive(Debug, Clone)]
pub struct Path<'s> {
    segments: Vec<String>,
    kind: PathKind<'s>,
}

#[derive(Debug, Clone)]
enum PathKind<'s> {
    Unix {
        root: bool,
    },
    Windows {
        prefix: Option<Utf8WindowsPrefixComponent<'s>>,
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

impl Path<'_> {
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
            Some(segs) => segs.map(|s| s.to_owned()).filter(|x| x.len() > 0).collect(),
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
                        segments.push(s.to_string())
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
                        Prefix(p) => prefix = Some(p),
                    };
                    if let Some(s) = segment {
                        segments.push(s.to_string())
                    }
                }
                Path {
                    segments,
                    kind: PathKind::Windows { root, prefix },
                }
            }
        }
    }

    pub fn serialize(self) -> String {
        match self.kind {
            PathKind::Unix { root } => {
                format!("{}{}", if root { "/" } else { "" }, self.segments.join("/"))
            }
            PathKind::Windows { root, prefix } => {
                let left = if root { r"\" } else { "" };
                let right = String::from_utf8(self.segments.join(r"\").into()).unwrap();
                if let Some(p) = prefix {
                    format!("{}{}{}", p.as_str(), left, right)
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
                    ss.push(s);
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
            (Extension, _) if self.segments.len() > 0 => {
                let parts: Vec<&str> = self.segments.last().unwrap().split(".").collect();
                match parts.len() {
                    0 | 1 => "".into(),
                    _ => parts.last().unwrap().to_string(),
                }
            }
            (Stem, _) if self.segments.len() > 0 => {
                let parts: Vec<&str> = self.segments.last().unwrap().split(".").collect();
                match parts.len() {
                    0 | 1 => "".into(),
                    _ => parts[..parts.len() - 1].join("."),
                }
            }
            (Name, _) if self.segments.len() > 0 => self.segments.last().unwrap().clone(),
            _ => "".into(),
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

    #[test]
    fn hueristic() {
        // if it contains a back slash and no forward slashes, assume windows
        // if no schema or authority, and the first segment contains dots, assume URL
    }
}
