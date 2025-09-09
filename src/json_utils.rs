use anyhow::Result;
use std::collections::HashMap;

use serde_json::Value;

/// Parses a JSON string into a `HashMap<String, Value>`.
///
/// # Arguments
///
/// * `json` - A string slice containing a JSON object.
///
/// # Returns
///
/// * `Result<HashMap<String, Value>>` - A result containing the parsed hashmap on success,
///   or an error if the JSON is invalid.
///
/// # Examples
///
/// ```
/// let parsed = json_to_hashmap("{\"name\":\"John\", \"age\":30, \"car\":null}").unwrap();
/// println!("parsed: {:?}", parsed);
/// ```
///
/// # Errors
///
/// Returns an error if the input string is not valid JSON or cannot be parsed into a hashmap.
pub fn json_to_hashmap(json: &str) -> Result<HashMap<String, Value>> {
    let parsed: HashMap<String, Value> = serde_json::from_str(json)?;

    return Ok(parsed);
}
