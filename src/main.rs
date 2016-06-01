extern crate itertools;
#[macro_use]
extern crate log;
#[macro_use]
extern crate nom;
extern crate simple_logger;

use std::io::{self, Read};
use std::collections::HashSet;

mod ceph_command;

fn print_run_command() {
    println!("def run_ceph_command(conffile, cmd, inbuf):");
    println!("    \"\"\"Run a ceph command and return the results");
    println!("");
    println!("    :param conffile: The ceph.conf configuration location");
    println!("    :param cmd: The json command to run");
    println!("    :param inbuf:");
    println!("    :return: (string outbuf, string outs)");
    println!("    :raise rados.Error: Raises on rados errors");
    println!("    \"\"\"");
    println!("    cluster = rados.Rados(conffile=conffile)");
    println!("    try:");
    println!("        cluster.connect()");
    println!("        result = cluster.mon_command(json.dumps(cmd), inbuf=inbuf)");
    println!("        if result[0] is not 0:");
    println!("            raise CephError(cmd=cmd, msg=os.strerror(abs(result[0])))");
    println!("        return result[1], result[2]");
    println!("    except rados.Error as e:");
    println!("        raise e");
}

fn print_exception_class() {
    println!("class CephError(Exception):");
    println!("    \"\"\"Exception raised for errors with running a Ceph command");
    println!("");
    println!("        :param cmd: cmd in which the error occurred");
    println!("        :param msg: explanation of the error");
    println!("    \"\"\"");
    println!("    def __init__(self, cmd, msg):");
    println!("        self.cmd = cmd");
    println!("        self.msg = msg");
    println!("");
}

fn print_init() {
    println!("    def __init__(self, rados_config_file):");
    println!("        self.rados_config_file = rados_config_file");
    println!("");
}

fn print_imports() {
    println!("from ceph_api.validator import validator");
    println!("import json");
    println!("import os");
    println!("import os.path");
    println!("import rados");
    println!("import re");
    println!("import six");
    println!("import stat");
    println!("import uuid as pyuuid");
    println!("");
}

/// This tries to find ceph_api commands that are going to have duplicate names
/// and rename them with a _2 or _3 suffix.  Python doesn't allow method overloading
/// so this is a hacky workaround.
fn rename_duplicate_functions(v: &mut Vec<ceph_command::Command>) {
    let mut h = HashSet::new();

    for i in v.iter_mut() {
        let prefix_method_name = i.signature.prefix.replace(" ", "_").replace("-", "_");
        let already_present = h.insert(prefix_method_name.clone());

        if !already_present {
            // rename
            i.signature.duplicate = true;
        }
    }
}

fn main() {
    // Read in the MonCommands.h file and produce ceph-commands.py file
    simple_logger::init_with_level(log::LogLevel::Warn).unwrap();
    let mut buffer: Vec<u8> = vec![];
    match io::stdin().read_to_end(&mut buffer) {
        Ok(_) => trace!("Read input from STDIN"),
        Err(e) => trace!("Failed to read STDIN: {:?}", e),
    };

    let input: &[u8] = &buffer.as_slice();
    let commands = ceph_command::parse_commands(input);

    match commands {
        nom::IResult::Done(_, cmds) => {

            // NOTE: Classes are grouped here.  Add more if needed
            // Group commands by module name

            // TODO: Optimize me for less brute force crap
            print_imports();
            print_exception_class();
            print_run_command();

            let mut pg_commands: Vec<ceph_command::Command> = cmds.iter()
                .cloned()
                .filter(|c| c.module_name == ceph_command::Module::Pg)
                .collect::<Vec<ceph_command::Command>>();
            rename_duplicate_functions(&mut pg_commands);
            if pg_commands.len() > 0 {
                println!("class {}:", ceph_command::Module::Pg.to_string());
                print_init();
                for result in pg_commands.iter() {
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let mut mds_commands: Vec<ceph_command::Command> = cmds.iter()
                .cloned()
                .filter(|c| c.module_name == ceph_command::Module::Mds)
                .collect::<Vec<ceph_command::Command>>();
            rename_duplicate_functions(&mut mds_commands);
            if mds_commands.len() > 0 {
                println!("class {}:", ceph_command::Module::Mds.to_string());
                print_init();
                for result in mds_commands.iter() {
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let mut osd_commands: Vec<ceph_command::Command> = cmds.iter()
                .cloned()
                .filter(|c| c.module_name == ceph_command::Module::Osd)
                .collect::<Vec<ceph_command::Command>>();
            rename_duplicate_functions(&mut osd_commands);
            if osd_commands.len() > 0 {
                println!("class {}:", ceph_command::Module::Osd.to_string());
                print_init();
                for result in osd_commands.iter() {
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let mut mon_commands: Vec<ceph_command::Command> = cmds.iter()
                .cloned()
                .filter(|c| c.module_name == ceph_command::Module::Mon)
                .collect::<Vec<ceph_command::Command>>();
            rename_duplicate_functions(&mut mon_commands);
            if mon_commands.len() > 0 {
                println!("class {}:", ceph_command::Module::Mon.to_string());
                print_init();
                for result in mon_commands.iter() {
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let mut auth_commands: Vec<ceph_command::Command> = cmds.iter()
                .cloned()
                .filter(|c| c.module_name == ceph_command::Module::Auth)
                .collect::<Vec<ceph_command::Command>>();
            rename_duplicate_functions(&mut auth_commands);
            if auth_commands.len() > 0 {
                println!("class {}:", ceph_command::Module::Auth.to_string());
                print_init();
                for result in auth_commands.iter() {
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let mut log_commands: Vec<ceph_command::Command> = cmds.iter()
                .cloned()
                .filter(|c| c.module_name == ceph_command::Module::Log)
                .collect::<Vec<ceph_command::Command>>();
            rename_duplicate_functions(&mut log_commands);
            if log_commands.len() > 0 {
                println!("class {}:", ceph_command::Module::Log.to_string());
                print_init();
                for result in log_commands.iter() {
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let mut configkey_commands: Vec<ceph_command::Command> = cmds.iter()
                .cloned()
                .filter(|c| c.module_name == ceph_command::Module::ConfigKey)
                .collect::<Vec<ceph_command::Command>>();
            rename_duplicate_functions(&mut configkey_commands);
            if configkey_commands.len() > 0 {
                println!("class {}:", ceph_command::Module::ConfigKey.to_string());
                print_init();
                for result in configkey_commands.iter() {
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

        }
        _ => {
            println!("Failed to parse commands");
        }
    }
}
