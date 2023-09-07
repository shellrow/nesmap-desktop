#[macro_use]
extern crate clap;

mod handler;
mod json_models;
mod output;
mod parser;
mod validator;

use clap::{App, AppSettings, Arg, ArgGroup, Command};
use std::env;
use rushmap_core::option;
use rushmap_core::process;
use rushmap_core::sys;
use rushmap_core::define;

// APP information
pub const CRATE_BIN_NAME: &str = "rmap";
pub const CRATE_UPDATE_DATE: &str = "2023-09-06";
pub const CRATE_REPOSITORY: &str = "https://github.com/shellrow/rushmap";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        show_app_desc();
        std::process::exit(0);
    }
    let app = get_app_settings();
    let matches = app.get_matches();
    show_banner_with_starttime();

    let pb = output::get_spinner();
    pb.set_message("Initializing ...");

    // Default is port scan
    let command_type: option::CommandType = 
    if matches.is_present("port") {
        option::CommandType::PortScan
    } else if matches.contains_id("host") {
        option::CommandType::HostScan
    } else if matches.contains_id("ping") {
        option::CommandType::Ping
    } else if matches.contains_id("trace") {
        option::CommandType::Traceroute
    } else if matches.contains_id("domain") {
        option::CommandType::DomainScan
    } else {
        option::CommandType::PortScan
    };

    pb.finish_and_clear();

    //output::show_options(opt.clone());
    match command_type {
        option::CommandType::PortScan => {
            let opt = parser::parse_port_args(matches).unwrap();
            output::show_port_options(opt.clone());
            match opt.scan_type {
                option::PortScanType::TcpSynScan => {
                    if opt.async_scan && sys::get_os_type() == "windows" {
                        exit_with_error_message("Async TCP SYN Scan is not supported on Windows");
                    }
                    if process::privileged() {
                        async_io::block_on(async {
                            handler::handle_port_scan(opt).await;
                        })
                    } else {
                        exit_with_error_message("Requires administrator privilege");
                    }
                },
                option::PortScanType::TcpConnectScan => {
                    async_io::block_on(async {
                        handler::handle_port_scan(opt).await;
                    })
                },
            }
        },
        option::CommandType::HostScan => {
            let opt = parser::parse_host_args(matches).unwrap();
            output::show_host_options(opt.clone());
            match opt.scan_type {
                option::HostScanType::IcmpPingScan => {
                    if process::privileged() {
                        async_io::block_on(async {
                            handler::handle_host_scan(opt).await;
                        })
                    } else {
                        exit_with_error_message("Requires administrator privilege");
                    }
                },
                _ => async_io::block_on(async {
                    handler::handle_host_scan(opt).await;
                }),
            }
        },
        option::CommandType::Ping => {
            let opt: option::PingOption = parser::parse_ping_args(matches).unwrap();
            output::show_ping_options(opt.clone());
            if process::privileged() || sys::get_os_type() == "windows" {
                handler::handle_ping(opt);
            } else {
                exit_with_error_message("Requires administrator privilege");
            }
        }
        option::CommandType::Traceroute => {
            let opt: option::TracerouteOption = parser::parse_trace_args(matches).unwrap();
            output::show_trace_options(opt.clone());
            if process::privileged() || sys::get_os_type() == "windows" {
                handler::handle_trace(opt);
            } else {
                exit_with_error_message("Requires administrator privilege");
            }
        }
        option::CommandType::DomainScan => {
            let opt: option::DomainScanOption = parser::parse_domain_args(matches).unwrap();
            output::show_domain_options(opt.clone());
            handler::handle_domain_scan(opt);
        }
    }
}

