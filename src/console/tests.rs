// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use super::*;

struct RejectIo {
    write: bool,
}

impl Write for RejectIo {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.write {
            Err(io::Error::other("write failed"))
        } else {
            Ok(bytes.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("flush failed"))
    }
}

#[test]
fn record_writer_reports_write_and_flush_failures() {
    let mut writer = RecordWriter(RejectIo { write: true });
    assert_eq!(
        writer.write_all(b"record").unwrap_err().to_string(),
        "write failed"
    );
    let mut writer = RecordWriter(RejectIo { write: false });
    assert_eq!(
        writer.write_all(b"record").unwrap_err().to_string(),
        "flush failed"
    );
    assert_eq!(writer.flush().unwrap_err().to_string(), "flush failed");
}
