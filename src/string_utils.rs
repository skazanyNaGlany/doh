use ::anyhow::{Error, Result};
use chrono::{DateTime, Local};
use regex::Regex;
use std::collections::HashMap;

/// Converts a table-like string input into a vector of hashmaps, where each hashmap
/// represents a row of the table with column names as keys and cell values as values.
///
/// # Arguments
///
/// * `input` - A string slice containing the table-like input. The first line is treated
///   as the header, and subsequent lines are treated as rows of the table.
/// * `default` - A string slice representing the default value to use for empty cells.
///
/// # Returns
///
/// A `Vec<HashMap<String, String>>` where each hashmap corresponds to a row in the table.
/// The keys in the hashmap are derived from the header row, and the values are the
/// corresponding cell values from the row. If a cell is empty, the `default` value is used.
///
/// # Example
///
/// ```rust
/// let input = r#"
/// CURRENT   NAME      CLUSTER   AUTHINFO   NAMESPACE
/// *         default   default   default    
///           123       456       789        
///                                        
///           default   default   default    
/// "#;
///
/// let result = table_to_hashmap(input, "N/A");
///
/// assert_eq!(result, vec![
///     {
///         let mut row = HashMap::new();
///         row.insert("CURRENT".to_string(), "*".to_string());
///         row.insert("NAME".to_string(), "default".to_string());
///         row.insert("CLUSTER".to_string(), "default".to_string());
///         row.insert("AUTHINFO".to_string(), "default".to_string());
///         row.insert("NAMESPACE".to_string(), "N/A".to_string());
///         row
///     },
///     {
///         let mut row = HashMap::new();
///         row.insert("CURRENT".to_string(), "N/A".to_string());
///         row.insert("NAME".to_string(), "123".to_string());
///         row.insert("CLUSTER".to_string(), "456".to_string());
///         row.insert("AUTHINFO".to_string(), "789".to_string());
///         row.insert("NAMESPACE".to_string(), "N/A".to_string());
///         row
///     },
///     {
///         let mut row = HashMap::new();
///         row.insert("CURRENT".to_string(), "N/A".to_string());
///         row.insert("NAME".to_string(), "N/A".to_string());
///         row.insert("CLUSTER".to_string(), "N/A".to_string());
///         row.insert("AUTHINFO".to_string(), "N/A".to_string());
///         row.insert("NAMESPACE".to_string(), "N/A".to_string());
///         row
///     },
///     {
///         let mut row = HashMap::new();
///         row.insert("CURRENT".to_string(), "N/A".to_string());
///         row.insert("NAME".to_string(), "default".to_string());
///         row.insert("CLUSTER".to_string(), "default".to_string());
///         row.insert("AUTHINFO".to_string(), "default".to_string());
///         row.insert("NAMESPACE".to_string(), "N/A".to_string());
///         row
///     }
/// ]);
/// ```
///
/// # Notes
///
/// - The function assumes that the input is well-formed, with the header and rows aligned.
/// - If a row is shorter than the header, it is padded with spaces to match the header length.
/// - The `string_utils` module provides helper functions for splitting lines, tokenizing strings,
///   and determining token positions.
pub fn table_to_hashmap(input: &str, default: &str) -> Vec<HashMap<String, String>> {
    let mut result = Vec::new();
    let mut lines = split_lines(input);

    // get and remove first line which should be the header
    let header = lines.remove(0);

    // trim whitespaces from end of the header
    let header = header.trim_end().to_string();

    // tokenize header into labels and get position of each label
    let header_tokens = tokenize(header.as_str());
    let header_tokens_pos = tokens_position(header.as_str(), &header_tokens);

    // here comes the magic - iterate and process rest of the lines
    for iline in lines {
        // remove whitespaces from line
        let mut iline_copy = iline.trim_end().to_string();

        // make sure length of the line is the same as
        // length of the header
        let spaces_needed = header.len() - iline_copy.len();

        if spaces_needed > 0 {
            iline_copy.push_str(&String::from(" ").repeat(spaces_needed));
        }

        // TODO test it
        // add defaults when needed
        for iheader_label in &header_tokens {
            match header_tokens_pos.get(*iheader_label) {
                Some(pos) => match iline_copy.chars().nth(*pos as usize) {
                    Some(char_) => {
                        if char_ == ' ' {
                            let pos_u = *pos as usize;

                            iline_copy.replace_range(pos_u..pos_u + default.len(), default);
                        }
                    }
                    _ => panic!(),
                },
                _ => panic!(),
            }
        }

        // split line by whitespace
        let iline_copy_splitted = tokenize(&iline_copy);

        // this hashmap will hold "label from the header: value from line"
        let mut line_hash: HashMap<String, String> = HashMap::new();

        for (index, iheader_label) in header_tokens.iter().enumerate() {
            line_hash.insert(
                iheader_label.to_string(),
                iline_copy_splitted
                    .get(index)
                    .unwrap()
                    .to_string()
                    .trim()
                    .to_string(),
            );
        }

        result.push(line_hash);
    }

    return result;
}

