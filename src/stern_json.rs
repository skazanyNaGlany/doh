use serde_json::Value;
use std::collections::HashMap;

use crate::stern_json_regex::SternJSONRegEx;
use crate::{json_utils::json_to_hashmap, string_utils::tokenize_by};

#[derive(Debug)]
pub struct SternJSON {
    // same names as exists in the json
    pub timestamp: String,      // timestamp extracted from "message"
    pub message: String,        // message
    pub node_name: String,      // nodeName
    pub namespace: String,      // namespace
    pub pod_name: String,       // podName
    pub container_name: String, // containerName

    pub is_valid: bool,
    pub raw: String,
    pub internal_json_message: Option<HashMap<String, Value>>, // parsed json from "message"
}

impl SternJSON {
    pub fn parse(lines: &str, regex: Option<&SternJSONRegEx>) -> Vec<Self> {
        let mut parsed = vec![];

        for iline in tokenize_by(lines, "\n".into(), -1, true, true) {
            let mut json = SternJSON {
                timestamp: "".to_string(),
                message: "".to_string(),
                node_name: "".to_string(),
                namespace: "".to_string(),
                pod_name: "".to_string(),
                container_name: "".to_string(),
                is_valid: false,
                raw: iline.to_string(),
                internal_json_message: None,
            };

            if iline.starts_with("{") && iline.ends_with("}") {
                match json_to_hashmap(&iline) {
                    Ok(hashmap) => Self::fill_from_hashmap(&mut json, hashmap, regex),
                    _ => {}
                }
            } else {
            }

            parsed.push(json);
        }

        return parsed;
    }

    fn fill_from_hashmap(
        json: &mut SternJSON,
        hashmap: HashMap<String, Value>,
        regex: Option<&SternJSONRegEx>,
    ) {
        json.is_valid = false;

        if !hashmap.contains_key("message")
            || !hashmap.contains_key("nodeName")
            || !hashmap.contains_key("namespace")
            || !hashmap.contains_key("podName")
            || !hashmap.contains_key("containerName")
        {
            return;
        }

        json.message = hashmap["message"].as_str().unwrap().trim().to_string();
        json.node_name = hashmap["nodeName"].as_str().unwrap().trim().to_string();
        json.namespace = hashmap["namespace"].as_str().unwrap().trim().to_string();
        json.pod_name = hashmap["podName"].as_str().unwrap().trim().to_string();
        json.container_name = hashmap["containerName"]
            .as_str()
            .unwrap()
            .trim()
            .to_string();

        json.is_valid = true;

        Self::fill_internal_json_message(json, regex);
    }

    fn fill_internal_json_message(json: &mut SternJSON, regex: Option<&SternJSONRegEx>) {
        if json.message.starts_with("{") && json.message.ends_with("}") {
            // plain json message, just parse it
            match json_to_hashmap(&json.message) {
                Ok(parsed_json) => json.internal_json_message = Some(parsed_json),
                _ => {}
            }
        } else {
            // try to find and extract "timestamp" and "message"
            if let Some(regex) = regex {
                Self::extract_ts_message_internal_message(json, regex);
            }
        }
    }

    fn extract_ts_message_internal_message(json: &mut SternJSON, regex: &SternJSONRegEx) {
        if let Some(parsed) = regex.full_timestamp_and_message.captures(&json.message) {
            json.timestamp = parsed["full_timestamp"].to_string().trim().to_string();
            json.message = parsed["message"].to_string().trim().to_string();
        } else if let Some(parsed) = regex.short_timestamp_and_message.captures(&json.message) {
            json.timestamp = parsed["short_timestamp"].to_string().trim().to_string();
            json.message = parsed["message"].to_string().trim().to_string();
        }

        if !json.message.is_empty() {
            if let Ok(parsed_json) = json_to_hashmap(&json.message) {
                json.internal_json_message = Some(parsed_json);
            }
        }
    }
}
