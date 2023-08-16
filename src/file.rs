//! File wrapper with extra context on errors and buffered reading/writing.
use crate::prelude::*;
use std::{
    fmt,
    io::{self, BufWriter, Read, Seek, Write},
    path::PathBuf,
};

/// Wraps a std [`File`](std::fs::File) which provides extra context for errors and buffered
/// writing.
#[derive(Debug)]
pub struct File {
    inner: BufWriter<std::fs::File>,
    path: PathBuf,
}

impl File {
    /// Opens a file in write-only mode.
    ///
    /// This function will create a file if it does not exist, and will truncate it if it does.
    pub fn create(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let inner = std::fs::File::create(&path)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create or open file '{}'", path.display()))
            .map(BufWriter::new)?;

        Ok(Self { path, inner })
    }

    /// Opens a file in write-only mode.
    ///
    /// This function will create a file if it does not exist, and will append to it if it does.
    pub fn append(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let inner = std::fs::File::options()
            .create(true)
            .append(true)
            .open(&path)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to create or open file '{}'", path.display()))
            .map(BufWriter::new)?;

        Ok(Self { path, inner })
    }

    /// Opens a file in read-only mode.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let inner = std::fs::File::open(&path)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to open file '{}'", path.display()))
            .map(BufWriter::new)?;

        Ok(Self { path, inner })
    }

    /// Unwrap into `std::fs::File`, flushing any data to be written.
    pub fn into_std_file(self) -> Result<std::fs::File> {
        self.inner.into_inner().into_diagnostic()
    }

    /// Read entire file contents to byte buffer.
    ///
    /// Note that reading starts from where the cursor is.
    /// Previous reads may have advanced the cursor.
    pub fn read_to_vec(&mut self) -> Result<Vec<u8>> {
        let len = self
            .inner
            .get_ref()
            .metadata()
            .map(|x| x.len())
            .unwrap_or_default() as usize;
        let mut buf = Vec::with_capacity(len);
        self.read_to_end(&mut buf)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed reading bytes from '{}'", self.path.display()))?;
        Ok(buf)
    }

    /// Read entire file contents as a UTF8 encoded string.
    ///
    /// Note that reading starts from where the cursor is.
    /// Previous reads may have advanced the cursor.
    pub fn read_to_string(&mut self) -> Result<String> {
        self.read_to_vec().and_then(|x| {
            String::from_utf8(x).into_diagnostic().wrap_err_with(|| {
                format!(
                    "failed to encode bytes from '{}' as UTF8",
                    self.path.display()
                )
            })
        })
    }

    fn wrap_err(&self, err: io::Error) -> io::Error {
        let kind = err.kind();
        io::Error::new(
            kind,
            Error {
                path: self.path.clone(),
                inner: err,
            },
        )
    }
}

#[derive(Debug)]
struct Error {
    path: PathBuf,
    inner: io::Error,
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        Some(&self.inner)
    }

    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "io error with file '{}': {}",
            self.path.display(),
            self.inner
        )
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.get_mut().read(buf).map_err(|e| self.wrap_err(e))
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.inner
            .get_mut()
            .read_exact(buf)
            .map_err(|e| self.wrap_err(e))
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.inner
            .get_mut()
            .read_to_end(buf)
            .map_err(|e| self.wrap_err(e))
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner
            .get_mut()
            .read_vectored(bufs)
            .map_err(|e| self.wrap_err(e))
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.inner
            .get_mut()
            .read_to_string(buf)
            .map_err(|e| self.wrap_err(e))
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf).map_err(|e| self.wrap_err(e))
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.inner
            .write_vectored(bufs)
            .map_err(|e| self.wrap_err(e))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush().map_err(|e| self.wrap_err(e))
    }
}

impl Seek for File {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos).map_err(|e| self.wrap_err(e))
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        self.inner.stream_position().map_err(|e| self.wrap_err(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_not_found() {
        let x = File::open("wont-exist.txt").unwrap_err().to_string();
        assert_eq!(&x, "failed to open file 'wont-exist.txt'");
    }
}
