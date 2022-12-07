use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use ifconnect::connection::Connection;
use ifconnect::event_args::{ReceivedDataArgs, ReceivedManifestArgs};
use ifconnect::{TCP_PORT_V2, UDP_PORT};

#[tokio::main]
async fn main() {
    let mut conn = Connection::new();
    let instance = conn.listen_udp(&UDP_PORT, Some(Duration::from_secs(30))).unwrap(); // this will block the current thread

    let ipv4_addresses = ifconnect::helpers::get_ipv4_addresses(instance.addresses);
    if ipv4_addresses.is_empty() {
        println!("no IPv4 addresses were supplied, quitting");
        return;
    } // quit if no ipv4 addresses were supplied by the manifest

    // start the tcp connection
    println!("using {} to connect to IF", ipv4_addresses[0]);
    conn.start_tcp(TCP_PORT_V2.clone(), ipv4_addresses[0].clone()).await.unwrap();
    println!("connected successfully.");

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
    let conn = other_conn.lock().await;
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
