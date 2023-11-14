use std::cmp;
use std::ffi::CString;
use std::str;
use substring::Substring;

use nsstring::nsACString;

pub mod stream;

#[xpcom::xpcom(implement(nsIInputStream), atomic)]
pub struct SimpleInputStream {
    src: &str,
    cur_index: u32,
}

impl SimpleInputStream {
    pub fn new(src: *const nsACString) -> RefPtr<HttpClient> {
        HttpClient::allocate(InitHttpClient {
            src: str::from_utf8(*src).expect("source is not UTF8"),
            cur_index: 0,
        })
    }

    xpcom_method!(close => Close());
    fn close(&self) -> Result<(), nserror> {
        Ok(())
    }

    xpcom_method!(available => Available() -> u64);
    fn available(&self, available: *mut u64) -> Result<u64, nserror> {
        Ok(u64(src.len() - cur_index))
    }

    xpcom_method!(stream_status => StreamStatus());
    fn stream_status(&self) -> Result<(), nserror> {
        if (cur_index == self.src.len()) {
            return nserror::NS_BASE_STREAM_CLOSED;
        }

        Ok(())
    }

    xpcom_method!(read => Read(aBuf: *mut c_char, aCount: u32) -> u32);
    unsafe fn Read(&self, aBuf: *mut c_char, aCount: u32) -> Result<u32, nserror> {
        if (cur_index == self.src.len()) {
            return Err(nserror::NS_BASE_STREAM_CLOSED);
        }

        let end = cmp::min(cur_index + aCount, self.src.len());
        let count = self.src.len() - end;

        aBuf = CString::new(self.src.substring(cur_index, count));

        Ok(count)
    }

    xpcom_method!(is_non_blocking => isNonBlocking() -> bool);
    fn is_non_blocking(&self) -> Result<bool, nserror> {
        Ok(false)
    }
}
