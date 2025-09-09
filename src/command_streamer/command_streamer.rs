use anyhow::{Error, Result};
use nonblock::NonBlockingReader;
use std::{
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
    time::Duration,
};

use crate::string_utils::trim_lines;

/// A struct that provides non-blocking streaming capabilities for command execution.
///
/// This implementation allows for capturing and processing the stdout and stderr streams
/// of a spawned child process in a non-blocking manner. It provides methods to manage
/// buffers, check for EOF, and extract lines from the output streams.
pub struct CommandStreamer {
    child: Option<Child>,
    noblock_stdout: Option<NonBlockingReader<ChildStdout>>,
    noblock_stderr: Option<NonBlockingReader<ChildStderr>>,
    stdout_buffer: String,
    stderr_buffer: String,
    stdout_last_used: bool,
    stdout_at_eof: bool,
    stderr_at_eof: bool,
    program: Option<String>,
    args: Option<Vec<String>>,
    pub user_data: Option<String>,
}

impl CommandStreamer {
    pub fn new_from_child(mut child: Child, user_data: Option<String>) -> Result<Self> {
        let stdout_option = child.stdout.take();
        let stderr_option = child.stderr.take();

        if stdout_option.is_none() && stderr_option.is_none() {
            return Err(Error::msg("both stdout and stderr are none"));
        }

        let mut noblock_stdout = None;
        let mut noblock_stderr = None;

        if stdout_option.is_some() {
            let from_fd_result = NonBlockingReader::from_fd(stdout_option.unwrap());

            if from_fd_result.is_ok() {
                noblock_stdout = Some(from_fd_result.unwrap());
            }
        }

        if stderr_option.is_some() {
            let from_fd_result = NonBlockingReader::from_fd(stderr_option.unwrap());

            if from_fd_result.is_ok() {
                noblock_stderr = Some(from_fd_result.unwrap());
            }
        }

        if noblock_stdout.is_none() && noblock_stderr.is_none() {
            return Err(Error::msg("both stdout and stderr are none"));
        }

        return Ok(CommandStreamer {
            child: Some(child),
            noblock_stdout,
            noblock_stderr,
            stdout_buffer: String::new(),
            stderr_buffer: String::new(),
            stdout_last_used: false,
            stdout_at_eof: false,
            stderr_at_eof: false,
            program: None,
            args: None,
            user_data: user_data,
        });
    }

    pub fn new(program: &str, args: &Vec<String>, user_data: Option<String>) -> Result<Self> {
        let child = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        return match CommandStreamer::new_from_child(child, user_data) {
            Ok(mut streamer) => {
                streamer
                    .set_program(program)
                    .set_args(args.iter().map(|s| s.to_string()).collect());

                return Ok(streamer);
            }
            Err(e) => Err(e),
        };
    }

    pub fn set_program(&mut self, program: &str) -> &mut Self {
        self.program = Some(program.to_string());
        return self;
    }

    pub fn set_args(&mut self, args: Vec<String>) -> &mut Self {
        self.args = Some(args);
        return self;
    }

    pub fn get_program(&self) -> &Option<String> {
        return &self.program;
    }

    pub fn get_args(&self) -> &Option<Vec<String>> {
        return &self.args;
    }

    pub fn format_command(&self) -> Result<String> {
        let mut full_args: Vec<String> = vec![];

        match &self.program {
            Some(program) => full_args.push(program.to_string()),
            None => return Err(Error::msg("program not set")),
        }

        match &self.args {
            Some(args) => full_args.append(&mut args.clone()),
            None => return Err(Error::msg("args not set")),
        }

        return Ok(full_args.join(" "));
    }

    pub fn get_child(&mut self) -> Option<&mut Child> {
        return self.child.as_mut();
    }

    pub fn has_data_in_buffers(&self) -> bool {
        return !self.stdout_buffer.is_empty() || !self.stderr_buffer.is_empty();
    }

    pub fn get_stdout_buffer(&self) -> String {
        return self.stdout_buffer.clone();
    }

    pub fn get_stderr_buffer(&self) -> String {
        return self.stderr_buffer.clone();
    }

    pub fn is_eof(&mut self) -> bool {
        let mut count_eof: u8 = 0;

        if !self.stdout_at_eof {
            match &self.noblock_stdout {
                Some(noblock_stdout) => {
                    if noblock_stdout.is_eof() {
                        count_eof += 1;

                        self.stdout_at_eof = true;
                    }
                }
                _ => {}
            }
        } else {
            count_eof += 1;
        }

        if !self.stderr_at_eof {
            match &self.noblock_stderr {
                Some(noblock_stderr) => {
                    if noblock_stderr.is_eof() {
                        count_eof += 1;

                        self.stderr_at_eof = true;
                    }
                }
                _ => {}
            }
        } else {
            count_eof += 1;
        }

        return count_eof == 2;
    }