/// Splits a given string into lines, terminating at newline (`\n`) or carriage return (`\r`) characters.
///
/// # Arguments
///
/// * `s` - A string slice to be split into lines.
///
/// # Returns
///
/// A `Vec<String>` where each element is a line from the input string. The newline or carriage return
/// characters are not included in the resulting lines.
///
/// # Example
///
/// ```rust
/// let input = "line1\nline2\rline3";
/// let lines = split_lines(input);
/// assert_eq!(lines, vec!["line1".to_string(), "line2".to_string(), "line3".to_string()]);
/// ```
///
/// # Notes
///
/// - This function preserves the order of lines as they appear in the input string.
/// - Empty lines are included in the output as empty strings.
pub fn split_lines(s: &str) -> Vec<String> {
    return s
        .split_terminator(['\n', '\r'])
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
}

/// Determines the starting position of each token within a given string.
///
/// # Arguments
///
/// * `s` - A string slice in which the positions of the tokens will be determined.
/// * `tokens` - A vector of string slices representing the tokens whose positions need to be found.
///
/// # Returns
///
/// A `HashMap<String, isize>` where each key is a token (as a `String`) and the value is its starting
/// position (as an `isize`) in the input string `s`. If a token is not found in the string, its position
/// is set to `-1`.
///
/// # Example
///
/// ```rust
/// use std::collections::HashMap;
///
/// let s = "CURRENT NAME CLUSTER AUTHINFO NAMESPACE";
/// let tokens = vec!["CURRENT", "NAME", "CLUSTER", "AUTHINFO", "NAMESPACE"];
/// let positions = tokens_position(s, &tokens);
///
/// let mut expected = HashMap::new();
/// expected.insert("CURRENT".to_string(), 0);
/// expected.insert("NAME".to_string(), 8);
/// expected.insert("CLUSTER".to_string(), 13);
/// expected.insert("AUTHINFO".to_string(), 21);
/// expected.insert("NAMESPACE".to_string(), 30);
///
/// assert_eq!(positions, expected);
/// ```
///
/// # Notes
///
/// - The function uses the `find` method to locate the starting position of each token in the string.
/// - If a token appears multiple times in the string, only the position of its first occurrence is returned.
/// - The function assumes that the input string and tokens are well-formed and does not handle overlapping tokens.
pub fn tokens_position(s: &str, tokens: &Vec<&str>) -> HashMap<String, isize> {
    let mut positions: HashMap<String, isize> = HashMap::new();

    for itoken in tokens {
        match s.find(itoken) {
            Some(pos) => {
                positions.insert(itoken.to_string(), pos as isize);
            }
            None => {
                positions.insert(itoken.to_string(), -1);
            }
        }
    }

    return positions;
}

/// Splits a given string into a vector of substrings (tokens) separated by whitespace.
///
/// # Arguments
///
/// * `s` - A string slice to be tokenized.
///
/// # Returns
///
/// A `Vec<&str>` where each element is a substring (token) from the input string, split by whitespace.
///
/// # Example
///
/// ```rust
/// let input = "CURRENT NAME CLUSTER AUTHINFO NAMESPACE";
/// let tokens = tokenize(input);
/// assert_eq!(tokens, vec!["CURRENT", "NAME", "CLUSTER", "AUTHINFO", "NAMESPACE"]);
/// ```
///
/// # Notes
///
/// - Consecutive whitespace characters are treated as a single delimiter.
/// - Leading and trailing whitespace is ignored.
pub fn tokenize(s: &str) -> Vec<&str> {
    return s.split_whitespace().collect::<Vec<&str>>();
}

