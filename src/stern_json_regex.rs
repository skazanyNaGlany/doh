use regex::Regex;

const FULL_TIMESTAMP_AND_MESSAGE: &str = r"^(?P<full_timestamp>(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})T(?P<hour>\d{2}):(?P<minute>\d{2}):(?P<second>\d{2})(?:\.(?P<nanoseconds>\d{1,9}))?(?P<tz_sign>[+-])(?P<tz_hour>\d{2}):(?P<tz_minute>\d{2})) ?(?P<message>.*)$";
const SHORT_TIMESTAMP_AND_MESSAGE: &str =
    r"^(?P<short_timestamp>\d{2}-\d{2} \d{2}:\d{2}:\d{2}) ?(?P<message>.*)$";

pub(crate) struct SternJSONRegEx {
    pub(crate) full_timestamp_and_message: Regex, // 2021-08-26T21:52:09+02:00 message
    pub(crate) short_timestamp_and_message: Regex, // 08-26 22:08:51 message
}

impl SternJSONRegEx {
    pub(crate) fn new() -> Self {
        return SternJSONRegEx {
            full_timestamp_and_message: Regex::new(FULL_TIMESTAMP_AND_MESSAGE).unwrap(),
            short_timestamp_and_message: Regex::new(SHORT_TIMESTAMP_AND_MESSAGE).unwrap(),
        };
    }
}
