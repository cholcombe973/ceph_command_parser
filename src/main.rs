extern crate itertools;
#[macro_use] extern crate log;
#[macro_use] extern crate nom;
extern crate simple_logger;

use std::io::{self, Read};

mod ceph_command;

use ceph_command::Command;

fn main(){
    //Read in the MonCommands.h file and produce ceph-commands.py file
    simple_logger::init_with_level(log::LogLevel::Warn).unwrap();
    let mut buffer: Vec<u8> = vec![];
    match io::stdin().read_to_end(&mut buffer) {
        Ok(_) => trace!("Read input from STDIN"),
        Err(e) => trace!("Failed to read STDIN: {:?}", e)
    };

    let input: &[u8] = &buffer.as_slice();
    let commands = ceph_command::parse_commands(input);

    match commands{
        nom::IResult::Done(_, cmds) => {

            //NOTE: Classes are grouped here.  Add more if needed
            // Group commands by module name

            //TODO: Optimize me for less brute force crap
            let pg_commands:Vec<&ceph_command::Command> = cmds.iter().filter(|c| c.module_name == ceph_command::Module::Pg).collect();
            if pg_commands.len() > 0{
                println!("class {}:", ceph_command::Module::Pg.to_string());
                for result in pg_commands.iter(){
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let mds_commands: Vec<&ceph_command::Command> = cmds.iter().filter(|c| c.module_name == ceph_command::Module::Mds).collect();
            if mds_commands.len() > 0{
                println!("class {}:", ceph_command::Module::Mds.to_string());
                for result in mds_commands.iter(){
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let osd_commands: Vec<&ceph_command::Command> = cmds.iter().filter(|c| c.module_name == ceph_command::Module::Osd).collect();
            if osd_commands.len() > 0{
                println!("class {}:", ceph_command::Module::Osd.to_string());
                for result in osd_commands.iter(){
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let mon_commands: Vec<&ceph_command::Command> = cmds.iter().filter(|c| c.module_name == ceph_command::Module::Mon).collect();
            if mon_commands.len() > 0{
                println!("class {}:", ceph_command::Module::Mon.to_string());
                for result in mon_commands.iter(){
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let auth_commands: Vec<&ceph_command::Command> = cmds.iter().filter(|c| c.module_name == ceph_command::Module::Auth).collect();
            if auth_commands.len() > 0{
                println!("class {}:", ceph_command::Module::Auth.to_string());
                for result in auth_commands.iter(){
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let log_commands: Vec<&ceph_command::Command> = cmds.iter().filter(|c| c.module_name == ceph_command::Module::Log).collect();
            if log_commands.len() > 0{
                println!("class {}:", ceph_command::Module::Log.to_string());
                for result in log_commands.iter(){
                    let r = result.to_string();
                    println!("{}", r);
                }
            }

            let configkey_commands: Vec<&ceph_command::Command> = cmds.iter().filter(|c| c.module_name == ceph_command::Module::ConfigKey).collect();
            if configkey_commands.len() > 0{
                println!("class {}:", ceph_command::Module::ConfigKey.to_string());
                for result in configkey_commands.iter(){
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
