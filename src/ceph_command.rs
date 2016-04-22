extern crate nom;

use nom::{is_digit, is_alphabetic, is_alphanumeric, eof, multispace, not_line_ending, rest, space};

use std::collections::HashMap;
use std::io::{Read};
use std::str::{from_utf8, FromStr};

#[test]
fn one_command() {
    let input = r#"COMMAND("pg dump_pools_json", "show pg pools info in json only",\
	"pg", "r", "cli,rest")"#;
    let result = Command::parse(&input.as_bytes());
    println!("Result: {:?}", result);
}

#[test]
fn piped_command() {
    let input = r#"COMMAND("pg ls-by-osd " \
        "name=osd,type=CephOsdName " \
        "name=pool,type=CephInt,req=false " \
    	"name=states,type=CephChoices,strings=active|clean|down|replay|splitting|scrubbing|scrubq|degraded|inconsistent|peering|repair|recovering|backfill_wait|incomplete|stale|remapped|deep_scrub|backfill|backfill_toofull|recovery_wait|undersized|activating|peered,n=N,req=false ", \
    	"list pg on osd [osd]", "pg", "r", "cli,rest")"#;
    let result = Command::parse(&input.as_bytes());
    println!("Result: {:?}", result);
}

#[derive(Debug)]
pub enum Availability {
    Cli,
    Rest,
    Both,
    Unknown,
}

