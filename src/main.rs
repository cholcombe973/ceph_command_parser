#[macro_use] extern crate log;
#[macro_use] extern crate nom;
extern crate simple_logger;

use nom::{space};

use std::io::{self, ErrorKind, Read};
use std::str::from_utf8;

#[test]
fn one_command(){
    let input = r#"COMMAND("pg dump_pools_json", "show pg pools info in json only",\
	"pg", "r", "cli,rest")"#;
    let result = Command::parse(&input.as_bytes());
    println!("Result: {:?}", result);
}

#[test]
fn piped_command(){
    let input = r#"COMMAND("pg ls-by-osd " \
        "name=osd,type=CephOsdName " \
        "name=pool,type=CephInt,req=false " \
	"name=states,type=CephChoices,strings=active|clean|down|replay|splitting|scrubbing|scrubq|degraded|inconsistent|peering|repair|recovering|backfill_wait|incomplete|stale|remapped|deep_scrub|backfill|backfill_toofull|recovery_wait|undersized|activating|peered,n=N,req=false ", \
	"list pg on osd [osd]", "pg", "r", "cli,rest")"#;
    let result = Command::parse(&input.as_bytes());
    println!("Result: {:?}", result);
}

#[derive(Debug)]
enum Availability{
    Cli,
    Rest,
    Both,
    Unknown
}

impl Availability{
    fn from_str(m: &str) -> Availability{
        println!("Input to Availability: {:?}", m);
        match m{
            "cli" => Availability::Cli,
            "rest" => Availability::Rest,
            "cli,rest" => Availability::Both,
            _ => Availability::Unknown,
        }
    }
}

#[derive(Debug)]
enum Module{
    Mds,
    Osd,
    Pg,
    Mon,
    Auth,
    Log,
    ConfigKey,
    Unknown
}

impl Module{
    fn from_str(m: &str) -> Module{
        println!("Input to Module: {:?}", m);
        match m{
            "mds" => Module::Mds,
            "osd" => Module::Osd,
            "pg" => Module::Pg,
            "mon" => Module::Mon,
            "auth" => Module::Auth,
            "log" => Module::Log,
            "config-key" => Module::ConfigKey,
            _ => Module::Unknown,
        }
    }
}

#[derive(Debug)]
struct Permissions{
    read: bool,
    write: bool,
    execute: bool,
}

impl Permissions {
    fn from_str(perms: &str) -> Permissions{
        Permissions{
            read: perms.contains("r"),
            write: perms.contains("w"),
            execute: perms.contains("x"),
        }
    }
}
#[derive(Debug)]
enum CephType{
    CephInt, //Optional: range=min[|max]
    CephFloat, //Optional range
    CephString, //optional badchars
    CephSocketpath, //validation involves "is it S_ISSOCK"
    CephIPAddr, //v4 or v6 addr with optional port, syntax validated
    CephEntityAddr, //CephIPAddr + optional '/nonce'
    CephPoolname, //Plainold string
    CephObjectname, //Another plainold string
    CephPgid, //n.xxx where n is an int > 0, xxx is a hex number > 0
    CephName, //daemon name, '*' or '<type>.<id>' (id must be int for type osd)
    CephOsdName, //osd name, '*' or '<id> or 'osd.<id>' (id must be int)
    CephChoices, //strings="foo|bar" means this param can be either
    CephFilepath, //openable file
    CephFragment, //cephfs 'fragID': val/bits, val in hex 0xnnn, bits in dec
    CephUUID, //uuid in text matching Python uuid.UUID()
    CephPrefix, //special type assigned to literals
}

named!(quoted_string <&[u8], &str>,
    map_res!(
        chain!(
            space? ~
            take_until!("\"") ~
            tag!("\"") ~
            s: take_until!("\",") ~
            tag!("\"")~
            tag!(",")?,
            ||{
                s
            }
        ), from_utf8
    )
);

named!(quoted_avail_string <&[u8], &str>,
    map_res!(
        chain!(
            space? ~
            take_until!("\"") ~
            tag!("\"") ~
            s: take_until!("\")") ~
            tag!("\""),
            ||{
                s
            }
        ), from_utf8
    )
);
named!(module <&[u8], Module>,
    map!(
        chain!(
            module_name: quoted_string,
            ||{
                module_name
            }
        ), Module::from_str
    )
);

named!(availability <&[u8], Availability>,
    map!(
        chain!(
            availabity_string: quoted_avail_string,
            ||{
                availabity_string
            }
        ), Availability::from_str
    )
);

named!(permissions <&[u8], Permissions>,
    map!(
        chain!(
            perms: quoted_string,
            ||{
                perms
            }
        ), Permissions::from_str
    )
);

//COMMAND(signature, helpstring, modulename, req perms, availability)
#[derive(Debug)]
struct Command<'a>{
    signature: &'a str,
    helpstring: &'a str,
    module_name: Module,
    permissions: Permissions,
    availability: Availability,
}

impl<'a> Command<'a> {
    fn parse(input: &'a [u8]) -> nom::IResult<&[u8], Self>{
        //println!("Command input: {:?}", input);
        chain!(
            input,
                tag!("COMMAND(") ~
                signature: dbg_dmp!(quoted_string) ~
                helpstring: dbg_dmp!(quoted_string) ~
                module_name: dbg_dmp!(module) ~
                permissions: dbg_dmp!(permissions) ~
                availability: dbg_dmp!(availability) ~
                tag!(")"),
            ||{
                Command{
                    signature: signature,
                    helpstring: helpstring,
                    module_name: module_name,
                    permissions: permissions,
                    availability: availability,
                }
            }
        )
    }
}

fn main(){
    simple_logger::init_with_level(log::LogLevel::Warn).unwrap();
    let mut buffer: Vec<u8> = vec![];
    match io::stdin().read_to_end(&mut buffer) {
        Ok(_) => trace!("Read input from STDIN"),
        Err(e) => trace!("Failed to read STDIN: {:?}", e)
    };

    let input: &[u8] = &buffer.as_slice();
    /*
    let result: CrushMap = match parse_crushmap(&input){
        nom::IResult::Done(_, r) => r,
        _ => panic!("There was a problem parsing the crushmap"),
    };
    if result.magic != CRUSH_MAGIC {
        panic!("Could not decompile crushmap");
    }
    */
    //println!("{:?}", result);

}
