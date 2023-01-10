use std::{
    fs::OpenOptions,
    io::{Read, Write},
};

fn main() -> std::io::Result<()> {
    // Create a temporary file
    let mut file = tempfile::NamedTempFile::new()?;

    // Get the path to the temporary file
    let path = file.path().to_path_buf();

    // 1GB of data
    let data = vec![8; 1_000_000_000];
    for _ in 0..3 {
        file.write_all(&data)?;
        file.flush()?;
        println!("wrote data to file");

        let mut read_file = OpenOptions::new().read(true).open(&path)?;
        let mut buffer = Vec::new();
        read_file
            .read_to_end(&mut buffer)
            .expect("Error while reading file");
        println!("read {} bytes from file", buffer.len());
    }

    eprintln!("finished writes");
    std::thread::sleep(std::time::Duration::from_secs(4));
    Ok(())
}
