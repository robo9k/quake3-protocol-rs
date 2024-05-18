use crate::qstr::{QStr, QString};

const BACKSLASH: u8 = b'\\';

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InfoStr(QStr);

#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
#[cfg_attr(feature = "std", error("NUL at {}", self.0))]
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
        if let Some(index) = memchr::memchr(BACKSLASH, bytes) {
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
#[cfg_attr(feature = "std", derive(thiserror::Error))]
#[cfg_attr(feature = "std", error("NUL at {}", self.0))]
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
        if let Some(index) = memchr::memchr(BACKSLASH, &bytes) {
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

trait InfoSize {
    // BACKSLASH + bytes
    fn encoded_size(&self) -> usize;
}

impl InfoSize for InfoStr {
    fn encoded_size(&self) -> usize {
        1 + self.0.len()
    }
}

impl InfoSize for InfoString {
    fn encoded_size(&self) -> usize {
        1 + self.0.len()
    }
}

// TODO: derives if K, V, S permit
pub struct InfoMap<K, V, const L: usize, S = std::collections::hash_map::RandomState>(
    indexmap::IndexMap<K, V, S>,
);

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
#[cfg_attr(feature = "std", error("limit"))]
pub struct LimitError<K, V>(K, V);

impl<K, V, const L: usize, S> InfoMap<K, V, L, S> {
    pub const LIMIT: usize = L;
}

impl<K, V, const L: usize> InfoMap<K, V, L> {
    pub fn new() -> Self {
        Self(indexmap::IndexMap::new())
    }
}

impl<K, V, const L: usize, S> InfoMap<K, V, L, S>
where
    K: core::hash::Hash + core::cmp::Eq,
    S: core::hash::BuildHasher,
    K: InfoSize,
    V: InfoSize,
{
    fn encoded_size(&self, ignore: &K) -> usize {
        self.0
            .iter()
            .filter(|(k, _v)| *k != ignore)
            .fold(0, |acc, (k, v)| acc + k.encoded_size() + v.encoded_size())
    }

    pub fn try_insert(
        &mut self,
        key: K,
        value: V,
    ) -> core::result::Result<Option<V>, LimitError<K, V>> {
        let size = self.encoded_size(&key);
        if size + key.encoded_size() + value.encoded_size() > Self::LIMIT {
            return core::result::Result::Err(LimitError(key, value));
        }

        Ok(self.0.insert(key, value))
    }

    // at least the following makes the API map-ish, everything that mutates needs to be fallible to obey LIMIT
    // advanced functions could be dodged by into_hashmap() ?
    // TODO: pub fn get<Q>(&self, key: &Q) -> Option<&V>
    // TODO: pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    // TODO: pub fn iter(&self) -> Iter<'_, K, V>

    // TODO: test that insertion and removal work like in Q3

    // TODO: read from bytes aka parse
    // TODO: write as bytes
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
    fn infostr_from_bytes() {
        assert!(InfoStr::from_bytes(b"lorem ipsum").is_ok());

        assert!(InfoStr::from_bytes(b"lorem\0ipsum").is_err());
        assert!(InfoStr::from_bytes(b"lorem\\ipsum").is_err());

        assert!(InfoStr::from_bytes(b"lorem ipsum\0").is_err());
    }

    #[test]
    fn infostring_from_bytes() {
        assert!(InfoString::from_bytes(b"lorem ipsum".to_vec()).is_ok());

        assert!(InfoString::from_bytes(b"lorem\0ipsum".to_vec()).is_err());
        assert!(InfoString::from_bytes(b"lorem\\ipsum".to_vec()).is_err());

        assert!(InfoString::from_bytes(b"lorem ipsum\0".to_vec()).is_err());
    }

    #[test]
    fn infomap_tryinsert() -> Result<(), Box<dyn std::error::Error>> {
        let mut info: InfoMap<InfoString, InfoString, 13> = InfoMap::new();

        // new entry below limit
        let res = info.try_insert(
            InfoString::from_bytes(b"k0")?,
            InfoString::from_bytes(b"vA")?,
        )?;
        assert_eq!(res, None);

        // new entry below limit
        let res = info.try_insert(
            InfoString::from_bytes(b"k1")?,
            InfoString::from_bytes(b"vB")?,
        )?;
        assert_eq!(res, None);

        // existing key below limit
        let res = info.try_insert(
            InfoString::from_bytes(b"k1")?,
            InfoString::from_bytes(b"vC")?,
        )?;
        assert_eq!(res, Some(InfoString::from_bytes(b"vB")?));

        // \k0\vA\k1\vC\ == 13 == limit
        let res = info.try_insert(
            InfoString::from_bytes(b"k2")?,
            InfoString::from_bytes(b"vD")?,
        );
        assert_eq!(
            res,
            Err(LimitError(
                InfoString::from_bytes(b"k2")?,
                InfoString::from_bytes(b"vD")?
            ))
        );

        Ok(())
    }
}