impl Availability {
    fn from_str(m: &str) -> Availability {
        trace!("Input to Availability: {:?}", m);
        match m {
            "cli" => Availability::Cli,
            "rest" => Availability::Rest,
            "cli,rest" => Availability::Both,
            _ => Availability::Unknown,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Module {
    Mds,
    Osd,
    Pg,
    Mon,
    Auth,
    Log,
    ConfigKey,
    Unknown,
}

impl Module {
    fn from_str(m: &str) -> Module {
        trace!("Input to Module: {:?}", m);
        match m {
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
    pub fn to_string(self) -> String{
        match self{
            Module::Mds => "MdsCommand".to_string(),
            Module::Osd => "OsdCommand".to_string(),
            Module::Pg => "PlacementGroupCommand".to_string(),
            Module::Mon => "MonitorCommand".to_string(),
            Module::Auth => "AuthCommand".to_string(),
            Module::Log => "LogCommand".to_string(),
            Module::ConfigKey => "ConfigKeyCommand".to_string(),
            Module::Unknown => "UnknownCommand".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Permissions {
    fn from_str(perms: &str) -> Permissions {
        Permissions {
            read: perms.contains("r"),
            write: perms.contains("w"),
            execute: perms.contains("x"),
        }
    }
}

#[derive(Debug)]
pub struct Signature {
    pub prefix: String,
    pub parameters: HashMap<String, CephType>,
}

impl Signature {
    // This takes a &str not a [u8] like the others
    fn parse(input: & str) -> Self {
        let mut prefix: Vec<String> = Vec::new();

        // Replace all the nasty things
        let no_slashes_input = input.replace("\\", "")
                                    .replace("\"", "")
                                    .replace("\n", "")
                                    .replace("    ", "");

        //println!("Signature input: {:?}", no_slashes_input);
        let parts: Vec<&str> = no_slashes_input.split_whitespace()
                                               .filter(|x| x.len() > 0)
                                               .collect();
        let mut parameters: HashMap<String, CephType> = HashMap::new();
        // If name= in the parts array than we have a CephType and not the prefix
        for part in parts.iter() {
            if part.contains("name=") {
                // We have a parameter
                // "name=pool,type=CephInt,req=false "

                //println!("parse_param_map: {:?}", part);
                let result = parse_param_map(part.as_bytes());
                match result{
                    nom::IResult::Done(_, ref param_tuple) => {
                        parameters.insert(param_tuple.0.clone(), param_tuple.1.clone());
                    }
                    _ =>{
                        println!("Failed to parse: {:?}", result);
                    }
                }
            } else {
                // This is part of the prefix
                prefix.push(part.to_string());
            }
        }

        Signature {
            prefix: prefix.join(" "),
            parameters: parameters,
        }
    }
}


#[derive(Clone, Debug)]
pub enum AllowedRepeats{
    One, //Argument is allowed only once
    Many, //Argument is allowed 1 or more times
}

impl AllowedRepeats{
    fn from_str(repeats: &str) -> AllowedRepeats {
        match repeats{
            "N" => AllowedRepeats::Many,
            _ => AllowedRepeats::One,
        }
    }
}

#[derive(Clone, Debug)]
pub enum CephType {
    CephInt {
        req: bool,
        min: Option<u32>,
        max: Option<u32>,
    }, // Optional: range=min[|max]
    CephFloat {
        req: bool,
        min: Option<f32>,
        max: Option<f32>,
    }, // Optional range
    CephString {
        req: bool,
        goodchars: Option<String>,
        allowed_repeats: AllowedRepeats,
    }, // optional badchars
    CephSocketpath {
        req: bool,
    }, // validation involves "is it S_ISSOCK"
    CephIPAddr {
        req: bool,
    }, // v4 or v6 addr with optional port, syntax validated
    CephEntityAddr {
        req: bool,
    }, // CephIPAddr + optional '/nonce'
    CephPoolname {
        req: bool,
    }, // Plainold string
    CephObjectname {
        req: bool,
    }, // Another plainold string
    CephPgid {
        req: bool,
    }, // n.xxx where n is an int > 0, xxx is a hex number > 0
    CephName {
        req: bool,
    }, // daemon name, '*' or '<type>.<id>' (id must be int for type osd)
    CephOsdName {
        req: bool,
    }, // osd name, '*' or '<id> or 'osd.<id>' (id must be int)
    CephChoices {
        req: bool,
        choices: Vec<String>, /* Note that
                               * 	- string literals are accumulated into 'prefix'
                               * 	- n=1 descriptors are given normal string or int object values
                               * 	- n=N descriptors are given array values
                               * */
        allowed_repeats: AllowedRepeats,
    }, // strings="foo|bar" means this param can be either
    CephFilepath {
        req: bool,
    }, // openable file
    CephFragment {
        req: bool,
    }, // cephfs 'fragID': val/bits, val in hex 0xnnn, bits in dec
    CephUUID {
        req: bool,
    }, // uuid in text matching Python uuid.UUID()
    CephPrefix {
        req: bool,
    }, // special type assigned to literals
    Unknown,
}

impl CephType {
    fn parse<'a>(input: &'a [u8], ceph_type: String) -> nom::IResult<&'a [u8], Self> {
        match &ceph_type[..] {
            "CephInt" => {
                chain!(
                    input,
                    range_min: dbg!(call!(u32_min_range)) ~
                    range_max: dbg!(call!(u32_max_range)) ~
                    req: dbg!(call!(req)),
                    ||{
                        CephType::CephInt{
                            req: req,
                            min: range_min,
                            max: range_max,
                        }
                    }
                )
            }
            "CephFloat" => {
                chain!(
                    input,
                    min_range: dbg!(call!(f32_min_range)) ~
                    max_range: dbg!(call!(f32_max_range)) ~
                    req: dbg!(call!(req)) ,
                    ||{
                        CephType::CephFloat{
                            req: req,
                            min: min_range,
                            max: max_range,
                        }
                    }
                )
            }
            "CephString" => {
                if input.len() == 0{
                    nom::IResult::Done(input, CephType::CephString{
                        req: true,
                        goodchars: None,
                        allowed_repeats: AllowedRepeats::One,
                    })
                }else{
                    chain!(
                        input,
                        repeats: call!(one_or_more) ~
                        goodchars: call!(good_chars) ~
                        req: call!(req),
                        ||{
                            CephType::CephString{
                                req: req,
                                goodchars: goodchars,
                                allowed_repeats: repeats,
                            }
                        }
                    )
                }
            }
            "CephSocketpath" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephSocketpath{
                            req: req,
                        }
                    }
                )
            }
            "CephIPAddr" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephIPAddr{
                            req: req,
                        }
                    }
                )
            }
            "CephEntityAddr" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephEntityAddr{
                            req: req,
                        }
                    }
                )
            }
            "CephPoolname" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephPoolname{
                            req: req,
                        }
                    }
                )
            }
            "CephObjectname" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephObjectname{
                            req: req,
                        }
                    }
                )
            }
            "CephPgid" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephPgid{
                            req: req,
                        }
                    }
                )
            }
            "CephName" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephName{
                            req: req,
                        }
                    }
                )
            }
            "CephOsdName" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephOsdName{
                            req: req,
                        }
                    }
                )
            }
            "CephChoices" => {
                chain!(
                    input,
                    choices: choices ~
                    repeats: call!(one_or_more) ~
                    req: call!(req) ,
                    ||{
                        CephType::CephChoices{
                            req: req,
                            choices: choices.clone(),
                            allowed_repeats: repeats,
                        }
                    }
                )
            }
            "CephFilepath" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephFilepath{
                            req: req,
                        }
                    }
                )
            }
            "CephFragment" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephFragment{
                            req: req,
                        }
                    }
                )
            }
            "CephUUID" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephUUID{
                            req: req,
                        }
                    }
                )
            }
            "CephPrefix" => {
                chain!(
                    input,
                    req: call!(req) ,
                    ||{
                        CephType::CephPrefix{
                            req: req,
                        }
                    }
                )
            }
            _ => {
                nom::IResult::Done(input, CephType::Unknown)
            }
        }
    }

    fn validate_string(&self, param_name: &String) -> String{
        match self{
            &CephType::CephInt{req, min, max}  => {
                let mut validate = String::new();
                validate.push_str("assert isinstance(");
                validate.push_str(param_name);
                validate.push_str(",");
                validate.push_str(" int");
                validate.push_str(")");

                validate
            },
            &CephType::CephFloat{req, min, max} => {
                let mut validate = String::new();
                validate.push_str("assert isinstance(");
                validate.push_str(param_name);
                validate.push_str(",");
                validate.push_str(" float");
                validate.push_str(")");

                validate
            },
            &CephType::CephString{req, ref goodchars, ref allowed_repeats} => {
                let mut validate = String::new();
                validate.push_str("assert isinstance(");
                validate.push_str(param_name);
                validate.push_str(",");
                validate.push_str(" six.string_types");
                validate.push_str(")");

                validate
            },
            &CephType::CephSocketpath{req} => {
                let mut validate = String::new();

                validate
            },
            &CephType::CephIPAddr{req} => {
                let mut validate = String::new();

                validate
            },
            &CephType::CephEntityAddr{req} => {
                let mut validate = String::new();

                validate
            },
            &CephType::CephPoolname{req} => {
                let mut validate = String::new();
                validate.push_str("assert isinstance(");
                validate.push_str(param_name);
                validate.push_str(",");
                validate.push_str(" six.string_types");
                validate.push_str(")");

                validate
            },
            &CephType::CephObjectname{req} => {
                let mut validate = String::new();
                validate.push_str("assert isinstance(");
                validate.push_str(param_name);
                validate.push_str(",");
                validate.push_str(" six.string_types");
                validate.push_str(")");

                validate
            },
            &CephType::CephPgid{req} => {
                let mut validate = String::new();

                validate
            },
            &CephType::CephName{req} => {
                let mut validate = String::new();

                validate
            },
            &CephType::CephOsdName{req} => {
                let mut validate = String::new();

                validate
            },
            &CephType::CephChoices{req, ref choices, ref allowed_repeats} => {
                let mut validate = String::new();
                validate.push_str("validator(value=");
                validate.push_str(param_name);
                validate.push_str(",");
                validate.push_str(" valid_type=list,");
                validate.push_str(" valid_range=[");
                //choices
                let quoted_choices:Vec<String> = choices.iter().map(|s| format!("\"{}\"", s)).collect();
                validate.push_str(&quoted_choices.join(","));
                validate.push_str("]");
                validate.push_str(")");

                validate
            },
            &CephType::CephFilepath{req} => {
                let mut validate = String::new();

                validate
            },
            &CephType::CephFragment{req} => {
                let mut validate = String::new();

                validate
            },
            &CephType::CephUUID{req} => {
                let mut validate = String::new();
                validate.push_str("assert isinstance(");
                validate.push_str(param_name);
                validate.push_str(",");
                validate.push_str(" uuid.UUID");
                validate.push_str(")");

                validate
            },
            &CephType::CephPrefix{req} => {
                let mut validate = String::new();

                validate
            },
            &CephType::Unknown => {
                "".to_string()
            }
        }
    }
}