    pub fn fill_buffers(&mut self) -> Result<()> {
        if !self.stdout_at_eof {
            if self.noblock_stdout.is_some() {
                self.noblock_stdout
                    .as_mut()
                    .unwrap()
                    .read_available_to_string(&mut self.stdout_buffer)?;
            }
        }

        if !self.stderr_at_eof {
            if self.noblock_stderr.is_some() {
                self.noblock_stderr
                    .as_mut()
                    .unwrap()
                    .read_available_to_string(&mut self.stderr_buffer)?;
            }
        }

        return Ok(());
    }

    fn buffer_vec_line_pos(&mut self, buffer_vec: &mut Vec<char>) -> Option<usize> {
        let mut end: usize = 0;

        loop {
            if end >= buffer_vec.len() {
                return None;
            }

            let c = buffer_vec[end];

            end += 1;

            if c == '\n' || c == '\r' {
                return Some(end);
            }
        }
    }

    fn buffer_vec_extract_lines(
        &mut self,
        buffer_vec: &mut Vec<char>,
        count_lines: i128,
    ) -> (Option<String>, bool) {
        let mut lines = String::new();
        let mut buffer_affected = false;
        let mut extracted: i128 = 0;

        loop {
            let buffer_vec_line_pos_option = self.buffer_vec_line_pos(buffer_vec);

            match buffer_vec_line_pos_option {
                None => break,
                _ => {}
            }

            let lines_vec: Vec<char> = buffer_vec
                .drain(0..buffer_vec_line_pos_option.unwrap())
                .collect();

            lines.push_str(String::from_iter(lines_vec).as_str());

            buffer_affected = true;
            extracted += 1;

            if count_lines != -1 {
                if extracted >= count_lines {
                    break;
                }
            }
        }

        return (Some(lines), buffer_affected);
    }

    fn get_buffer_lines(&mut self, stdout_buffer: bool, count_lines: i128) -> Option<String> {
        let mut buffer_vec: Vec<char>;

        if stdout_buffer {
            buffer_vec = self.stdout_buffer.chars().collect();
        } else {
            buffer_vec = self.stderr_buffer.chars().collect();
        }

        let (lines_option, buffer_affected) =
            self.buffer_vec_extract_lines(&mut buffer_vec, count_lines);

        if buffer_affected {
            let buffer_new = String::from_iter(buffer_vec);

            if stdout_buffer {
                self.stdout_buffer = buffer_new;
            } else {
                self.stderr_buffer = buffer_new;
            }
        }

        return lines_option;
    }

    pub fn get_lines(
        &mut self,
        count_lines: i128,
        fill_buffers: bool,
        trim: bool,
    ) -> (Result<Option<String>>, &Self, bool) {
        if fill_buffers {
            let fill_buffers_result = self.fill_buffers();

            if fill_buffers_result.is_err() {
                return (Err(fill_buffers_result.err().unwrap()), self, false);
            }
        }

        self.stdout_last_used = !self.stdout_last_used;

        if self.stdout_last_used {
            if self.stdout_buffer.is_empty() {
                self.stdout_last_used = false;
            }
        }

        if !self.stdout_last_used {
            if self.stderr_buffer.is_empty() {
                self.stdout_last_used = true;
            }
        }

        if self.stdout_buffer.is_empty() && self.stderr_buffer.is_empty() {
            return (Ok(None), self, self.stdout_last_used);
        }

        let mut lines_option = self.get_buffer_lines(self.stdout_last_used, count_lines);

        if trim {
            lines_option = Self::trim_lines(lines_option);
        }

        return (Ok(lines_option), self, self.stdout_last_used);
    }

    fn trim_lines(lines: Option<String>) -> Option<String> {
        return match lines {
            Some(mut lines) => {
                lines = trim_lines(lines);

                if lines == "" {
                    return None;
                }

                return Some(lines);
            }
            _ => None,
        };
    }

    pub fn get_all_lines(&mut self, fill_buffers: bool, trim: bool) -> Result<Option<String>> {
        let mut lines = String::new();

        if fill_buffers {
            self.fill_buffers()?;
        }

        while !self.is_eof() || self.has_data_in_buffers() {
            match self.get_lines(-1, true, trim).0 {
                Ok(option) => match option {
                    Some(ilines) => lines.push_str(&ilines),
                    None => {}
                },
                Err(e) => return Err(e),
            }

            std::thread::sleep(Duration::from_secs(0));
        }

        return Ok(Some(lines));
    }
}
