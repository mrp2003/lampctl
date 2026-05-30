//! Minimal `hidraw` feature-report ioctls (Linux), computed without any C bindings.
use std::io;
use std::os::unix::io::RawFd;

const HIDIOC_MAGIC: u32 = b'H' as u32;
const IOC_WRITE: u32 = 1;
const IOC_READ: u32 = 2;

/// Build a Linux `_IOC()` request number.
const fn ioc(dir: u32, ty: u32, nr: u32, size: u32) -> u32 {
    (dir << 30) | (ty << 8) | nr | (size << 16)
}

fn hidioc_sfeature(len: usize) -> u32 {
    ioc(IOC_WRITE | IOC_READ, HIDIOC_MAGIC, 0x06, len as u32)
}

fn hidioc_gfeature(len: usize) -> u32 {
    ioc(IOC_WRITE | IOC_READ, HIDIOC_MAGIC, 0x07, len as u32)
}

/// `HIDIOCSFEATURE` — send a feature report. `buf[0]` is the report id.
pub fn set_feature(fd: RawFd, buf: &[u8]) -> io::Result<()> {
    let req = hidioc_sfeature(buf.len()) as libc::c_ulong;
    let ret = unsafe { libc::ioctl(fd, req, buf.as_ptr()) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// `HIDIOCGFEATURE` — read a feature report into `buf` (`buf[0]` = report id on entry).
pub fn get_feature(fd: RawFd, buf: &mut [u8]) -> io::Result<usize> {
    let req = hidioc_gfeature(buf.len()) as libc::c_ulong;
    let ret = unsafe { libc::ioctl(fd, req, buf.as_mut_ptr()) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret as usize)
    }
}
