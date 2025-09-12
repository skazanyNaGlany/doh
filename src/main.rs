extern crate jsonxf;

pub mod command_streamer;

mod arg_parser;
mod consts;
mod env_utils;
mod file_utils;
mod json_utils;
mod kubectl;
mod message_regex;
mod stats;
mod stern_json;
mod stern_json_regex;
mod string_utils;

use crate::arg_parser::ArgParser;
use crate::command_streamer::{CommandStreamer, MultiCommandStreamer};
use crate::env_utils::{args_to_string, args_vec};
use crate::file_utils::my_println;
use crate::kubectl::Context;
use crate::message_regex::MessageRegEx;
use crate::stats::Stats;
use crate::string_utils::{
    current_datetime_string, normalize_spaces, replace_by_regex, replace_non_alphabetic_with_space,
    tokenize_by,
};
use anyhow::{Error, Result};
use consts::{APP_NAME, APP_VERSION, BINARY_KUBECTL, BINARY_STERN, BINARY_STERN_URL};
use execution_time::ExecutionTime;
use kubectl::Kubectl;
use realpath::realpath;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::env::set_current_dir;
use std::fs::{canonicalize, File, OpenOptions};
use std::ops::Not;
use std::path::{Path, PathBuf};
use std::time::Duration;
use stern_json::SternJSON;
use stern_json_regex::SternJSONRegEx;
use which::which;

fn get_full_app_name() -> String {
    format!("{} v{}", APP_NAME, APP_VERSION)
}

fn print_app_name(log_handle: &mut Option<File>) -> Result<()> {
    my_println(log_handle, &true, &true, &get_full_app_name())?;
    my_println(log_handle, &true, &true, &"".into())?;

    return Ok(());
}

fn check_required_binaries() -> Result<()> {
    match which(BINARY_KUBECTL) {
        Err(err) => {
            return Err(Error::msg(format!(
                "Make sure \"{}\" exists in your PATH ({})",
                BINARY_KUBECTL, err
            )))
        }
        _ => {}
    }

    match which(BINARY_STERN) {
        Err(err) => {
            return Err(Error::msg(format!(
                "Make sure \"{}\" exists in your PATH, get it from {} ({})",
                BINARY_STERN_URL, BINARY_STERN, err
            )))
        }
        _ => {}
    }

    return Ok(());
}

fn print_app_info() {
    println!("Download logs from one or more Kubernetes contexts.");
    println!("");
}

fn get_app_exe_name() -> String {
    let (args, _) = args_vec(false);

    let full_pathname = args[0].clone();

    return Path::new(&full_pathname)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
}

fn print_usages() {
    println!("Usage:");
    println!("\t{} [option] -- <pod-query>", get_app_exe_name());
    println!("");
    println!("Options:");
    println!("\t-h, --help                                  this help message");
    println!("\t-c, --context <string>[,...]                select context(s) separated by comma or use \"all\" for all contexts (default \"default\")");
    println!("\t-a, --all-at-once <bool>                    gather logs from all contexts at once (default \"false\"); use with caution since it could be heavy to your network infrastructure");
    println!("\t-s, --skip-invalid-messages <bool>          skip invalid messages (default \"false\"); skip non-json messages returned by Stern");
    println!("\t-b, --blank-line-after-entry <bool>         blank line after each log entry (default \"false\")");
    println!("\t-i, --include-container <string>[,...]      include logs from only such container(s); use \"all\" for all containers (default \"all\")");
    println!("\t-f, --save <filename>                       save logs to file, leave empty to auto generate file name");
    println!("\t-w, --work-dir                              set working directory");
    println!("\t-m, --fix-up-messages <bool>                remove some redundant data from each log entry, like timestamps etc. (default \"true\")");
    println!("\t-p, --pretty-print-objects <bool>           pretty print Python like and JSON like objects, experimental (default \"false\")");
    println!("\t-t, --since <duration>                      return logs newer than a relative duration like 5s, 2m, or 3h (default \"1h\")");
    println!("\t-r, --space-after-message <bool>            add a space character after each message (default \"true\")");
    println!("\t-g, --follow                                wait for new messages");
    println!(
        "\t-q, --quiet                                 do not output any log messages to stdout"
    );
    println!("");
    println!("Example:");
    println!("\t{} -- nginx", get_app_exe_name());
    println!("");
}

