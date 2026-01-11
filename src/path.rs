// experiment:
// build a data structure that deserialize and re-serialize any URL or Path
// without losing any information

use typed_path::{
    Component, PathType, TypedComponent, UnixComponent, UnixPath, Utf8TypedComponent,
    Utf8TypedPath, Utf8UnixComponent, Utf8WindowsComponent, Utf8WindowsPrefixComponent,
    WindowsComponent, WindowsPrefix, WindowsPrefixComponent,
};

use url::{ParseError, Url};

// note that Url only stores locations into the string
// Url also works entirely with str.  Maybe I want to use utf8 with typed path?
//pub struct Path<'a> {
//    scheme: Option<String>,
//    prefix: Option<Utf8WindowsPrefixComponent<'a>>,
//    /// contains root?
//    root: bool,
//    /// path segments
//    segments: Vec<String>,
//    is_windows: bool,
//}

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
        fragments: Vec<String>,
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
            Some(segs) => segs.map(|s| s.to_owned()).collect(),
            Option::None => Vec::new(),
        };
        Ok((
            Path {
                segments,
                kind: PathKind::Url {
                    scheme,
                    username: None,
                    password: None,
                    host: url
                        .host()
                        .map(|h| vec![h.to_string()])
                        .unwrap_or(Vec::new()),
                    query_params: Vec::new(),
                    fragments: Vec::new(),
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
                fragments,
            } => {
                let mut ss: Vec<String> = Vec::new();
                if let Some(s) = scheme {
                    ss.push(s);
                    ss.push("://".to_owned());
                }
                ss.push(host.join("."));
                ss.join("")
            }
        }

        //if let Some(prefix) = self.prefix {
        //    s.push(prefix.as_str().to_owned())
        //}
        //if let Some(scheme) = self.scheme {
        //    s.push(scheme);
        //    s.push("//".to_owned());
        //}
        //let separator = (if self.is_windows { r"\" } else { "/" }).to_owned();
        //if self.root {
        //    s.push(separator.clone());
        //}
        //for (i, segment) in self.segments.iter().enumerate() {
        //    if i > 0 {
        //        s.push(separator.clone());
        //    }
        //    s.push(segment.clone());
        //}
        //String::from_utf8(s.join("").into()).unwrap()
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
    // schemes
    #[case("https://github.com")]
    fn identity(#[case] path: &str) {
        let p = Path::parse(path);
        assert_eq!(path.to_string(), p.clone().serialize(), "{:?}", p);
    }
}

/// Parts of a file, dot separated
/// This might not be a helpful abstraction
/// I guess the reason that all these libs dont just store this stuff
/// is so they dont have to do all this computation up front
pub struct File<'a> {
    parts: Vec<&'a [u8]>,
}

impl File<'_> {
    pub fn parse(path: &str) -> File {
        // will need to be more robust
        let parts = path.split(".").into_iter().map(|s| s.as_bytes()).collect();
        File { parts }
    }
}
