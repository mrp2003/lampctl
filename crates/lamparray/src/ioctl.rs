use std::io;
use std::os::unix::io::RawFd;

use crate::hid;

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

/// `HIDIOCSFEATURE` — send a feature report. `id` is the report id.
pub fn set_feature_typed<T: Sized>(fd: RawFd, id: u8, data: T) -> io::Result<i32> {
    let data = hid::Report {
        report_id: id,
        data,
    };

    let req = hidioc_sfeature(std::mem::size_of_val(&data)) as libc::c_ulong;
    let ret = unsafe { libc::ioctl(fd, req, &data) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}

/// `HIDIOCGFEATURE` — read a feature report
pub fn get_feature_typed<T: Sized + Default>(fd: RawFd, id: u8) -> io::Result<T> {
    let mut data = hid::Report {
        report_id: id,
        data: T::default(),
    };
    let req = hidioc_gfeature(std::mem::size_of_val(&data)) as libc::c_ulong;
    let ret = unsafe { libc::ioctl(fd, req, &mut data) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(data.data)
    }
}
