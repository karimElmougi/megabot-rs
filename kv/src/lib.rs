use std::fs::File;
use std::io;
use std::io::{BufRead, Seek, Write};
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unable to write record: {0}")]
    Read(String),

    #[error("Unable to read record: {0}")]
    Write(String),
}

fn write_err<E: std::error::Error>(err: E) -> Error {
    Error::Write(err.to_string())
}

fn read_err<E: std::error::Error>(err: E) -> Error {
    Error::Read(err.to_string())
}

#[derive(Debug)]
enum Storage {
    File(File),
    Memory(Vec<u8>),
}

impl io::Write for Storage {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Storage::File(f) => f.write(buf),
            Storage::Memory(v) => v.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Storage::File(f) => f.flush(),
            Storage::Memory(_) => Ok(()),
        }
    }
}

#[derive(Clone)]
pub struct Store<T>(Arc<Mutex<StoreInner<T>>>);

struct StoreInner<T> {
    backing_storage: Storage,
    _phantom: PhantomData<T>,
}

impl<T> Store<T>
where
    T: Serialize + for<'a> Deserialize<'a>,
{
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(path)?;

        let inner = StoreInner {
            backing_storage: Storage::File(file),
            _phantom: PhantomData::default(),
        };

        Ok(Store(Arc::new(Mutex::new(inner))))
    }

    pub fn in_memory() -> Self {
        let inner = StoreInner {
            backing_storage: Storage::Memory(Default::default()),
            _phantom: PhantomData::default(),
        };
        Store(Arc::new(Mutex::new(inner)))
    }

    pub fn set(&self, key: &str, data: T) -> Result<(), Error> {
        let mut inner = self.0.lock();
        let data = serde_json::to_string(&Some(data)).map_err(write_err)?;
        writeln!(inner.backing_storage, "{key},{data}").map_err(write_err)
    }

    pub fn unset(&self, key: &str) -> Result<(), Error> {
        let mut inner = self.0.lock();
        let data = serde_json::to_string(&Option::<T>::None).map_err(write_err)?;
        writeln!(inner.backing_storage, "{key},{data}").map_err(write_err)
    }

    pub fn get(&self, key: &str) -> Result<Option<T>, Error> {
        let mut inner = self.0.lock();

        match inner.backing_storage {
            Storage::File(ref mut f) => {
                f.rewind().map_err(read_err)?;
                search_lines(f, key)
            }
            Storage::Memory(ref mut b) => search_lines(&mut b.as_slice(), key),
        }
    }
}

fn search_lines<T, R: io::Read>(reader: R, key: &str) -> Result<Option<T>, Error>
where
    T: for<'a> Deserialize<'a>,
{
    let mut reader = io::BufReader::new(reader);
    let mut value = None;
    let mut line = String::with_capacity(100);
    let mut line_number = 0;

    fn line_error(line_number: u64, line: &str) -> Error {
        Error::Read(format!("Invalid data as line {line_number}: `{line}`"))
    }

    while reader.read_line(&mut line).map_err(read_err)? != 0 {
        let mut split = line.split(',');
        let k = split.next().ok_or_else(|| line_error(line_number, &line))?;
        if k == key {
            let v = split
                .next()
                .ok_or_else(|| line_error(line_number, &line))?
                .trim();

            value = serde_json::from_str(v).map_err(read_err)?;
        }
        line.clear();
        line_number += 1;
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test() {
        let store = Store::<u8>::in_memory();
        assert_eq!(None, store.get("key1").unwrap());

        store.set("key1", 1).unwrap();
        assert_eq!(Some(1), store.get("key1").unwrap());
        assert_eq!(None, store.get("not a key").unwrap());

        store.set("key2", 1).unwrap();
        store.set("key3", 1).unwrap();
        store.set("key1", 2).unwrap();
        store.set("key1", 3).unwrap();

        assert_eq!(Some(3), store.get("key1").unwrap());

        store.unset("key1").unwrap();
        assert_eq!(None, store.get("key1").unwrap());
    }

    #[test]
    fn line_error_test() {
        let data = "key1,1\nkey2".as_bytes();

        // We don't attempt to access the data if the key doesn't match, so we never notice the
        // data is missing.
        assert_eq!(None, search_lines::<u8, _>(data, "not a key").unwrap());

        assert!(search_lines::<u8, _>(data, "key2").is_err());
    }

    #[test]
    fn file_test() {
        let f = NamedTempFile::new().unwrap();
        let store = Store::<u8>::open(f.path()).unwrap();

        store.set("key1", 1).unwrap();
        store.set("key1", 2).unwrap();

        assert_eq!(Some(2), store.get("key1").unwrap());
        assert_eq!(Some(2), store.get("key1").unwrap());

        store.set("key1", 3).unwrap();
        assert_eq!(Some(3), store.get("key1").unwrap());
    }
}
