use std::io::Error as io_err;
use std::io::ErrorKind as io_errkind;

pub fn io_other_err_wrapper(msg: &str) -> io_err
{
    io_err::new(io_errkind::Other, msg)
}