#[test]
fn test_good_chars(){
    let x: &[u8] = &[];
    let input = "goodchars=[A-Za-z0-9-_.],";
    let result = good_chars(input.as_bytes());
    assert_eq!(nom::IResult::Done(x, Some("A-Za-z0-9-_.".to_string())), result);

    let input2 = "goodchars=[A-Za-z0-9-_.] ";
    let result2 = good_chars(input2.as_bytes());
    assert_eq!(nom::IResult::Done(x, Some("A-Za-z0-9-_.".to_string())), result2);

    let input3 = "goodchars=[A-Za-z0-9-_.=]";
    let result3 = good_chars(input3.as_bytes());
    assert_eq!(nom::IResult::Done(x, Some("A-Za-z0-9-_.=".to_string())), result3);
}

named!(parse_good_chars<&[u8], Option<String> >,
    chain!(
        tag!("goodchars=[") ~
        chars: map_res!(
                take_until!("]"), from_utf8) ~
        tag!("]") ~
        call!(trailing_chars),
        ||{
            Some(chars.to_string())
        }
    )
);

fn good_chars(input: &[u8]) -> nom::IResult<&[u8], Option<String>>{
    let chars = tag!(input, "goodchars=");
    match chars{
        nom::IResult::Done(_, _) => {
            return parse_good_chars(input);
        }
        nom::IResult::Incomplete(_) => {
            nom::IResult::Done(input, None)
        }
        nom::IResult::Error(_) => {
            nom::IResult::Done(input, None)
        }
    }
}

