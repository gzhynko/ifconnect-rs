use std::sync::Arc;
use std::time::Duration;
use dialoguer::{Input, Select};
use dialoguer::theme::ColorfulTheme;
use tokio::sync::Mutex;
use ifconnect::connection::Connection;
use ifconnect::event_args::{ReceivedDataArgs, ReceivedManifestArgs};
use ifconnect::{TCP_PORT_V2, UDP_PORT};

const UDP_TIMEOUT_SECS: u64 = 30;

#[tokio::main]
async fn main() {
    let mut conn = Connection::new();

    let ip = get_device_ip(&mut conn);
    println!("Connecting to {}:{} via TCP...", ip, TCP_PORT_V2.clone());
    conn.start_tcp(TCP_PORT_V2.clone(), ip.clone()).await.unwrap();
    println!("Connected to {}:{}.", ip, TCP_PORT_V2.clone());

    // setup event callbacks
    conn.on_receive_data(Some(on_receive_data));
    conn.on_receive_manifest(Some(on_receive_manifest));

    // Wrap the connection with Arc<> so we can share it between threads
    // (in this case, the connection has to go inside the update loop, so we can't just move it there since we won't be able to use it elsewhere.
    let arc_conn = Arc::new(Mutex::new(conn));

    // Start the update loop to receive/send data from/to the API.
    let loop_conn = Arc::clone(&arc_conn);
    tokio::spawn(async move {
        loop {
            let mut conn = loop_conn.lock().await;
            conn.update().await.unwrap();
        }
    });

    let other_conn = Arc::clone(&arc_conn);
    display_menu(&other_conn).await;
}

fn get_device_ip(conn: &mut Connection) -> String {
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you have the IP address of the device running IF on the local network?")
        .default(0)
        .item("No, perform a UDP search.")
        .item("Yes, I know the device IP.")
        .interact()
        .unwrap();

    let result;

    if selection == 0 {
        println!("Running a UDP search with a {}s timeout...", UDP_TIMEOUT_SECS);
        let instance_information = conn.listen_udp(&UDP_PORT, Some(Duration::from_secs(UDP_TIMEOUT_SECS))).unwrap(); // this will block the current thread

        let ipv4_addresses = ifconnect::helpers::get_ipv4_addresses(instance_information.addresses);
        if ipv4_addresses.is_empty() {
            return String::new();
        }

        if ipv4_addresses.len() == 1 {
            result = ipv4_addresses[0].clone();
        } else {
            let ip_selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Got several IPv4 addresses, please choose one:")
                .default(0)
                .items(&ipv4_addresses)
                .interact()
                .unwrap();

            result = ipv4_addresses[ip_selection].clone();
        }
    } else {
        let ip: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter the local IP address of the device running IF:")
            .interact_text()
            .unwrap();

        result = ip;
    }

    result
}

async fn display_menu(arc_conn: &Arc<Mutex<Connection>>) {
    let conn = arc_conn.lock().await;
    conn.get_manifest().await;

    // prevent this conn from blocking the update thread by dropping the mutex lock
    drop(conn);

    // loop infinitely to keep the cli app running
    tokio::spawn(async move {
        loop {
        }
    }).await.unwrap();
}

fn on_receive_data(args: ReceivedDataArgs) {
    println!("on receive data: {} {}", args.command_id, args.data);
}

fn on_receive_manifest(args: ReceivedManifestArgs) {
    println!("on receive manifest: {} entries", args.manifest.entries_num());

    let entries = args.manifest.get_entries_with_prefix("infiniteflight");
    println!("entries with 'infiniteflight': ");
    for entry in entries {
        println!("{}: {}", entry.id, entry.string);
    }
}
