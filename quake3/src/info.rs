use crate::qstr::{QStr, QString};

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InfoStr(QStr);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FromBytesError(usize);

impl InfoStr {
    #[inline]
    #[must_use]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        // TODO: debug_assert
        // SAFETY: ???
        unsafe { &*(QStr::from_bytes_unchecked(bytes) as *const QStr as *const Self) }
    }

    pub fn from_bytes<B: core::convert::AsRef<[u8]> + ?Sized>(
        bytes: &B,
    ) -> core::result::Result<&Self, FromBytesError> {
        let bytes = bytes.as_ref();
        if let Some(index) = memchr::memchr(b'\\', bytes) {
            return core::result::Result::Err(FromBytesError(index));
        }
        let qstr = QStr::from_bytes(bytes).map_err(|e| FromBytesError(e.0))?;
        // SAFETY: ???
        core::result::Result::Ok(unsafe { &*(qstr as *const QStr as *const Self) })
    }

    #[must_use]
    #[inline(always)]
    pub const fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl alloc::borrow::ToOwned for InfoStr {
    type Owned = InfoString;

    fn to_owned(&self) -> InfoString {
        InfoString(self.0.to_owned().into())
    }
}

impl core::convert::AsRef<[u8]> for InfoStr {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct InfoString(QString);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ByteError(usize, alloc::vec::Vec<u8>);

impl InfoString {
    #[must_use]
    pub unsafe fn from_bytes_unchecked(bytes: alloc::vec::Vec<u8>) -> Self {
        // TODO: debug_assert
        Self(QString::from_bytes_unchecked(bytes))
    }

    pub fn from_bytes<B: core::convert::Into<alloc::vec::Vec<u8>>>(
        bytes: B,
    ) -> core::result::Result<Self, ByteError> {
        let bytes = bytes.into();
        if let Some(index) = memchr::memchr(b'\\', &bytes) {
            return core::result::Result::Err(ByteError(index, bytes));
        }
        match QString::from_bytes(bytes) {
            core::result::Result::Err(e) => Err(ByteError(e.0, e.1)),
            core::result::Result::Ok(qstring) => Ok(Self(qstring)),
        }
    }
}

impl core::ops::Deref for InfoString {
    type Target = InfoStr;

    #[inline]
    fn deref(&self) -> &InfoStr {
        // SAFETY: ???
        unsafe { InfoStr::from_bytes_unchecked(&self.0.as_ref()) }
    }
}

impl core::borrow::Borrow<InfoStr> for InfoString {
    #[inline]
    fn borrow(&self) -> &InfoStr {
        self
    }
}

impl<T> core::convert::AsRef<T> for InfoString
where
    T: ?Sized,
    <InfoString as core::ops::Deref>::Target: core::convert::AsRef<T>,
{
    #[inline]
    fn as_ref(&self) -> &T {
        use core::ops::Deref;

        self.deref().as_ref()
    }
}

pub struct InfoMap<K, V, const L: usize, S = std::collections::hash_map::RandomState>(
    indexmap::IndexMap<K, V, S>,
);

pub struct LimitError(());

impl<K, V, const L: usize> InfoMap<K, V, L> {
    const LIMIT: usize = L;

    pub fn new() -> Self {
        Self(indexmap::IndexMap::new())
    }
}

impl<K, V, const L: usize, S> InfoMap<K, V, L, S>
where
    K: core::hash::Hash + core::cmp::Eq,
    S: core::hash::BuildHasher,
{
    pub fn try_insert(&mut self, key: K, value: V) -> core::result::Result<Option<V>, LimitError> {
        // TODO: if self.size(key) + size(key) + size(value) < Self::LIMIT..
        todo!();
    }
}

// MAX_INFO_STRING
pub const INFO_LIMIT: usize = 1024;
// BIG_INFO_STRING
pub const INFO_BIG_LIMIT: usize = 8192;

pub type Info = InfoMap<InfoString, InfoString, INFO_LIMIT>;

pub type BigInfo = InfoMap<InfoString, InfoString, INFO_BIG_LIMIT>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qstr_from_bytes() {
        assert!(InfoStr::from_bytes(b"lorem ipsum").is_ok());

        assert!(InfoStr::from_bytes(b"lorem\0ipsum").is_err());
        assert!(InfoStr::from_bytes(b"lorem\\ipsum").is_err());

        assert!(InfoStr::from_bytes(b"lorem ipsum\0").is_err());
    }

    #[test]
    fn qstring_from_bytes() {
        assert!(InfoString::from_bytes(b"lorem ipsum".to_vec()).is_ok());

        assert!(InfoString::from_bytes(b"lorem\0ipsum".to_vec()).is_err());
        assert!(InfoString::from_bytes(b"lorem\\ipsum".to_vec()).is_err());

        assert!(InfoString::from_bytes(b"lorem ipsum\0".to_vec()).is_err());
    }
}
