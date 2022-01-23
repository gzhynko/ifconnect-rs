use std::io::Read;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use futures::executor::block_on;
use futures::{future};
use tokio::io;
use tokio::net::tcp::{ReadHalf, WriteHalf};
use tokio::sync::Mutex;
use ifconnect::connection::Connection;
use ifconnect::event_args::{ReceivedDataArgs, ReceivedManifestArgs};
use ifconnect::typed_value::TypedValue;
use ifconnect::{TCP_PORT_V2, UDP_PORT};

#[tokio::main]
async fn main() {
    let mut conn = Connection::new();
    let instance = conn.listen_udp(&UDP_PORT, Some(Duration::from_secs(20))).unwrap(); // this hangs the process
    if instance.addresses.is_empty() { return; } // cancel if no addresses were supplied by the manifest

    conn.start_tcp(TCP_PORT_V2.clone(), instance.addresses[0].clone()).await.unwrap();

    // Wrap the connection with Arc<> so we can share it between threads
    // (in this case, the connection has to go inside the update loop, so we can't just move it there as we won't be able to use it elsewhere.
    let arc_conn = Arc::new(Mutex::new(conn));
    let loop_conn = Arc::clone(&arc_conn);
    tokio::spawn(async move {
        loop {
            let mut conn = loop_conn.lock().await;
            conn.update().await;
        }
    });

    let other_conn = Arc::clone(&arc_conn);

    let mut conn = other_conn.lock().await;
    conn.get_id(-1).await;

    conn.on_receive_data(on_receive_data);
    conn.on_receive_manifest(on_receive_manifest);

    std::mem::drop(conn);

    tokio::spawn(async move {
        loop {
        }
    }).await;
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