fn trailing_chars(input: &[u8]) ->nom::IResult<&[u8], ()>{
    //3 possible trailing chars either "," " " or "".  They all need to be handled
    let comma = tag!(input,",");
    match comma{
        nom::IResult::Done(remaining, _) => {
            //Found a comma, we're done
            return nom::IResult::Done(remaining, ());
        }
        nom::IResult::Incomplete(_) => {
            //Ran out of input.  We're done
            return nom::IResult::Done(input, ());
        }
        nom::IResult::Error(_) => {
            //Possibly a space?
            let space = tag!(input, " ");
            match space{
                nom::IResult::Done(remaining, _) => {
                    //Found a space, we're done
                    return nom::IResult::Done(remaining, ());
                }
                nom::IResult::Incomplete(_) => {
                    //Ran out of input.  We're done
                    return nom::IResult::Done(input, ());
                }
                nom::IResult::Error(_) => {
                    return nom::IResult::Done(input, ());
                }
            }
        }
    }
}

fn req(input: &[u8]) -> nom::IResult<&[u8], bool>{
    if input.len() == 0{
        return nom::IResult::Done(input, true);
    }else{
        return parse_req(input);
    }
}

named!(parse_req<&[u8], bool>,
    map_res!(
        map_res!(
            chain!(
                tag!("req=") ~
                req: take_while!(is_alphabetic) ,
                ||{
                    req
                }
            ), from_utf8),
        bool::from_str
    )
);

fn u32_min_range(input: &[u8]) -> nom::IResult<&[u8], Option<u32>>{
    let range = tag!(input, "range=");
    match range{
        nom::IResult::Done(remaining, _) => {
            chain!(
                remaining,
                min: map_res!(
                map_res!(
                    take_while!(is_digit),
                    from_utf8), u32::from_str) ~
                call!(trailing_chars),
                ||{
                    Some(min)
                }
            )
        }
        nom::IResult::Incomplete(_) => {
            nom::IResult::Done(input, None)
        }
        nom::IResult::Error(_) => {
            nom::IResult::Done(input, None)
        }
    }
}