fn run(args: ArgParser, log_handle: &mut Option<File>) -> Result<()> {
    let mut contexts: Vec<Context> = vec![];
    let arg_context = args.get_kv_arg_string("--context", false, false).unwrap();
    let mut stats = Stats::new();

    if arg_context == "all" {
        contexts = Kubectl::get_contexts(log_handle)?;
    } else {
        for icontext in tokenize_by(&arg_context, ",".into(), -1, true, true) {
            contexts.push(Context {
                auth_info: "".to_string(),
                current: false,
                name: icontext.to_string(),
                cluster: "".to_string(),
                namespace: "".to_string(),
            });
        }
    }

    if !contexts.is_empty() {
        run_level_0(args, &mut contexts, &mut stats, log_handle)?;
    }

    println!("Total logs: {}", stats.total_logs);
    println!("Filtered out logs: {}", stats.filtered_out_logs);
    println!("Printed logs: {}", stats.printed_logs);

    return Ok(());
}

fn create_multi_streamer(
    contexts: &Vec<Context>,
    arg_stern_defaults: bool,
    arg_since: &String,
    arg_ext_args: &Vec<String>,
    arg_follow: &bool,
) -> Result<MultiCommandStreamer> {
    let mut multi_streamer = MultiCommandStreamer::new_empty();

    for icontext in contexts {
        let mut stern_args: Vec<String> = vec![];

        stern_args.push("--context".into());
        stern_args.push(icontext.name.to_string());

        if arg_stern_defaults {
            stern_args.append(&mut vec![
                "--all-namespaces".into(),
                "--output".into(),
                "json".into(),
                "--timestamps=short".into(),
                "--since".into(),
                arg_since.to_string(),
                "--timezone".into(),
                "UTC".into(),
            ]);

            if !*arg_follow {
                stern_args.push("--no-follow".into());
            }
        }

        stern_args.append(&mut arg_ext_args.clone());

        multi_streamer.add(BINARY_STERN, &stern_args, Some(icontext.name.to_string()))?;
    }

    return Ok(multi_streamer);
}

fn generate_log_filename() -> Option<String> {
    let mut args = args_vec(false).0;

    // remove binary pathname
    args.remove(0);

    if args.is_empty() {
        // should not get here since we are requiring
        // that the user must type at least one argument
        // which is a pod-query
        return None;
    }

    let filename =
        normalize_spaces(&replace_non_alphabetic_with_space(&args.join(" "))).replace(" ", "-");

    return Some(format!(
        "{}-{}-{}.txt",
        APP_NAME,
        filename,
        current_datetime_string(&"-".into(), &"-".into(), &"-".into())
    ));
}

fn get_log_filename(args: &ArgParser) -> Result<Option<String>> {
    let mut filename = "".to_string();
    let mut auto_generate = false;

    if args.kv_args.contains_key("--save") {
        filename = args.kv_args.get("--save").unwrap().trim().into();

        if filename.is_empty() {
            auto_generate = true;
        }
    } else if args.args.contains(&"--save".into()) {
        auto_generate = true;
    } else {
        return Ok(None);
    }

    if auto_generate {
        match generate_log_filename() {
            Some(filename2) => filename = filename2,
            None => return Ok(None),
        }
    }

    match realpath(&PathBuf::from(filename)) {
        Ok(filename2) => filename = filename2.to_string_lossy().to_string(),
        Err(e) => return Err(Error::from(e)),
    }

    return Ok(Some(filename));
}

