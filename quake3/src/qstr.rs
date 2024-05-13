/// A borrowed C-compatible byte string that contains no interior `\0` but no terminating `\0` either
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QStr([u8]);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FromBytesNulError(/* FIXME: */ pub(crate) usize);

impl QStr {
    #[inline]
    #[must_use]
    pub unsafe fn from_bytes_unchecked(bytes: &[u8]) -> &Self {
        // TODO: debug_assert
        // SAFETY: ???
        unsafe { &*(bytes as *const [u8] as *const Self) }
    }

    pub fn from_bytes<B: core::convert::AsRef<[u8]> + ?Sized>(
        bytes: &B,
    ) -> core::result::Result<&Self, FromBytesNulError> {
        let bytes = bytes.as_ref();
        if let Some(index) = memchr::memchr(b'\0', bytes) {
            return core::result::Result::Err(FromBytesNulError(index));
        }
        // SAFETY: ???
        core::result::Result::Ok(unsafe { Self::from_bytes_unchecked(bytes) })
    }

    #[must_use]
    #[inline(always)]
    pub const fn as_bytes(&self) -> &[u8] {
        // SAFETY: const sound because we transmute two types with the same layout
        unsafe { core::intrinsics::transmute(self) }
    }
}

impl alloc::borrow::ToOwned for QStr {
    type Owned = QString;

    fn to_owned(&self) -> QString {
        QString(self.0.to_owned().into())
    }
}

impl core::convert::AsRef<[u8]> for QStr {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

/// An owned C-compatible byte string that contains no interior `\0` but no terminating `\0` either
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct QString(alloc::boxed::Box<[u8]>);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NulError(
    /* FIXME: */ pub(crate) usize,
    /* FIXME: */ pub(crate) alloc::vec::Vec<u8>,
);

impl QString {
    #[must_use]
    pub unsafe fn from_bytes_unchecked(bytes: alloc::vec::Vec<u8>) -> Self {
        // TODO: debug_assert
        Self(bytes.into_boxed_slice())
    }

    pub fn from_bytes<B: core::convert::Into<alloc::vec::Vec<u8>>>(
        bytes: B,
    ) -> core::result::Result<Self, NulError> {
        let bytes = bytes.into();
        if let Some(index) = memchr::memchr(b'\0', &bytes) {
            return core::result::Result::Err(NulError(index, bytes));
        }
        // SAFETY: ???
        core::result::Result::Ok(unsafe { Self::from_bytes_unchecked(bytes) })
    }
}

impl core::ops::Deref for QString {
    type Target = QStr;

    #[inline]
    fn deref(&self) -> &QStr {
        // SAFETY: ???
        unsafe { QStr::from_bytes_unchecked(&self.0) }
    }
}

impl core::borrow::Borrow<QStr> for QString {
    #[inline]
    fn borrow(&self) -> &QStr {
        self
    }
}

impl<T> core::convert::AsRef<T> for QString
where
    T: ?Sized,
    <QString as core::ops::Deref>::Target: core::convert::AsRef<T>,
{
    #[inline]
    fn as_ref(&self) -> &T {
        use core::ops::Deref;

        self.deref().as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qstr_from_bytes() {
        assert!(QStr::from_bytes(b"lorem ipsum").is_ok());

        assert!(QStr::from_bytes(b"lorem\0ipsum").is_err());

        assert!(QStr::from_bytes(b"lorem ipsum\0").is_err());
    }

    #[test]
    fn qstring_from_bytes() {
        assert!(QString::from_bytes(b"lorem ipsum".to_vec()).is_ok());

        assert!(QString::from_bytes(b"lorem\0ipsum".to_vec()).is_err());

        assert!(QString::from_bytes(b"lorem ipsum\0".to_vec()).is_err());
    }
}
