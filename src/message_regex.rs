use regex::Regex;

// 20250902140313.122[ERR][service.views, function (file.py:618)][NULL]: message ...
// 20250902140313.474[INF][profiler, __call__ (file.py:70)][NULL]: message ...
const START_TIMESTAMP_1: &str = r"^(\d{14}\.?\d{0,3}\[[A-Z]+\]\[.*?\]\[[A-Z]+\]:\s+)";

// 2025-09-02 12:58:52.123 INFO [140358121944832] HandlerBase:61 | [persistent://cloud/events, ] message ...
const START_TIMESTAMP_2: &str = r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.?\d{0,3} [A-Z]+\s+)";

const MIXED_OBJECT_1: &str = r"\{(?:[^{}]|(?R))*\}";

pub(crate) struct MessageRegEx {
    pub(crate) start_timestamp_1: Regex,
    pub(crate) start_timestamp_2: Regex,
    pub(crate) mixed_object_1: Regex,
}

impl MessageRegEx {
    pub(crate) fn new() -> Self {
        return MessageRegEx {
            start_timestamp_1: Regex::new(START_TIMESTAMP_1).unwrap(),
            start_timestamp_2: Regex::new(START_TIMESTAMP_2).unwrap(),
            mixed_object_1: Regex::new(MIXED_OBJECT_1).unwrap(),
        };
    }
}
