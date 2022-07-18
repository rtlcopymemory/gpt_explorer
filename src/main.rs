use std::env;
use std::fs;

mod args;
mod gpt;

use args::get_path;
use gpt::get_gpt_header;
use gpt::get_partition_table;
use gpt::verify_mbr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let path: String;

    if let Some(p) = get_path(&args) {
        path = p;
    } else {
        println!("\tUsage: {} <path_raw_image>", args[0]);
        return Ok(());
    }

    let buff = fs::read(path)?;

    verify_mbr(&buff).expect("MBR not correct");

    let header = get_gpt_header(&buff).expect("Messed up GPT header");

    let table = get_partition_table(&buff, header);

    for partition in table.partitions {
        print!("{}", partition);
        println!("========================================");
    }

    Ok(())
}
