use anyhow::Result;
use std::{fs::File, io::Write};

// TODO make it like println! macro
pub fn my_println(
    log_handle: &mut Option<File>,
    write_log: &bool,
    write_stdout: &bool,
    s: &String,
) -> Result<()> {
    if *write_stdout {
        println!("{}", s);
    }

    if *write_log {
        if let Some(log_handle) = log_handle {
            log_handle.write_fmt(format_args!("{}\n", s))?;
        }
    }

    return Ok(());
}
