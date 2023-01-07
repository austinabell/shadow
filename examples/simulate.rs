use std::io::{Read, Write};
use tempfile::tempfile;

fn main() {
    let mut file = tempfile().unwrap();
    let junk_data = vec![8; u32::MAX as usize];
    for _ in 0..4 {
        file.write(&junk_data).unwrap();
        println!("wrote data to file");

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("Error while reading file");
        println!("read all data from file");
    }

    eprintln!("finished writes");
    std::thread::sleep(std::time::Duration::from_secs(4));
}
