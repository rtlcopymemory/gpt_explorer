use std::env;
use std::fs;

mod args;
mod gpt;

use args::arg_parse::get_path;
use gpt::gpt::get_gpt_header;
use gpt::gpt::get_partition_table;
use gpt::gpt::verify_mbr;

fn bytes_to_guid(arr: &[u8]) -> String {
    if arr.len() < 16 {
        panic!("GUID length was {} instead of 16", arr.len());
    }

    let parts = [
        &arr[0x0..0x4]
            .iter()
            .map(|c| format!("{:02X}", c))
            .collect::<String>(),
        &arr[0x4..0x6]
            .iter()
            .map(|c| format!("{:02X}", c))
            .collect::<String>(),
        &arr[0x6..0x8]
            .iter()
            .map(|c| format!("{:02X}", c))
            .collect::<String>(),
        &arr[0x8..0xa]
            .iter()
            .map(|c| format!("{:02X}", c))
            .collect::<String>(),
        &arr[0xa..]
            .iter()
            .map(|c| format!("{:02X}", c))
            .collect::<String>(),
    ];

    let mut res = parts.iter().fold(String::from(""), |acc, x| acc + "-" + x);

    res.remove(0);

    res
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let path: String;

    if let Some(p) = get_path(args) {
        path = p;
    } else {
        panic!("Please provide a path to a raw image");
    }

    let buff = fs::read(path)?;

    println!("MBR: {:?}", verify_mbr(&buff));

    let header = get_gpt_header(&buff).expect("Messed up GPT header");
    //println!("GPT Sign: {:?}", header);

    let table = get_partition_table(&buff, header);
    //println!("\nPartition Table: {:?}", &table);

    println!("");

    for partition in table.partitions {
        println!("Type GUID: \t{}", bytes_to_guid(&*partition.type_guid));
        println!("GUID: \t\t{}", bytes_to_guid(&*partition.guid));
        println!("Name: \t\t{}", partition.name);
        println!("Start LBA: \t{}", partition.start_lba);
        println!("End LBA: \t{}", partition.end_lba);
        println!(
            "Size (MB): \t{}",
            (partition.end_lba - partition.start_lba) * 512 / 1024 / 1024
        );
        println!("Attributes: \t{}", partition.attributes);
        println!("========================================");
    }

    Ok(())
}