fn open_log_file_handle(args: &ArgParser) -> Result<Option<File>> {
    let pathname = get_log_filename(args)?;

    if let None = pathname {
        return Ok(None);
    }

    return Ok(Some(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(Path::new(&pathname.unwrap()))?,
    ));
}

fn sync_log_file_handle(log_handle: &Option<File>) -> Result<()> {
    if let Some(log_handle) = log_handle {
        log_handle.sync_all()?;
    }

    Ok(())
}

fn run_level_0(
    args: ArgParser,
    contexts: &mut Vec<Context>,
    stats: &mut Stats,
    log_handle: &mut Option<File>,
) -> Result<()> {
    let regex = SternJSONRegEx::new();
    let message_regex = MessageRegEx::new();

    let arg_stern_defaults = args.get_bool_kv_arg("--stern-defaults", false).unwrap();
    let arg_all_contexts_at_once = args.get_bool_kv_arg("--all-at-once", false).unwrap();
    let arg_ext_args = args.ext_args_as_str_vec();
    let arg_quiet = args.args.contains(&"--quiet".into());
    let arg_follow = args.args.contains(&"--follow".into());
    let arg_since: String = args.get_kv_arg_string("--since", false, false).unwrap();

    my_println(
        log_handle,
        &true,
        &true,
        &"Streaming logs from contexts:".into(),
    )?;

    for icontext in contexts.iter() {
        my_println(log_handle, &true, &true, &format!("\t{}", icontext.name))?;
    }

    if arg_all_contexts_at_once {
        let mut multi_streamer = create_multi_streamer(
            contexts,
            arg_stern_defaults,
            &arg_since,
            &arg_ext_args,
            &arg_follow,
        )?;

        gather_logs_from_multi_streamer(
            &args,
            &mut multi_streamer,
            &regex,
            &message_regex,
            &arg_quiet,
            stats,
            log_handle,
        )?;
    } else {
        loop {
            if contexts.is_empty() {
                break;
            }

            let icontext = contexts.remove(0);
            let mut multi_streamer = create_multi_streamer(
                &vec![icontext],
                arg_stern_defaults,
                &arg_since,
                &arg_ext_args,
                &arg_follow,
            )?;

            gather_logs_from_multi_streamer(
                &args,
                &mut multi_streamer,
                &regex,
                &message_regex,
                &arg_quiet,
                stats,
                log_handle,
            )?;
        }
    }

    return Ok(());
}

fn gather_logs_from_multi_streamer(
    args: &ArgParser,
    multi_streamer: &mut MultiCommandStreamer,
    regex: &SternJSONRegEx,
    message_regex: &MessageRegEx,
    arg_quiet: &bool,
    stats: &mut Stats,
    log_handle: &mut Option<File>,
) -> Result<()> {
    let arg_skip_invalid_messages = args
        .get_bool_kv_arg("--skip-invalid-messages", false)
        .unwrap();
    let arg_blank_line_after_entry = args
        .get_bool_kv_arg("--blank-line-after-entry", false)
        .unwrap();
    let arg_space_after_message = args
        .get_bool_kv_arg("--space-after-message", false)
        .unwrap();
    let arg_include_container =
        args.get_kv_arg_array_string("--include-container", ",", false, false);
    let arg_fix_up_messages = args.get_bool_kv_arg("--fix-up-messages", false).unwrap();
    let arg_pretty_print_objects = args
        .get_bool_kv_arg("--pretty-print-objects", false)
        .unwrap();

    for streamer in multi_streamer.get_streamers() {
        my_println(
            log_handle,
            &true,
            &true,
            &format!("Running: {}", streamer.format_command().unwrap()),
        )?;
    }

    for result in multi_streamer.fill_buffers() {
        result?;
    }

    while !multi_streamer.is_eof() || multi_streamer.has_data_in_buffers() {
        let lines = multi_streamer.get_lines(-1, true, true);

        for (ilines, streamer, _) in lines {
            match ilines {
                Ok(ilines) => match ilines {
                    Some(ilines) => {
                        let parsed_lines = SternJSON::parse(&ilines, Some(regex));

                        print_parsed_stern_json(
                            streamer,
                            &parsed_lines,
                            arg_skip_invalid_messages,
                            arg_blank_line_after_entry,
                            &arg_include_container,
                            arg_quiet,
                            &arg_fix_up_messages,
                            &arg_pretty_print_objects,
                            &arg_space_after_message,
                            message_regex,
                            stats,
                            log_handle,
                        )?;
                    }
                    _ => {
                        // did not got any lines this time
                        // but the process is still running
                    }
                },
                Err(err) => {
                    my_println(log_handle, &true, &true, &format!("{}", err))?;
                }
            }
        }

        std::thread::sleep(Duration::from_secs(0));
    }

    return Ok(());
}

