use anyhow::{Error, Result};
use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
};

use crate::{
    env_utils::args_vec,
    string_utils::{_string_to_i128, string_to_bool, tokenize_by},
};

pub struct ArgParser {
    pub kv_args: BTreeMap<String, String>,
    pub args: Vec<String>,
    pub main_arg: String,
    pub ext_kv_args: BTreeMap<String, String>,
    pub ext_args: Vec<String>,
    pub ext_main_arg: String,
    pub unknown_args: Vec<String>,
}

impl ArgParser {
    pub fn new(
        supported_kv_args: &[&str],
        supported_args: &[&str],
        supported_kv_ext_args: &[&str],
        supported_ext_args: &[&str],
        args_to_merge: &[&[&str; 2]],
        ext_args_to_merge: &[&[&str; 2]],
        default_kv_args: BTreeMap<&str, &str>,
        default_args: &[&str],
        default_kv_ext_args: BTreeMap<&str, &str>,
        default_ext_args: &[&str],
        support_all_args: bool,
        support_all_ext_args: bool,
        support_main_arg: bool,
        support_main_ext_arg: bool,
    ) -> Self {
        let parsed_args = Self::parse_args(
            supported_kv_args,
            supported_args,
            supported_kv_ext_args,
            supported_ext_args,
            args_to_merge,
            ext_args_to_merge,
            default_kv_args,
            default_args,
            default_kv_ext_args,
            default_ext_args,
            support_all_args,
            support_all_ext_args,
            support_main_arg,
            support_main_ext_arg,
        );

        return ArgParser {
            kv_args: parsed_args.0 .0,
            args: parsed_args.0 .1,
            main_arg: parsed_args.0 .2,
            ext_kv_args: parsed_args.1 .0,
            ext_args: parsed_args.1 .1,
            ext_main_arg: parsed_args.1 .2,
            unknown_args: parsed_args.2,
        };
    }

    pub fn _args_as_str_vec(&self) -> Vec<&str> {
        return self.args.iter().map(|s| s.as_str()).collect();
    }

    pub fn ext_args_as_str_vec(&self) -> Vec<String> {
        return self.ext_args.iter().map(|s| s.to_string()).collect();
    }

    pub fn to_string(&self) -> String {
        return format!(
            "kv_args={:?}, args={:?}, main_arg={:?}, ext_kv_args={:?}, ext_args={:?}, ext_main_arg={:?}, unknown_args={:?}",
            self.kv_args, self.args, self.main_arg, self.ext_kv_args, self.ext_args, self.ext_main_arg, self.unknown_args
        );
    }

    pub fn get_kv_arg_array_string(
        &self,
        name: &str,
        separator: &str,
        should_panic: bool,
        ext: bool,
    ) -> Option<Vec<String>> {
        let value;

        if ext {
            value = self.ext_kv_args.get(name);
        } else {
            value = self.kv_args.get(name);
        }

        match value {
            Some(value) => {
                return Some(tokenize_by(&value, separator, -1, true, true));
            }
            None => {
                let msg = format!("key \"{}\" is missing", name);

                if should_panic {
                    panic!("{}", msg);
                } else {
                    return None;
                }
            }
        }
    }

    pub fn get_kv_arg_string(&self, name: &str, should_panic: bool, ext: bool) -> Option<String> {
        let value;

        if ext {
            value = self.ext_kv_args.get(name);
        } else {
            value = self.kv_args.get(name);
        }

        match value {
            Some(value) => Some(value.trim().to_string()),
            None => {
                let msg = format!("key \"{}\" is missing", name);

                if should_panic {
                    panic!("{}", msg);
                } else {
                    return None;
                }
            }
        }
    }

    fn get_kv_arg_bool(&self, name: &str, should_panic: bool, ext: bool) -> Result<bool> {
        let value;

        if ext {
            value = self.ext_kv_args.get(name);
        } else {
            value = self.kv_args.get(name);
        }

        match value {
            Some(value) => match string_to_bool(&value.trim().to_lowercase()) {
                Ok(b) => Ok(b),
                Err(e) => {
                    if should_panic {
                        panic!("{}", e);
                    } else {
                        return Err(e);
                    }
                }
            },
            None => {
                let msg = format!("key \"{}\" is missing", name);

                if should_panic {
                    panic!("{}", msg);
                } else {
                    return Err(Error::msg(msg));
                }
            }
        }
    }

