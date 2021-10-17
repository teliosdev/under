use crate::router::Route;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

/// Contains all of the fragment information from the route definition.
///
/// This is meant to act as an extension on a request, and never used publicly.
#[derive(Debug)]
pub struct Fragment {
    base: String,
    fragments_index: Vec<Option<Range<usize>>>,
    fragments_hash: HashMap<Arc<str>, Option<Range<usize>>>,
}

impl Fragment {
    pub(crate) fn new(path: impl Into<String>, route: &Route) -> Option<Self> {
        let path = path.into();
        let captures = route.pattern.regex().captures(&path)?;
        let fragments_index = captures
            .iter()
            .map(|v| v.map(|v| v.range()))
            .collect::<Vec<_>>();
        let fragments_hash = route
            .pattern
            .match_keys()
            .iter()
            .enumerate()
            .flat_map(|(i, n)| n.clone().map(|nn| (nn, fragments_index[i].clone())))
            .collect::<HashMap<_, _>>();

        Some(Fragment {
            base: path,
            fragments_index,
            fragments_hash,
        })
    }

    pub(crate) fn get(&self, i: usize) -> Option<&str> {
        self.fragments_index
            .get(i)
            .and_then(|r| r.as_ref())
            .map(|r| &self.base[r.clone()])
    }

    pub(crate) fn name<Q>(&self, n: &Q) -> Option<&str>
    where
        Q: ?Sized,
        Arc<str>: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + Eq,
    {
        self.fragments_hash
            .get(n)
            .and_then(|r| r.as_ref())
            .map(|r| &self.base[r.clone()])
    }

    pub(crate) fn select<K>(&self, key: K) -> Option<&str>
    where
        K: FragmentSelect,
    {
        key.select(self)
    }
}

/// A trait used to implement path fragment retrieval.
///
/// This is defined as opposed to implementing [`std::ops::Index`] as
/// [`std::ops::Index`] would not be able to output an optional value.
pub trait FragmentSelect: self::sealed::FragmentSelectSealed {}

mod sealed {
    pub trait FragmentSelectSealed {
        fn select(self, fragment: &super::Fragment) -> Option<&str>;
    }
}

impl sealed::FragmentSelectSealed for usize {
    fn select(self, fragment: &Fragment) -> Option<&str> {
        fragment.get(self)
    }
}

impl FragmentSelect for usize {}

impl<'v, Q> sealed::FragmentSelectSealed for &'v Q
where
    Q: ?Sized,
    Arc<str>: std::borrow::Borrow<Q>,
    Q: std::hash::Hash + Eq,
{
    fn select(self, fragment: &Fragment) -> Option<&str> {
        fragment.name(self)
    }
}

impl<'v, Q> FragmentSelect for &'v Q
where
    Q: ?Sized,
    Arc<str>: std::borrow::Borrow<Q>,
    Q: std::hash::Hash + Eq,
{
}