fn print_parsed_stern_json(
    streamer: &CommandStreamer,
    parsed_lines: &Vec<SternJSON>,
    arg_skip_invalid_messages: bool,
    arg_blank_line_after_entry: bool,
    arg_include_container: &Option<Vec<String>>,
    arg_quiet: &bool,
    arg_fix_up_messages: &bool,
    arg_pretty_print_objects: &bool,
    arg_space_after_message: &bool,
    message_regex: &MessageRegEx,
    stats: &mut Stats,
    log_handle: &mut Option<File>,
) -> Result<()> {
    let context = streamer.user_data.as_ref().unwrap();

    for ipar in parsed_lines {
        stats.total_logs += 1;

        if !ipar.is_valid {
            if !arg_skip_invalid_messages {
                print_raw_message(
                    &mut ipar.raw.to_string(),
                    arg_blank_line_after_entry,
                    arg_quiet,
                    arg_fix_up_messages,
                    arg_pretty_print_objects,
                    arg_space_after_message,
                    message_regex,
                    log_handle,
                )?;

                stats.printed_logs += 1;
            }

            continue;
        }

        // valid
        if let Some(include_container) = &arg_include_container {
            if !include_container.contains(&ipar.container_name) {
                stats.filtered_out_logs += 1;
                continue;
            }
        }

        let basics = format!(
            "{} {} {} {}    ",
            context, ipar.pod_name, ipar.container_name, ipar.timestamp,
        );

        if let Some(internal_json_message) = &ipar.internal_json_message {
            let mut request_id = None;

            if internal_json_message.contains_key("request_id") {
                request_id = Some(internal_json_message["request_id"].to_string());
            }

            if internal_json_message.contains_key("exc_info")
                && internal_json_message.contains_key("message")
            {
                if !print_json_exc_info_message(
                    &basics,
                    &request_id,
                    internal_json_message,
                    arg_blank_line_after_entry,
                    arg_quiet,
                    arg_fix_up_messages,
                    arg_pretty_print_objects,
                    arg_space_after_message,
                    message_regex,
                    log_handle,
                )? {
                    print_internal_json_message(
                        &basics,
                        internal_json_message,
                        arg_blank_line_after_entry,
                        arg_quiet,
                        log_handle,
                    )?;
                }
            } else if internal_json_message.contains_key("message") {
                if !print_json_message(
                    &basics,
                    internal_json_message,
                    arg_blank_line_after_entry,
                    arg_quiet,
                    arg_fix_up_messages,
                    arg_pretty_print_objects,
                    arg_space_after_message,
                    message_regex,
                    log_handle,
                )? {
                    print_internal_json_message(
                        &basics,
                        internal_json_message,
                        arg_blank_line_after_entry,
                        arg_quiet,
                        log_handle,
                    )?;
                }
            } else if internal_json_message.contains_key("downstream_local_address")
                && internal_json_message.contains_key("method")
                && internal_json_message.contains_key("path")
                && internal_json_message.contains_key("protocol")
                && internal_json_message.contains_key("response_code")
                && internal_json_message.contains_key("bytes_sent")
                && internal_json_message.contains_key("bytes_received")
                && internal_json_message.contains_key("duration")
                && internal_json_message.contains_key("upstream_service_time")
            {
                if !print_json_proxy(
                    &basics,
                    &request_id,
                    internal_json_message,
                    arg_blank_line_after_entry,
                    arg_quiet,
                    log_handle,
                )? {
                    print_internal_json_message(
                        &basics,
                        internal_json_message,
                        arg_blank_line_after_entry,
                        arg_quiet,
                        log_handle,
                    )?;
                }
            } else {
                print_internal_json_message(
                    &basics,
                    internal_json_message,
                    arg_blank_line_after_entry,
                    arg_quiet,
                    log_handle,
                )?;
            }
        } else {
            print_message(
                &basics,
                &mut ipar.message.to_string(),
                arg_blank_line_after_entry,
                arg_quiet,
                arg_fix_up_messages,
                arg_pretty_print_objects,
                arg_space_after_message,
                message_regex,
                log_handle,
            )?;
        }

        stats.printed_logs += 1;
    }

    return Ok(());
}