fn u32_max_range(input: &[u8]) -> nom::IResult<&[u8], Option<u32>>{
    let start = tag!(input, "|");
    match start{
        nom::IResult::Done(_, _) => {
            return u32_max(input);
        }
        nom::IResult::Incomplete(_) => {
            nom::IResult::Done(input, None)
        }
        nom::IResult::Error(_) => {
            nom::IResult::Done(input, None)
        }
    }
}

named!(u32_max<&[u8], Option<u32> >,
    chain!(
        tag!("|") ~
        max: map_res!(
                map_res!(
                    take_while!(is_digit),
                    from_utf8), u32::from_str
            )? ~
        call!(trailing_chars),
        ||{
            max
        }
    )
);

#[test]
fn test_float(){
    let x: &[u8] = &[];
    let input = "type=CephFloat,name=weight,range=0.0|1.0";
    let result = parse_param_map(input.as_bytes());
    println!("Result: {:?}", result);
    //assert_eq!(nom::IResult::Done(x, Some("A-Za-z0-9-_.".to_string())), result);
}

fn is_float(chr: u8) -> bool {
    (chr >= 0x30 && chr <= 0x39) || chr == 0x2e
}

fn f32_min_range(input: &[u8]) -> nom::IResult<&[u8], Option<f32>>{
    //println!("input: {:?}", input);
    let range = tag!(input, "range=");
    match range{
        nom::IResult::Done(remaining, _) => {
            chain!(
                remaining,
                min: map_res!(
                map_res!(
                    take_while!(is_float),
                    from_utf8), f32::from_str) ~
                call!(trailing_chars),
                ||{
                    Some(min)
                }
            )
        }
        nom::IResult::Incomplete(_) => {
            nom::IResult::Done(input, None)
        }
        nom::IResult::Error(_) => {
            nom::IResult::Done(input, None)
        }
    }
}

named!(f32_max<&[u8], Option<f32> >,
    chain!(
        tag!("|") ~
        max: map_res!(
                map_res!(
                    take_while!(is_float),
                    from_utf8), f32::from_str
            )? ~
        call!(trailing_chars),
        ||{
            max
        }
    )
);

fn f32_max_range(input: &[u8]) -> nom::IResult<&[u8], Option<f32>> {
    //println!("input: {:?}", input);
    let start = tag!(input, "|");
    match start{
        nom::IResult::Done(_, _) => {
            return f32_max(input);
        }
        nom::IResult::Incomplete(_) => {
            nom::IResult::Done(input, None)
        }
        nom::IResult::Error(_) => {
            nom::IResult::Done(input, None)
        }
    }
}

#[test]
fn test_choices(){
    let x: &[u8] = &[];
    let input = "strings=replicated|erasure,";
    let result = choices(input.as_bytes());
    assert_eq!(nom::IResult::Done(x, vec!["replicated".to_string(), "erasure".to_string()]), result);

    let input2 = "strings=--yes-i-really-really-mean-it,";
    let result2 = choices(input2.as_bytes());
    assert_eq!(nom::IResult::Done(x, vec!["--yes-i-really-really-mean-it".to_string()]), result2);

    let input3 = "strings=unfound_objects_exist|degraded_pgs_exist";
    let result3 = choices(input3.as_bytes());
    assert_eq!(nom::IResult::Done(x, vec!["unfound_objects_exist".to_string(), "degraded_pgs_exist".to_string()]), result3);
}

named!(choices<&[u8], Vec<String> >,
    chain!(
        tag!("strings=") ~
        choices: map_res!(
                    alt!(
                        take_until_and_consume!(",") |
                        rest
                ), from_utf8),
        ||{
            choices.split("|").map(|s: &str| s.to_string()).collect()
        }
    )
);

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

named!(parse_name <&[u8], &str>,
    map_res!(
        chain!(
            tag!("name=") ~
            s: take_until_and_consume!(",") ,
            ||{
                s
            }
        ), from_utf8
    )
);

#[test]
fn check_type_parameter() {
    let x: &[u8] = &[];
    let input = "type=CephInt,";
    let result = parse_type(input.as_bytes());
    assert_eq!(nom::IResult::Done(x, "CephInt".to_string()), result);

    let input2 = "type=CephInt";
    let result2 = parse_type(input2.as_bytes());
    assert_eq!(nom::IResult::Done(x, "CephInt".to_string()), result2);
}