fn get_app_settings<'a>() -> Command<'a> {
    let app: App = Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(Arg::new("port")
            .help("Scan ports of the specified host. \nUse default port list if port range omitted. \nExamples: \n--port 192.168.1.8 -S -O \n--port 192.168.1.8:1-1000 \n--port 192.168.1.8:22,80,8080 \n--port 192.168.1.8 -l custom-list.txt")
            .short('p')
            .long("port")
            .takes_value(true)
            .value_name("target")
            .validator(validator::validate_port_opt)
        )
        .arg(Arg::new("host")
            .help("Scan hosts in specified network or host-list. \nExamples: \n--host 192.168.1.0 \n--host 192.168.1.0/24 \n--host custom-list.txt \n--host 192.168.1.10,192.168.1.20,192.168.1.30")
            .short('n')
            .long("host")
            .takes_value(true)
            .value_name("target")
            .validator(validator::validate_hostscan_opt)
        )
        .arg(Arg::new("ping")
            .help("Ping to specified host. \nExamples: \n--ping 192.168.1.8 -c 4")
            .short('g')
            .long("ping")
            .takes_value(true)
            .value_name("target")
            .validator(validator::validate_host_opt)
        )
        .arg(Arg::new("trace")
            .help("Traceroute to specified host. \nExamples: \n--trace 192.168.1.8")
            .short('e')
            .long("trace")
            .takes_value(true)
            .value_name("target")
            .validator(validator::validate_trace_opt)
        )
        .arg(Arg::new("domain")
            .help("Scan sub-domain . \nExamples: \n--domain example.com")
            .short('d')
            .long("domain")
            .takes_value(true)
            .value_name("target")
            .validator(validator::validate_domain_opt)
        )
        .arg(Arg::new("interface")
            .help("Specify the network interface")
            .short('i')
            .long("interface")
            .takes_value(true)
            .value_name("name")
            .validator(validator::validate_interface)
        )
        .arg(Arg::new("source")
            .help("Specify the source IP address")
            .short('s')
            .long("source")
            .takes_value(true)
            .value_name("ip_addr")
            .validator(validator::validate_interface)
        )
        .arg(Arg::new("protocol")
            .help("Specify the protocol")
            .short('P')
            .long("protocol")
            .takes_value(true)
            .value_name("protocol")
            .validator(validator::validate_protocol)
        )
        .arg(Arg::new("maxhop")
            .help("Set max hop(TTL) for ping or traceroute")
            .short('m')
            .long("maxhop")
            .takes_value(true)
            .value_name("maxhop")
            .validator(validator::validate_ttl)
        )
        .arg(Arg::new("scantype")
            .help("Specify the scan-type")
            .short('T')
            .long("scantype")
            .takes_value(true)
            .value_name("scantype")
            .validator(validator::validate_portscantype)
        )
        .arg(Arg::new("timeout")
            .help("Set timeout in ms - Example: -t 10000")
            .short('t')
            .long("timeout")
            .takes_value(true)
            .value_name("duration")
            .validator(validator::validate_timeout)
        )
        .arg(Arg::new("waittime")
            .help("Set wait-time in ms (default:100ms) - Example: -w 200")
            .short('w')
            .long("waittime")
            .takes_value(true)
            .value_name("duration")
            .validator(validator::validate_waittime)
        )
        .arg(Arg::new("rate")
            .help("Set send-rate in ms - Example: -r 1")
            .short('r')
            .long("rate")
            .takes_value(true)
            .value_name("duration")
            .validator(validator::validate_waittime)
        )
        .arg(Arg::new("count")
            .help("Set number of requests or pings to be sent")
            .short('c')
            .long("count")
            .takes_value(true)
            .value_name("count")
            .validator(validator::validate_count)
        )
        .arg(Arg::new("service")
            .help("Enable service detection")
            .short('S')
            .long("service")
            .takes_value(false)
        )
        .arg(Arg::new("os")
            .help("Enable OS detection")
            .short('O')
            .long("os")
            .takes_value(false)
        )
        .arg(Arg::new("async")
            .help("Perform asynchronous scan")
            .short('A')
            .long("async")
            .takes_value(false)
        )
        .arg(Arg::new("list")
            .help("Use list - Example: -l custom-list.txt")
            .short('l')
            .long("list")
            .takes_value(true)
            .value_name("file_path")
            .validator(validator::validate_filepath)
        )
        .arg(Arg::new("wellknown")
            .help("Use well-known ports")
            .short('W')
            .long("wellknown")
        )
        .arg(Arg::new("json")
            .help("Displays results in JSON format.")
            .short('j')
            .long("json")
            .takes_value(false)
        )
        .arg(Arg::new("save")
            .help("Save scan result in json format - Example: -o result.json")
            .short('o')
            .long("save")
            .takes_value(true)
            .value_name("file_path")
        )
        .arg(Arg::new("acceptinvalidcerts")
            .help("Accept invalid certs (This introduces significant vulnerabilities)")
            .long("acceptinvalidcerts")
            .takes_value(false)
        )
        .group(ArgGroup::new("mode").args(&["port", "host", "ping", "trace", "domain"]))
        .setting(AppSettings::DeriveDisplayOrder)
        ;
    app
}

fn show_app_desc() {
    println!(
        "{}({}) {} ({}) {}",
        crate_name!(),
        CRATE_BIN_NAME,
        crate_version!(),
        CRATE_UPDATE_DATE,
        sys::get_os_type()
    );
    println!("{}", crate_description!());
    println!("{}", crate_authors!());
    println!("{}", CRATE_REPOSITORY);
    println!();
    println!("'{} --help' for more information.", CRATE_BIN_NAME);
    println!();
}

fn show_banner_with_starttime() {
    println!(
        "{}({}) {} {}",
        crate_name!(),
        CRATE_BIN_NAME,
        crate_version!(),
        sys::get_os_type()
    );
    println!("{}", CRATE_REPOSITORY);
    println!();
    println!("Probe started at {}", sys::get_sysdate());
    println!();
}

fn exit_with_error_message(message: &str) {
    println!();
    println!("Error: {}", message);
    std::process::exit(0);
}