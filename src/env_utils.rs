use std::env::args;

pub fn args_vec(split_by_dash: bool) -> (Vec<String>, Vec<String>) {
    let mut result_args: Vec<String> = vec![];
    let mut result_args_ext: Vec<String> = vec![];
    let mut found_ext_args = false;
    let args = args().collect::<Vec<String>>();

    if !split_by_dash {
        return (args, vec![]);
    }

    for iarg in args {
        if iarg == "--" {
            found_ext_args = true;
            continue;
        }

        if found_ext_args {
            result_args_ext.push(iarg);
        } else {
            result_args.push(iarg);
        }
    }

    return (result_args, result_args_ext);
}

pub fn args_to_string() -> String {
    let args = args().collect::<Vec<String>>();

    args.join(" ")
}
