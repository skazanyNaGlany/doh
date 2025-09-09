use super::CommandStreamer;
use anyhow::Result;

pub struct MultiCommandStreamer {
    streamers: Vec<CommandStreamer>,
}

impl MultiCommandStreamer {
    pub fn new_empty() -> Self {
        return MultiCommandStreamer { streamers: vec![] };
    }

    pub fn new(program: &str, args: &Vec<String>, user_data: Option<String>) -> Result<Self> {
        return Ok(MultiCommandStreamer {
            streamers: vec![CommandStreamer::new(program, args, user_data)?],
        });
    }

    pub fn new_from_streamer(streamer: CommandStreamer) -> Result<Option<Self>> {
        return Ok(Some(MultiCommandStreamer {
            streamers: vec![streamer],
        }));
    }

    pub fn add_streamer(&mut self, streamer: CommandStreamer) {
        self.streamers.push(streamer);
    }

    pub fn add(
        &mut self,
        program: &str,
        args: &Vec<String>,
        user_data: Option<String>,
    ) -> Result<()> {
        self.streamers
            .push(CommandStreamer::new(program, args, user_data)?);

        return Ok(());
    }

    pub fn get_streamers(&mut self) -> &mut Vec<CommandStreamer> {
        return &mut self.streamers;
    }

    pub fn is_eof(&mut self) -> bool {
        let mut count_eof = 0;

        for streamer in self.streamers.iter_mut() {
            if streamer.is_eof() {
                count_eof += 1;
            }
        }

        return count_eof == self.streamers.len();
    }

    pub fn has_data_in_buffers(&self) -> bool {
        for streamer in self.streamers.iter() {
            if streamer.has_data_in_buffers() {
                return true;
            }
        }

        return false;
    }

    pub fn fill_buffers(&mut self) -> Vec<Result<()>> {
        let mut results = vec![];

        for streamer in self.streamers.iter_mut() {
            results.push(streamer.fill_buffers());
        }

        return results;
    }

    pub fn get_lines(
        &mut self,
        count_lines: i128,
        fill_buffers: bool,
        trim: bool,
    ) -> Vec<(Result<Option<String>>, &CommandStreamer, bool)> {
        let mut results = vec![];

        for streamer in self.streamers.iter_mut() {
            results.push(streamer.get_lines(count_lines, fill_buffers, trim));
        }

        return results;
    }

    pub fn get_all_lines(&mut self, fill_buffers: bool, trim: bool) -> Vec<Result<Option<String>>> {
        let mut results = vec![];

        for streamer in self.streamers.iter_mut() {
            results.push(streamer.get_all_lines(fill_buffers, trim));
        }

        return results;
    }
}