    fn _get_kv_arg_i128(&self, name: &str, should_panic: bool, ext: bool) -> Result<i128> {
        let value;

        if ext {
            value = self.ext_kv_args.get(name);
        } else {
            value = self.kv_args.get(name);
        }

        match value {
            Some(value) => match _string_to_i128(&value.trim().to_lowercase()) {
                Ok(v) => Ok(v),
                Err(e) => {
                    if should_panic {
                        panic!("{}", e);
                    } else {
                        return Err(e);
                    }
                }
            },
            None => {
                let msg = format!("key \"{}\" is missing", name);

                if should_panic {
                    panic!("{}", msg);
                } else {
                    return Err(Error::msg(msg));
                }
            }
        }
    }

    pub fn get_bool_kv_arg(&self, name: &str, should_panic: bool) -> Result<bool> {
        return self.get_kv_arg_bool(name, should_panic, false);
    }

    pub fn _get_bool_ext_kv_arg(&self, name: &str, should_panic: bool) -> Result<bool> {
        return self.get_kv_arg_bool(name, should_panic, true);
    }

    pub fn _get_i128_kv_arg(&self, name: &str, should_panic: bool) -> Result<i128> {
        return self._get_kv_arg_i128(name, should_panic, false);
    }

    pub fn _get_i128_ext_kv_arg(&self, name: &str, should_panic: bool) -> Result<i128> {
        return self._get_kv_arg_i128(name, should_panic, true);
    }

    fn parse_args(
        supported_kv_args: &[&str],
        supported_args: &[&str],
        supported_kv_ext_args: &[&str],
        supported_ext_args: &[&str],
        args_to_merge: &[&[&str; 2]],
        ext_args_to_merge: &[&[&str; 2]],
        default_kv_args: BTreeMap<&str, &str>,
        default_args: &[&str],
        default_kv_ext_args: BTreeMap<&str, &str>,
        default_ext_args: &[&str],
        support_all_args: bool,
        support_all_ext_args: bool,
        support_main_arg: bool,
        support_main_ext_arg: bool,
    ) -> (
        (BTreeMap<String, String>, Vec<String>, String),
        (BTreeMap<String, String>, Vec<String>, String),
        Vec<String>,
    ) {
        let mut parsed_kv_args: BTreeMap<String, String> = BTreeMap::new();
        let mut parsed_args = vec![];
        let mut parsed_ext_kv_args: BTreeMap<String, String> = BTreeMap::new();
        let mut parsed_ext_args = vec![];
        let mut parsed_main_arg = String::new();
        let mut parsed_main_ext_arg = String::new();
        let mut unknown_args = vec![];

        let (mut args, mut args_ext) = args_vec(true);

        // remove executable pathname
        args.remove(0);

        if support_main_arg {
            if !args.is_empty() {
                parsed_main_arg = args.remove(args.len() - 1);
            }
        }

        if support_all_args {
            parsed_args.extend_from_slice(&args);

            args.clear();
        } else {
            Self::process_fill_args(
                supported_kv_args,
                supported_args,
                &mut args,
                &mut parsed_kv_args,
                &mut parsed_args,
            );
        }

        if support_main_ext_arg {
            if !args_ext.is_empty() {
                parsed_main_ext_arg = args_ext.remove(args_ext.len() - 1);
            }
        }

        if support_all_ext_args {
            parsed_ext_args.extend_from_slice(&args_ext);

            args_ext.clear();
        } else {
            Self::process_fill_args(
                supported_kv_ext_args,
                supported_ext_args,
                &mut args_ext,
                &mut parsed_ext_kv_args,
                &mut parsed_ext_args,
            );
        }

        Self::add_defaults(
            default_kv_args,
            default_args,
            &mut parsed_kv_args,
            &mut parsed_args,
        );

        Self::add_defaults(
            default_kv_ext_args,
            default_ext_args,
            &mut parsed_ext_kv_args,
            &mut parsed_ext_args,
        );

        Self::merge_args(args_to_merge, &mut parsed_kv_args, &mut parsed_args);
        Self::merge_args(
            ext_args_to_merge,
            &mut parsed_ext_kv_args,
            &mut parsed_ext_args,
        );

        unknown_args.extend_from_slice(&args);
        unknown_args.extend_from_slice(&args_ext);

        return (
            (parsed_kv_args, parsed_args, parsed_main_arg),
            (parsed_ext_kv_args, parsed_ext_args, parsed_main_ext_arg),
            unknown_args,
        );
    }