pub fn tokenize_by(
    s: &str,
    separator: &str,
    n: isize,
    trim: bool,
    skip_empty: bool,
) -> Vec<String> {
    let mut tokens: Vec<String>;

    if n > -1 {
        tokens = s
            .splitn(n as usize, &separator)
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
    } else {
        tokens = s
            .split(&separator)
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
    }

    if trim {
        let mut trimmed: Vec<String> = vec![];

        for itoken in &tokens {
            trimmed.push(itoken.trim().to_string());
        }

        tokens = trimmed;
    }

    if skip_empty {
        let mut non_empty: Vec<String> = vec![];

        for itoken in &tokens {
            if itoken.is_empty() {
                continue;
            }

            non_empty.push(itoken.to_string());
        }

        tokens = non_empty;
    }

    return tokens;
}

pub fn lines_check_string_exists(lines: &str, s: &str) -> bool {
    let lines_vec = split_lines(&lines);
    let s_tokens = tokenize(s);

    for iline in lines_vec {
        let iline_copy = iline.trim();

        let iline_tokens = tokenize(iline_copy);

        if iline_tokens == s_tokens {
            return true;
        }
    }

    return false;
}

pub fn string_to_bool(s: &str) -> Result<bool> {
    return match s {
        "true" => Ok(true),
        "t" => Ok(true),
        "yes" => Ok(true),
        "y" => Ok(true),
        "1" => Ok(true),
        "false" => Ok(false),
        "f" => Ok(false),
        "no" => Ok(false),
        "n" => Ok(false),
        "0" => Ok(false),
        _ => Err(Error::msg(format!("\"{}\" no boolean value", s))),
    };
}

pub fn _string_to_i128(s: &str) -> Result<i128> {
    return match s.parse::<i128>() {
        Ok(v) => Ok(v),
        _ => Err(Error::msg(format!("\"{}\" no integer value", s))),
    };
}

pub fn trim_lines(lines: String) -> String {
    let mut trimmed = String::new();

    for iline in tokenize_by(&lines, "\n".into(), -1, true, true) {
        let iline_string = iline.to_string() + "\n";

        trimmed.push_str(&iline_string);
    }

    return trimmed.trim().to_string();
}

pub fn _remove_non_alphabetic(s: &String) -> String {
    return s
        .chars()
        .filter(|c| c.is_alphabetic() || *c == ' ')
        .collect();
}

pub fn replace_non_alphabetic_with_space(s: &String) -> String {
    return s
        .chars()
        .map(|c| {
            if c.is_alphabetic() || c.is_alphanumeric() {
                c
            } else {
                ' '
            }
        })
        .collect();
}

/// Normalizes a string by collapsing multiple spaces into a single space and trimming leading/trailing whitespace.
///
/// # Arguments
///
/// * `s` - A string slice to be normalized.
///
/// # Returns
///
/// A `String` with single spaces between tokens and no leading/trailing whitespace.
///
/// # Example
///
/// ```rust
/// let input = "  save   stern defaults false   skip invalid messages false    nginx   ";
/// let normalized = normalize_spaces(input);
/// assert_eq!(normalized, "save stern defaults false skip invalid messages false nginx");
/// ```
pub fn normalize_spaces(s: &String) -> String {
    s.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Generates a string with the current date and time without spaces.
///
/// # Returns
///
/// A `String` containing the current date and time in the format "YYYYMMDD-HHMMSS".
///
/// # Example
///
/// ```rust
/// let timestamp = current_datetime_string();
/// // Returns something like: "20250828-143052"
/// ```
pub fn current_datetime_string(
    date_separator: &String,
    time_date_separator: &String,
    time_separator: &String,
) -> String {
    let now: DateTime<Local> = Local::now();

    return now
        .format(&format!(
            "%Y{}%m{}%d{}%H{}%M{}%S",
            date_separator, date_separator, time_date_separator, time_separator, time_separator,
        ))
        .to_string();
}

pub fn replace_by_regex(s: &String, re: &Regex, to: &String) -> Option<String> {
    match re.find(s) {
        Some(match_str) => Some(s.replacen(match_str.as_str(), to, 1)),
        None => None,
    }
}