fn fix_up_message_by_regex(message: &String, message_regex: &MessageRegEx) -> Option<String> {
    // replace by only one regex at a time
    if let Some(fixed_str) = replace_by_regex(message, &message_regex.start_timestamp_1, &"".into())
    {
        return Some(fixed_str.trim().to_string());
    }

    if let Some(fixed_str) = replace_by_regex(message, &message_regex.start_timestamp_2, &"".into())
    {
        return Some(fixed_str.trim().to_string());
    }

    None
}

fn _pretty_print_objects(message: &String, message_regex: &MessageRegEx) -> Option<String> {
    let mut offset = 0;
    let mut fixed = false;
    let mut message_clone = message.to_string();

    while let Some(match_re) = message_regex.mixed_object_1.find_at(&message_clone, offset) {
        let match_str = match_re.as_str();

        if match_str == "{}" {
            // skip empty braces
            offset += 2;
            continue;
        }

        if !match_str.contains("\":") && !match_str.contains("':") {
            // at least one key is required
            offset += match_str.len();
            continue;
        }

        if match_str.len() < 64 {
            // skip small objects
            offset += match_str.len();
            continue;
        }

        if let Ok(fixed_str) = jsonxf::pretty_print(match_str) {
            message_clone = message_clone.replace(match_str, &fixed_str);

            offset += fixed_str.len();
            fixed = true;
        } else {
            offset += match_str.len();
        }
    }

    if fixed {
        Some(message_clone)
    } else {
        None
    }
}

fn fix_up_message(
    message: &String,
    arg_fix_up_messages: &bool,
    arg_pretty_print_objects: &bool,
    arg_space_after_message: &bool,
    message_regex: &MessageRegEx,
) -> Option<String> {
    // TODO make command line parameter to disable/enable
    // fix_up_message_by_regex and pretty_print_objects
    let mut message_clone = message.to_string();
    let mut changed = false;

    if *arg_fix_up_messages {
        if let Some(fixed_message) = fix_up_message_by_regex(&message_clone, message_regex) {
            message_clone = fixed_message;
            changed = true;
        }
    }

    if *arg_pretty_print_objects {
        if let Some(fixed_message) = _pretty_print_objects(&message_clone, message_regex) {
            message_clone = fixed_message;
            changed = true;
        }
    }

    if *arg_space_after_message {
        message_clone = message_clone.trim_end().into();
        message_clone.push(' ');
        changed = true;
    }

    if changed {
        Some(message_clone)
    } else {
        None
    }
}