named!(parse_type <&[u8], String>,
    map!(
    map_res!(
        chain!(
            tag!("type=") ~
            s: dbg!(alt!(
                take_until_and_consume!(",") |
                take_until_and_consume!(" ") |
                take_while!(is_alphabetic))),
            ||{
                s
            }
        ), from_utf8
    ), str::to_string)
);

fn one_or_more(input: &[u8]) -> nom::IResult<&[u8], AllowedRepeats>{
    let start = tag!(input, "n=");
    match start{
        nom::IResult::Done(_, _) => {
            return allowed_repeats(input);
        }
        nom::IResult::Incomplete(_) => {
            nom::IResult::Done(input, AllowedRepeats::One)
        }
        nom::IResult::Error(_) => {
            nom::IResult::Done(input, AllowedRepeats::One)
        }
    }
}

named!(allowed_repeats <&[u8], AllowedRepeats>,
    map!(
        map_res!(
            chain!(
                tag!("n=") ~
                more: alt!(
                    take_until_and_consume!(",") |
                    take_while!(is_alphanumeric)),
                ||{
                    more
                }
            ), from_utf8
        ), AllowedRepeats::from_str
    )
);

fn parse_param_map(input: &[u8]) -> nom::IResult<&[u8], (String, CephType)> {
    //A few of the Command's have a reversed type="",name="" which is unfortunate
    let name_first = tag!(input, "name=");
    match name_first{
        nom::IResult::Done(_,_) => {
            //name="" is first.  Parse normally
            chain!(
                input,
                name: parse_name ~
                ceph_type: parse_type ~
                ceph_struct: dbg!(call!(CephType::parse, ceph_type)) ,
                ||{
                    (name.to_string(), ceph_struct)
                }
            )
        }
        nom::IResult::Error(_) => {
            //name="" is not first. Lets try type="" and see if that is first
            let type_first = tag!(input, "type=");
            match type_first{
                nom::IResult::Done(_,_) => {
                    chain!(
                        input,
                        ceph_type: parse_type ~
                        name: parse_name ~
                        ceph_struct: dbg!(call!(CephType::parse, ceph_type)) ,
                        ||{
                            (name.to_string(), ceph_struct)
                        }
                    )
                }
                nom::IResult::Error(e) => {
                    //I don't know how to parse this
                    return nom::IResult::Error(e);
                }
                nom::IResult::Incomplete(needed) =>{
                    return nom::IResult::Incomplete(needed);
                }
            }
        }
        nom::IResult::Incomplete(needed) =>{
            return nom::IResult::Incomplete(needed);
        }
    }

}

#[test]
fn check_parse_param_map() {
    let input = "name=epoch,type=CephInt,range=0,req=false";
    let result = parse_param_map(input.as_bytes());
    println!("Result: {:?}", result);
}

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

// Copied from: https://github.com/filipegoncalves/rust-config/blob/master/src/parser.rs
named!(blanks,
       chain!(
           many0!(alt!(multispace | comment_one_line | comment_block)),
           || { &b""[..] }));

// Auxiliary parser to ignore newlines
// NOTE: In some cases, this parser is combined with others that use `not_line_ending`
//       However, `not_line_ending` won't match `\u{2028}` or `\u{2029}`
// Copied from: https://github.com/filipegoncalves/rust-config/blob/master/src/parser.rs
named!(eol,
       alt!(tag!("\n") | tag!("\r\n") | tag!("\u{2028}") | tag!("\u{2029}")));

// Auxiliary parser to ignore one-line comments
// Copied from: https://github.com/filipegoncalves/rust-config/blob/master/src/parser.rs
named!(comment_one_line,
       chain!(
           alt!(tag!("//") | tag!("#")) ~
           not_line_ending? ~
           alt!(eof | eol),
           || { &b""[..] }));

