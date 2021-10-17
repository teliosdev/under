use std::fmt::Write;
use std::sync::Arc;

#[derive(Clone, Debug)]
/// The pattern actually used to match against the path.  This contains both
/// the regular expression for the pattern, as well as an array of strings
/// that contain information about the capture.
pub(crate) struct Pattern {
    regex: regex::Regex,
    match_keys: Arc<[Option<Arc<str>>]>,
}

impl Pattern {
    pub fn new(prefix: &str) -> Self {
        let regex = regex::Regex::new(&regex_pattern(prefix)).unwrap();
        let match_keys = regex
            .capture_names()
            .map(|v| v.map(Arc::from))
            .collect::<Arc<[_]>>();

        Pattern { regex, match_keys }
    }

    /// Get a reference to the pattern's regex.
    pub(crate) fn regex(&self) -> &regex::Regex {
        &self.regex
    }

    /// Get a reference to the pattern's match keys.
    pub(crate) fn match_keys(&self) -> &Arc<[Option<Arc<str>>]> {
        &self.match_keys
    }
}

lazy_static::lazy_static! {
    static ref PATTERN: regex::Regex = regex::Regex::new("\\{(?P<name>[a-zA-Z]+)?(?::(?P<pattern>[a-zA-Z]+))?\\}").unwrap();
}

fn regex_pattern(path: &str) -> String {
    let mut start = 0;
    let mut buffer = String::with_capacity(path.len() + 2);
    buffer.push('^');

    for matches in PATTERN.find_iter(path) {
        buffer.push_str(&regex::escape(&path[start..matches.start()]));
        start = matches.end();
        let capture = PATTERN.captures(matches.as_str()).unwrap();
        let name = capture.name("name").map(|m| m.as_str());
        let pattern = capture.name("pattern").map(|m| m.as_str());
        push_pattern(&mut buffer, name, pattern);
    }

    buffer.push_str(&regex::escape(&path[start..]));

    buffer.push('$');
    buffer
}

static UUID_PATTERN: &str =
    "[a-fA-F0-9]{8}-[a-fA-F0-9]{4}-4[a-fA-F0-9]{3}-[89aAbB][a-fA-F0-9]{3}-[a-fA-F0-9]{12}";

fn push_pattern(buffer: &mut String, name: Option<&str>, pattern: Option<&str>) {
    struct NamePattern<'n>(Option<&'n str>);
    impl std::fmt::Display for NamePattern<'_> {
        fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if let Some(n) = self.0 {
                write!(fmt, "?P<{}>", n)
            } else {
                Ok(())
            }
        }
    }
    let name = NamePattern(name);
    match pattern {
        Some("oext") => write!(buffer, "(?:\\.({}[^/]+))?", name),
        Some("int") => write!(buffer, "({}[+-]?\\d+)", name),
        Some("uint") => write!(buffer, "({}\\d+)", name),
        Some("path") => write!(buffer, "({}.+)", name),
        Some("uuid") => write!(buffer, "({}{})", name, UUID_PATTERN),
        Some("str" | "s" | "string") | None => write!(buffer, "({}[^/]+)", name),
        Some(v) => panic!("unknown path pattern type {:?}", v),
    }
    .unwrap();
}