fn print_raw_message(
    message: &mut String,
    arg_blank_line_after_entry: bool,
    arg_quiet: &bool,
    arg_fix_up_messages: &bool,
    arg_pretty_print_objects: &bool,
    arg_space_after_message: &bool,
    message_regex: &MessageRegEx,
    log_handle: &mut Option<File>,
) -> Result<()> {
    if let Some(formatted_message) = fix_up_message(
        message,
        arg_fix_up_messages,
        arg_pretty_print_objects,
        arg_space_after_message,
        message_regex,
    ) {
        message.clear();
        message.push_str(&formatted_message);
    }

    my_println(log_handle, &true, &arg_quiet.not(), &format!("{}", message))?;

    if arg_blank_line_after_entry {
        my_println(log_handle, &true, &arg_quiet.not(), &"".into())?;
    }

    return Ok(());
}

fn print_internal_json_message(
    basics: &String,
    internal_json_message: &HashMap<String, Value>,
    arg_blank_line_after_entry: bool,
    arg_quiet: &bool,
    log_handle: &mut Option<File>,
) -> Result<()> {
    my_println(
        log_handle,
        &true,
        &arg_quiet.not(),
        &format!("{}{:?}", basics, internal_json_message),
    )?;

    if arg_blank_line_after_entry {
        my_println(log_handle, &true, &arg_quiet.not(), &"".into())?;
    }

    return Ok(());
}

fn print_message(
    basics: &String,
    message: &mut String,
    arg_blank_line_after_entry: bool,
    arg_quiet: &bool,
    arg_fix_up_messages: &bool,
    arg_pretty_print_objects: &bool,
    arg_space_after_message: &bool,
    message_regex: &MessageRegEx,
    log_handle: &mut Option<File>,
) -> Result<()> {
    if let Some(formatted_message) = fix_up_message(
        message,
        arg_fix_up_messages,
        arg_pretty_print_objects,
        arg_space_after_message,
        message_regex,
    ) {
        message.clear();
        message.push_str(&formatted_message);
    }

    my_println(
        log_handle,
        &true,
        &arg_quiet.not(),
        &format!("{}{}", basics, message),
    )?;

    if arg_blank_line_after_entry {
        my_println(log_handle, &true, &arg_quiet.not(), &"".into())?;
    }

    return Ok(());
}

fn print_json_exc_info_message(
    basics: &String,
    request_id: &Option<String>,
    internal_json_message: &HashMap<String, Value>,
    arg_blank_line_after_entry: bool,
    arg_quiet: &bool,
    arg_fix_up_messages: &bool,
    arg_pretty_print_objects: &bool,
    arg_space_after_message: &bool,
    message_regex: &MessageRegEx,
    log_handle: &mut Option<File>,
) -> Result<bool> {
    let mut exc_info = internal_json_message["exc_info"]
        .as_str()
        .unwrap()
        .to_string();
    let mut message = internal_json_message["message"]
        .as_str()
        .unwrap()
        .to_string();

    if let Some(formatted_exc_info) = fix_up_message(
        &exc_info,
        arg_fix_up_messages,
        &false,
        arg_space_after_message,
        message_regex,
    ) {
        exc_info = formatted_exc_info;
    }

    if let Some(formatted_message) = fix_up_message(
        &message,
        arg_fix_up_messages,
        arg_pretty_print_objects,
        arg_space_after_message,
        message_regex,
    ) {
        message = formatted_message;
    }

    let mut line0 = format!("{}{}", basics, exc_info);
    let mut line1 = format!("{}{}", basics, message);

    if let Some(request_id) = request_id {
        line0.push_str(&format!("    (request_id: {})", request_id));
        line1.push_str(&format!("    (request_id: {})", request_id));
    }

    my_println(log_handle, &true, &arg_quiet.not(), &format!("{}", line0))?;
    my_println(log_handle, &true, &arg_quiet.not(), &format!("{}", line1))?;

    if arg_blank_line_after_entry {
        my_println(log_handle, &true, &arg_quiet.not(), &"".into())?;
    }

    return Ok(true);
}

