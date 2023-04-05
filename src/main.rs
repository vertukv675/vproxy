use async_socks5::Result;
use colored::Colorize;
use rand::seq::SliceRandom;
use socks5_proto::{Address, Reply};
use socks5_server::{auth::NoAuth, Connection, IncomingConnection, Server};
use tokio::time::{sleep, Duration};

use std::error::Error;
use std::io::Write;
use std::process;
use std::sync::Arc;
use tokio::{io, net::TcpStream};

mod ports;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Proxy {
    ip: String,
    port: u16,
    protocol: String,
    priority: u16,
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    vproxy_port: u16,
    proxies: Vec<Proxy>,
}

impl Default for Proxy {
    fn default() -> Self {
        Proxy {
            ip: "127.0.0.1".to_string(),
            port: 8080,
            protocol: "socks5".to_string(),
            priority: 65535,
            username: "noauth".to_string(),
            password: "noauth".to_string(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            vproxy_port: 8899,
            proxies: vec![
                Proxy {
                    ip: "127.0.0.1".to_string(),
                    port: 9050,
                    protocol: "socks5".to_string(),
                    priority: 1,
                    username: "noauth".to_string(),
                    password: "noauth".to_string(),
                },
                Proxy {
                    ip: "127.0.0.1".to_string(),
                    port: 8085,
                    protocol: "socks5".to_string(),
                    priority: 2,
                    username: "noauth".to_string(),
                    password: "noauth".to_string(),
                },
            ],
        }
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    let mut vproxy_addr: String = "127.0.0.1:".to_string();
    const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
    let rand_emojis = vec![
        "😇", "🎔", "💗", "💕", "💡", "🌈", "😷", "💻", "🖳", "😈", "🦀", "😻", "🙊", "😮", "🐍",
    ];
    let mut prioritised_proxy: Proxy = Proxy::default();
    //let emoji: String = rand_emojis
    //    .choose(&mut rand::thread_rng())
    //    .unwrap()
    //    .to_string();
    println!("{} v{} starting.", "vProxy".yellow(), VERSION.bold());
    let second_line = format!("made by {} with love ", AUTHOR.blue().bold());
    print!("{second_line}");
    for i in 0..second_line.len() {
        let _idk = i;

        sleep(Duration::from_millis(150)).await;
        //let our_line = &second_line[..i];
        //print!("\r{}", our_line);
        print!(
            "\r{}{}",
            second_line,
            rand_emojis
                .choose(&mut rand::thread_rng())
                .unwrap()
                .to_string()
        );
        std::io::stdout().flush().expect("flush failed.");
    }
    println!("");
    sleep(Duration::from_millis(2000)).await;
    let cfg: AppConfig = confy::load("vproxy", None)?;
    //println!("{:#?}", cfg);
    println!("{}", "Checking if Vproxy port is available...".yellow());
    let vport_status = ports::port_is_used(cfg.vproxy_port);
    if vport_status {
        println!("{}", "Vproxy port is already used!".red().bold());
        process::exit(1);
    } else {
        println!("{}", "Vproxy port is available!".green().bold());
    }
    vproxy_addr.push_str(cfg.vproxy_port.to_string().as_str());
    for pr in cfg.proxies {
        println!(
            "Checking proxy: {}:{}...",
            pr.ip.bold().purple(),
            pr.port.to_string().bold().purple()
        );
        let status = ports::port_is_used(pr.port);
        if status == true {
            println!(
                "{} {}:{}",
                "This proxy is working! Ip:Port ",
                pr.ip.bold().purple(),
                pr.port.to_string().bold().purple()
            );

            if pr.priority < prioritised_proxy.priority {
                prioritised_proxy = pr;
                println!(
                    "{} {}:{}",
                    "Selected new proxy! Ip:Port ".yellow(),
                    prioritised_proxy.ip.bold().purple(),
                    prioritised_proxy.port.to_string().bold().purple()
                );
            }
        } else {
            println!("{}", "This proxy is offline!".to_string().bold().red());
        }
    }

    println!("vproxy addr: {}", vproxy_addr.to_string().bold().purple());

    let server = Server::bind(vproxy_addr, Arc::new(NoAuth)).await?;
    //let cfg_folder = confy::get_configuration_file_path("vproxy", None)?;
    let stats_db_path = confy::get_configuration_file_path("vproxy", None)
        .unwrap()
        .parent()
        .unwrap()
        .join("vproxy_stats.db")
        .into_os_string()
        .into_string()
        .unwrap();
    //let stats_db_path_string = stats_db_path.display().to_string();
    let db_conn = rusqlite::Connection::open(stats_db_path)?;

    db_conn.execute(
        "CREATE table if not exists stats(addr, counter INTEGER DEFAULT 0 NOT NULL);",
        (),
    )?;

    db_conn.execute(
        "CREATE UNIQUE INDEX if not exists idx_stats_addr on stats(addr);",
        (),
    )?;

    while let Ok((conn, _)) = server.accept().await {
        tokio::spawn(async move {
            match handle(conn).await {
                Ok(()) => {}
                Err(err) => eprintln!("{err}"),
            }
        });
    }

    Ok(())
}

async fn handle(conn: IncomingConnection) -> Result<(), Box<dyn Error>> {
    match conn.handshake().await? {
        Connection::Associate(associate, _) => {
            let mut conn = associate
                .reply(Reply::CommandNotSupported, Address::unspecified())
                .await?;
            conn.shutdown().await?;
        }
        Connection::Bind(bind, _) => {
            let mut conn = bind
                .reply(Reply::CommandNotSupported, Address::unspecified())
                .await?;
            conn.shutdown().await?;
        }
        Connection::Connect(connect, addr) => {
            println!("addr: {}", addr);
            //let proxy: SocketAddr = "127.0.0.1:9050".parse().unwrap();
            let target = match addr {
                Address::DomainAddress(domain, port) => {
                    //let targ: SocketAddrV4 = (domain + &port.to_string()).parse().expect("bruh");
                    match write_stats(&domain) {
                        Ok(()) => {}
                        Err(err) => eprintln!("{err}"),
                    }
                    TcpStream::connect((domain, port)).await
                    //SocksStream::connect(proxy, targ, Some(("".to_string(), "".to_string()))).await
                }
                Address::SocketAddress(addr) => {
                    //SocksStream::connect(proxy, addr, Some(("".to_string(), "".to_string()))).await
                    TcpStream::connect(addr).await
                }
            };
            println!("Target: {:?}", target);

            if let Ok(mut target) = target {
                let mut conn = connect
                    .reply(Reply::Succeeded, Address::unspecified())
                    .await?;
                //let mut targ = TcpStream::connect(("127.0.0.1", 8899)).await?;
                io::copy_bidirectional(&mut target, &mut conn).await?;
                //let (mut client_recv, mut client_send) = conn.split();

                //let (mut server_recv, mut server_send) = targ.split();

                //let handle_one =
                //  async { tokio::io::copy(&mut server_recv, &mut client_send).await };

                //let handle_two =
                //   async { tokio::io::copy(&mut client_recv, &mut server_send).await };

                //try_join!(handle_one, handle_two)?;
            } else {
                let mut conn = connect
                    .reply(Reply::HostUnreachable, Address::unspecified())
                    .await?;
                conn.shutdown().await?;
            }
        }
    }

    Ok(())
}

fn write_stats(addr: &String) -> Result<(), Box<dyn Error>> {
    //let cfg: AppConfig = confy::load("vproxy", None)?;

    let stats_db_path = confy::get_configuration_file_path("vproxy", None)
        .unwrap()
        .parent()
        .unwrap()
        .join("vproxy_stats.db")
        .into_os_string()
        .into_string()
        .unwrap();
    //let stats_db_path_string = stats_db_path.display().to_string();
    let conn = rusqlite::Connection::open(stats_db_path)?;

    let wr_command: String = format!("replace into stats (addr, counter) values ('{}', (select counter from stats where addr like '{}') + 1)", addr, addr);

    conn.execute(&wr_command, ())?;

    Ok(())
}

//async fn handle_conn_domain(domain: String, port: u16, proxy: Proxy) -> Result<()> {
//    let socksStream = Socks5Stream::connect("127.0.0.1:9050", "1:443");
//    let tcpStream = Socks5Stream::into_inner(socksStream);
//    Ok(())
//}