    fn process_fill_args(
        supported_kv_args: &[&str],
        supported_args: &[&str],
        args: &mut Vec<String>,
        parsed_kv_args: &mut BTreeMap<String, String>,
        parsed_args: &mut Vec<String>,
    ) {
        for iarg in args.clone() {
            if supported_args.contains(&iarg.as_str()) {
                parsed_args.push(iarg.to_string());

                args.remove(args.iter().position(|i| *i == iarg).unwrap());
            }
        }

        for supported_arg in supported_kv_args {
            let mut supported_arg_postfix = supported_arg.to_string();
            supported_arg_postfix.push_str("=");

            for (iarg_index, iarg) in args.clone().iter().enumerate() {
                if iarg == supported_arg {
                    if args.len() > iarg_index + 1 {
                        args.remove(iarg_index);

                        let value = args.remove(iarg_index);

                        parsed_kv_args.insert(supported_arg.to_string(), value);
                    }
                } else if iarg.starts_with(&supported_arg_postfix) {
                    let iarg_parts = iarg
                        .splitn(2, "=")
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>();

                    match iarg_parts.get(1) {
                        Some(iarg_part_1) => {
                            parsed_kv_args
                                .insert(supported_arg.to_string(), iarg_part_1.to_string());
                        }
                        None => continue,
                    }

                    args.remove(iarg_index);
                }
            }
        }
    }

    fn add_defaults(
        default_kv_args: BTreeMap<&str, &str>,
        default_args: &[&str],
        parsed_kv_args: &mut BTreeMap<String, String>,
        parsed_args: &mut Vec<String>,
    ) {
        for (iarg, iarg_value) in default_kv_args {
            if !parsed_kv_args.contains_key(iarg) {
                parsed_kv_args.insert(iarg.to_string(), iarg_value.to_string());
            }
        }

        for iarg in default_args {
            if parsed_args.contains(&iarg.to_string()) {
                parsed_args.push(iarg.to_string());
            }
        }
    }

    fn merge_args(
        merge_args: &[&[&str; 2]],
        parsed_kv_args: &mut BTreeMap<String, String>,
        parsed_args: &mut Vec<String>,
    ) {
        for to_merge_args in merge_args {
            for (iparsed_arg_index, iparsed_arg) in parsed_args.clone().iter().enumerate() {
                if to_merge_args[1] == iparsed_arg {
                    parsed_args.remove(iparsed_arg_index);
                    parsed_args.push(to_merge_args[0].to_string());

                    break;
                }
            }

            for (iparsed_arg, iparsed_arg_value) in parsed_kv_args.clone() {
                if to_merge_args[1] == iparsed_arg {
                    parsed_kv_args.remove(&iparsed_arg);
                    parsed_kv_args.insert(to_merge_args[0].to_string(), iparsed_arg_value);

                    break;
                }
            }
        }
    }
}

impl Debug for ArgParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParsedArgs")
            .field("kv_args", &self.kv_args)
            .field("args", &self.args)
            .field("main_arg", &self.main_arg)
            .field("ext_kv_args", &self.ext_kv_args)
            .field("ext_args", &self.ext_args)
            .field("ext_main_arg", &self.ext_main_arg)
            .field("unknown_args", &self.unknown_args)
            .finish()
    }
}

impl Display for ArgParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.write_str(&self.to_string());
    }
}