fn print_json_proxy(
    basics: &String,
    request_id: &Option<String>,
    internal_json_message: &HashMap<String, Value>,
    arg_blank_line_after_entry: bool,
    arg_quiet: &bool,
    log_handle: &mut Option<File>,
) -> Result<bool> {
    let downstream_local_address = internal_json_message["downstream_local_address"]
        .as_str()
        .unwrap();
    let method = internal_json_message["method"].as_str().unwrap();
    let path = internal_json_message["path"].as_str().unwrap();
    let protocol = internal_json_message["protocol"].as_str().unwrap();
    let response_code = internal_json_message["response_code"].as_str().unwrap();
    let bytes_sent = internal_json_message["bytes_sent"].as_str().unwrap();
    let bytes_received = internal_json_message["bytes_received"].as_str().unwrap();
    let duration = internal_json_message["duration"].as_str().unwrap();
    let upstream_service_time = internal_json_message["upstream_service_time"]
        .as_str()
        .unwrap();

    let mut line0 = format!(
        "{}{} \"{} {} {}\" {}, {} {}, {} {}",
        basics,
        downstream_local_address,
        method,
        path,
        protocol,
        response_code,
        bytes_sent,
        bytes_received,
        duration,
        upstream_service_time
    );

    if let Some(request_id) = request_id {
        line0.push_str(&format!("    (request_id: {})", request_id));
    }

    my_println(log_handle, &true, &arg_quiet.not(), &format!("{}", line0))?;

    if arg_blank_line_after_entry {
        my_println(log_handle, &true, &arg_quiet.not(), &"".into())?;
    }

    return Ok(true);
}

fn print_json_message(
    basics: &String,
    internal_json_message: &HashMap<String, Value>,
    arg_blank_line_after_entry: bool,
    arg_quiet: &bool,
    arg_fix_up_messages: &bool,
    arg_pretty_print_objects: &bool,
    arg_space_after_message: &bool,
    message_regex: &MessageRegEx,
    log_handle: &mut Option<File>,
) -> Result<bool> {
    let mut message = internal_json_message["message"]
        .as_str()
        .unwrap()
        .to_string();

    if let Some(formatted_message) = fix_up_message(
        &message,
        arg_fix_up_messages,
        arg_pretty_print_objects,
        arg_space_after_message,
        message_regex,
    ) {
        message = formatted_message;
    }

    my_println(
        log_handle,
        &true,
        &arg_quiet.not(),
        &format!("{}{}", basics, message),
    )?;

    if arg_blank_line_after_entry {
        my_println(log_handle, &true, &arg_quiet.not(), &"".into())?;
    }

    return Ok(true);
}

