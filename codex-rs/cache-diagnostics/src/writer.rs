use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::OpenError;
use crate::key::open_private_create_new;

const MAX_RECORD_BYTES: usize = 256 * 1024;

pub(crate) struct RecordWriter {
    inner: Mutex<RecordWriterInner>,
}

struct RecordWriterInner {
    file: File,
    next_sequence: u64,
}

impl RecordWriter {
    pub(crate) fn open(directory: &Path, run_id: &str) -> Result<Self, OpenError> {
        let directory = directory.join(chrono::Utc::now().format("%Y/%m/%d").to_string());
        std::fs::create_dir_all(&directory).map_err(OpenError::from_io)?;
        let path = directory.join(format!("run-{run_id}.jsonl"));
        let file = open_private_create_new(&path).map_err(OpenError::from_io)?;
        Ok(Self {
            inner: Mutex::new(RecordWriterInner {
                file,
                next_sequence: 1,
            }),
        })
    }

    pub(crate) fn append<T: Serialize>(
        &self,
        mut record: T,
        set_metadata: impl FnOnce(&mut T, u64, Option<u64>),
    ) -> Result<(), OpenError> {
        let mut inner = self.lock_inner();
        let sequence = inner.next_sequence;
        let next_sequence = sequence.checked_add(1).ok_or(OpenError::unavailable())?;
        set_metadata(&mut record, sequence, unix_time_ms());
        let line = serde_json::to_vec(&record).map_err(|_| OpenError::unavailable())?;
        if line.len() >= MAX_RECORD_BYTES {
            return Err(OpenError::unavailable());
        }
        inner.file.write_all(&line).map_err(OpenError::from_io)?;
        inner.file.write_all(b"\n").map_err(OpenError::from_io)?;
        inner.file.flush().map_err(OpenError::from_io)?;
        inner.next_sequence = next_sequence;
        Ok(())
    }

    fn lock_inner(&self) -> MutexGuard<'_, RecordWriterInner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn unix_time_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}
