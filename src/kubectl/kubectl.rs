use std::fs::File;

use super::consts::BINARY_KUBECTL;
use super::Context;
use crate::command_streamer::MultiCommandStreamer;
use crate::file_utils::my_println;
use crate::string_utils::{lines_check_string_exists, table_to_hashmap};
use anyhow::{Error, Result};

pub struct Kubectl {}

impl Kubectl {
    pub fn get_contexts(log_handle: &mut Option<File>) -> Result<Vec<Context>> {
        let mut contexts = Vec::new();
        let mut multi_streamer = MultiCommandStreamer::new_empty();
        let mut lines = String::new();

        my_println(
            log_handle,
            &true,
            &true,
            &"Getting Kubernetes contexts".into(),
        )?;

        multi_streamer.add(
            BINARY_KUBECTL,
            &vec!["config".into(), "get-contexts".into()],
            None,
        )?;

        match multi_streamer.get_all_lines(true, false).remove(0) {
            Ok(option) => match option {
                Some(iline) => lines.push_str(&iline),
                None => {}
            },
            Err(e) => return Err(e),
        }

        if !lines_check_string_exists(&lines, "CURRENT   NAME      CLUSTER   AUTHINFO   NAMESPACE")
        {
            return Err(Error::msg(
                "\"kubectl config get-contexts\" returns no header, cannot get Kubernetes contexts",
            ));
        }

        let lines_table = table_to_hashmap(&lines, "N/A");

        for irow in lines_table {
            if !irow.contains_key("CURRENT")
                || !irow.contains_key("NAME")
                || !irow.contains_key("CLUSTER")
                || !irow.contains_key("AUTHINFO")
                || !irow.contains_key("NAMESPACE")
            {
                continue;
            }

            let current = irow.get("CURRENT").unwrap() == "*";
            let mut name = irow.get("NAME").unwrap().to_string().trim().to_string();
            let mut cluster = irow.get("CLUSTER").unwrap().to_string().trim().to_string();
            let mut auth_info = irow.get("AUTHINFO").unwrap().to_string().trim().to_string();
            let mut namespace = irow
                .get("NAMESPACE")
                .unwrap()
                .to_string()
                .trim()
                .to_string();

            if name == "N/A" {
                name = "".to_string();
            }

            if cluster == "N/A" {
                cluster = "".to_string();
            }

            if auth_info == "N/A" {
                auth_info = "".to_string();
            }

            if namespace == "N/A" {
                namespace = "".to_string();
            }

            let context = Context {
                current,
                name,
                cluster,
                auth_info,
                namespace,
            };

            contexts.push(context);
        }

        if !contexts.is_empty() {
            my_println(
                log_handle,
                &true,
                &true,
                &"Found Kubernetes contexts:".into(),
            )?;

            for icontext in &contexts {
                let context_data = format!(
                    "\tname: {}, cluster: {}, auth_info: {}, namespace: {}, current: {}",
                    icontext.name,
                    icontext.cluster,
                    icontext.auth_info,
                    icontext.namespace,
                    icontext.current
                );

                my_println(log_handle, &true, &true, &context_data)?;
            }
        } else {
            my_println(
                log_handle,
                &true,
                &true,
                &"No Kubernetes contexts found".into(),
            )?;
        }

        return Ok(contexts);
    }
}