fn parse_args() -> Result<ArgParser> {
    let parsed = ArgParser::new(
        &vec![
            "--context",
            "-c",
            "--stern-defaults",
            "-d",
            "--all-at-once",
            "-a",
            "--skip-invalid-messages",
            "-s",
            "--blank-line-after-entry",
            "-b",
            "--include-container",
            "-i",
            "--save",
            "-f",
            "--work-dir",
            "-w",
            "--fix-up-messages",
            "-m",
            "--pretty-print-objects",
            "-p",
            "--since",
            "-t",
            "--space-after-message",
            "-r",
        ],
        &vec![
            "--help", "-h", "--save", "-f", "--quiet", "-q", "--follow", "-g",
        ],
        &vec![],
        &vec![],
        &vec![
            &["--context", "-c"],
            &["--help", "-h"],
            &["--stern-defaults", "-d"],
            &["--all-at-once", "-a"],
            &["--skip-invalid-messages", "-s"],
            &["--blank-line-after-entry", "-b"],
            &["--include-container", "-i"],
            &["--save", "-f"],
            &["--quiet", "-q"],
            &["--work-dir", "-w"],
            &["--fix-up-messages", "-m"],
            &["--pretty-print-objects", "-p"],
            &["--since", "-t"],
            &["--space-after-message", "-r"],
            &["--follow", "-g"],
        ],
        &vec![],
        BTreeMap::from([
            ("--context", "default"),
            ("--stern-defaults", "true"),
            ("--all-at-once", "false"),
            ("--skip-invalid-messages", "false"),
            ("--blank-line-after-entry", "false"),
            ("--fix-up-messages", "true"),
            ("--pretty-print-objects", "false"),
            ("--since", "1h"),
            ("--space-after-message", "true"),
        ]),
        &vec![],
        BTreeMap::from([]),
        &vec![],
        false,
        true,
        false,
        false,
    );

    parsed.get_bool_kv_arg("--stern-defaults", false)?;
    parsed.get_bool_kv_arg("--all-at-once", false)?;
    parsed.get_bool_kv_arg("--skip-invalid-messages", false)?;
    parsed.get_bool_kv_arg("--blank-line-after-entry", false)?;
    parsed.get_bool_kv_arg("--fix-up-messages", false)?;
    parsed.get_bool_kv_arg("--pretty-print-objects", false)?;
    parsed.get_bool_kv_arg("--space-after-message", false)?;

    if !parsed.unknown_args.is_empty() {
        return Err(Error::msg(format!(
            "Unknown parameter(s): {:?}",
            parsed.unknown_args
        )));
    }

    return Ok(parsed);
}

fn clean_args(args: &mut ArgParser) {
    if let Some(include_container) = args.kv_args.get("--include-container") {
        if include_container == "all" {
            args.kv_args.remove("--include-container");
        }
    }
}

fn should_print_usages(args: &ArgParser) -> bool {
    return args.args.contains(&"--help".to_string()) || args.ext_args.is_empty();
}

fn _set_current_dir(arg_work_dir: &Option<String>) -> Result<Option<String>> {
    match arg_work_dir {
        Some(arg_work_dir) => {
            let path = &canonicalize(arg_work_dir)?;
            set_current_dir(path)?;

            return Ok(Some(path.to_string_lossy().to_string()));
        }
        None => Ok(None),
    }
}

fn main() -> Result<()> {
    check_required_binaries()?;

    let mut args = parse_args()?;

    clean_args(&mut args);

    if should_print_usages(&args) {
        print_app_name(&mut None)?;
        print_app_info();
        print_usages();

        return Ok(());
    }

    let arg_work_dir = args.get_kv_arg_string("--work-dir", false, false);

    let work_dir = _set_current_dir(&arg_work_dir)?;
    let mut log_handle = open_log_file_handle(&args)?;
    let log_pathname = get_log_filename(&args)?;

    print_app_name(&mut log_handle)?;

    let timer = ExecutionTime::start();

    my_println(
        &mut log_handle,
        &true,
        &true,
        &format!(
            "Started at: {}",
            current_datetime_string(&"-".into(), &" ".into(), &":".into())
        ),
    )?;
    my_println(
        &mut log_handle,
        &true,
        &true,
        &format!("Command line: {}", args_to_string()),
    )?;

    if let Some(work_dir) = work_dir {
        my_println(
            &mut log_handle,
            &true,
            &true,
            &format!("Working directory: {}", work_dir),
        )?;
    }

    if let Some(log_pathname) = log_pathname {
        my_println(
            &mut log_handle,
            &true,
            &true,
            &format!("Saving logs to: {}", log_pathname),
        )?;
    }

    let result = run(args, &mut log_handle);

    my_println(
        &mut log_handle,
        &true,
        &true,
        &format!(
            "Done at: {}",
            current_datetime_string(&"-".into(), &" ".into(), &":".into())
        ),
    )?;
    my_println(
        &mut log_handle,
        &true,
        &true,
        &format!("Execution time: {}", timer.get_elapsed_time()),
    )?;

    sync_log_file_handle(&mut log_handle)?;

    result
}
