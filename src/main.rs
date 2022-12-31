use std::time::Duration;

use async_process::{Command, Stdio};
use bytesize::ByteSize;
use futures_lite::io::BufReader;
use futures_lite::prelude::*;
use sysinfo::{Pid, ProcessExt, System, SystemExt};
use tokio::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut sub_args = std::env::args().skip(1);
    let mut child = Command::new(sub_args.next().expect("must provide a command to run"))
        .args(sub_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sh command failed to start");

    // Get output lines to show in TUI in future.
    let mut std_out = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut std_err = BufReader::new(child.stderr.take().unwrap()).lines();

    // let mut sys = System::new_with_specifics(
    //     RefreshKind::new().with_processes(ProcessRefreshKind::new().with_cpu()),
    // );
    // TODO swap to only track relevant/used info (above).
    let mut sys = System::new_all();

    let mut interval_timer = tokio::time::interval_at(Instant::now(), Duration::from_millis(500));
    loop {
        tokio::select! {
            _ = interval_timer.tick() => {
                sys.refresh_all();

                // Print data at intervals
                let ps_info = sys
                    .process(Pid::from(child.id() as usize))
                    .expect("process not spawned correctly");
                // TODO fix what is output
                println!("mem: {}", ByteSize(ps_info.memory()));
                println!("CPU: {}", ps_info.cpu_usage());
                let disk_usage = ps_info.disk_usage();
                println!("Bytes read: {}", ByteSize(disk_usage.total_read_bytes));
                println!("Bytes written: {}", ByteSize(disk_usage.total_written_bytes));
				println!();

                // for (interface_name, data) in sys.networks() {
                //     println!(
                //         "{}: {}/{} B",
                //         interface_name,
                //         data.received(),
                //         data.transmitted()
                //     );
                // }
            }
            status = child.status() => {
                println!("Process exited with status code: {}", status?);
                break;
            },
            // TODO swap these to show in better way than just forwarding
            Some(line) = std_out.next() => println!("{}", line?),
            Some(line) = std_err.next() => eprintln!("{}", line?),
        }
    }

    // Flush remaining output and give summary
    // TODO incomplete
    while let Some(line) = std_out.next().await {
        println!("{}", line?);
    }
    while let Some(line) = std_err.next().await {
        eprintln!("{}", line?);
    }

    Ok(())
}
