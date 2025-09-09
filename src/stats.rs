pub struct Stats {
    pub total_logs: u128,
    pub filtered_out_logs: u128,
    pub printed_logs: u128,
}

impl Stats {
    pub fn new() -> Self {
        return Stats {
            total_logs: 0,
            filtered_out_logs: 0,
            printed_logs: 0,
        };
    }
}