// Auxiliary parser to ignore block comments
// Copied from: https://github.com/filipegoncalves/rust-config/blob/master/src/parser.rs
named!(comment_block,
       chain!(
           tag!("/*") ~
           take_until_and_consume!(&b"*/"[..]),
           || { &b""[..] }));

// COMMAND(signature, helpstring, modulename, req perms, availability)
#[derive(Debug)]
pub struct Command {
    pub signature: Signature,
    pub helpstring: String,
    pub module_name: Module,
    pub permissions: Permissions,
    pub availability: Availability,
}

impl Command {
    fn parse(input: & [u8]) -> nom::IResult<&[u8], Self> {
        //println!("Input to Command: {:?}", input);
        chain!(
            input,
                dbg!(many0!(blanks)) ~
                dbg!(tag!("COMMAND(")) ~
                signature: dbg_dmp!(quoted_string) ~
                helpstring: dbg_dmp!(quoted_string) ~
                module_name: dbg_dmp!(module) ~
                permissions: dbg_dmp!(permissions) ~
                availability: dbg_dmp!(availability) ~
                dbg!(tag!(")")) ~
                dbg!(blanks)? ,
            ||{
                Command{
                    signature: Signature::parse(signature),
                    helpstring: helpstring.to_string(),
                    module_name: module_name,
                    permissions: permissions,
                    availability: availability,
                }
            }
        )
    }

    pub fn to_string(&self) -> String {
        let mut output = String::new();
        output.push_str("    def ");
        output.push_str(&self.signature.prefix.replace(" ", "_").replace("-", "_"));
        output.push_str("(self");

        let num_of_params = self.signature.parameters.len();
        let mut counter = 0;

        for key in self.signature.parameters.keys(){
            if counter == 0{
                output.push_str(", ");
            }
            counter+=1;
            //NOTE: How can I handle optional parameters?
            //let ceph_type = self.signature.parameters.get(key).unwrap();
            //if ceph_type.req{
                output.push_str(&key);
            //}else{
                //output.push_str(&key);
                //output.push_str("=None");
            //}
            if counter < num_of_params{
                output.push_str(", ");
            }
        }
        output.push_str("):\n");

        //Help strings
        output.push_str("        \"\"\"\n");
        output.push_str("        ");
        output.push_str(&self.helpstring);
        output.push_str("\n");
        for key in self.signature.parameters.keys(){
            output.push_str("        :param ");
            output.push_str(key);
            output.push_str("\n");
        }
        output.push_str("\n        :return: (int ret, string outbuf, string outs)");
        output.push_str("\n        \"\"\"\n");
        //Help strings

        //Validate the parameters
        for (key, ceph_type) in self.signature.parameters.iter(){
            let validate_string = ceph_type.validate_string(&key);
            output.push_str("        ");
            output.push_str(&validate_string);
            output.push_str("\n");
        }

        //Create the cmd dictionary
        output.push_str("        cmd={");
        output.push_str("\n            'prefix':");
        output.push_str("'");
        output.push_str(&self.signature.prefix);
        output.push_str("',");
        for key in self.signature.parameters.keys(){
            output.push_str("\n");
            output.push_str("            '");
            output.push_str(key);
            output.push_str("':");
            output.push_str(key);
            output.push_str(",");
        }
        output.push_str("\n        }");

        //Connect to rados and run the command
        output.push_str("\n        cluster = rados.Rados(conffile='/etc/ceph/ceph.conf')");
        output.push_str("\n        try:");
        output.push_str("\n            cluster.connect()");
        output.push_str("\n            result = cluster.mon_command(json.dumps(cmd), inbuf='')");
        output.push_str("\n            cluster.shutdown()");
        output.push_str("\n            if result[0] is not 0:");
        output.push_str("\n                raise CephError(cmd=cmd, msg=os.strerror(abs(result[0])))");
        output.push_str("\n            return result");
        output.push_str("\n        except rados.Error as e:");
        output.push_str("\n            raise e");

        output
    }
}

pub fn parse_commands(input: &[u8]) -> nom::IResult<&[u8], Vec<Command>> {
    chain!(
        input,
        commands: many0!(
            dbg!(
                call!(Command::parse)
            )),
        ||{
            commands
        }
    )
}
