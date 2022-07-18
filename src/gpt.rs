use std::fmt::Display;

use bytes::{ByteOrder, LittleEndian};

extern crate bytes;

//use byteorder::{ByteOrder, LittleEndian};

const GPT_SIGN: &[u8] = b"EFI PART";
const SECT_SIZE: usize = 512; // Find a way to make this settable
const MBR_LEN: usize = 512; // Which is also a sector

#[derive(Debug)]
pub enum GPTError {
    BuffLenError,
    MBRError,
    GTPSignError,
}

//#[derive(Debug)]
pub struct GptHeader<'a> {
    pub signature: Box<&'a [u8]>,
    pub revision: Box<&'a [u8]>,
    pub size: u32,
    pub head_crc32: u32,
    // 4 null bytes
    pub my_lba: u64,
    pub alt_lba: u64,
    pub first_usable_lba: u64,
    pub last_usable_lba: u64,
    pub guid: Box<&'a [u8]>,
    pub partition_lba: u64, // first sector of partition table
    pub num_partitions: u32,
    pub size_part_entry: u32,
    pub part_crc32: u32,
}

#[derive(Debug)]
pub struct Partition {
    pub type_guid: Box<[u8; 16]>,
    pub guid: Box<[u8; 16]>,
    pub start_lba: u64,
    pub end_lba: u64,
    pub attributes: u64,
    pub name: String,
}

impl Display for Partition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Type GUID: \t{}", bytes_to_guid(&*self.type_guid))?;
        writeln!(f, "GUID: \t\t{}", bytes_to_guid(&*self.guid))?;
        writeln!(f, "Name: \t\t{}", self.name)?;
        writeln!(f, "Start LBA: \t{}", self.start_lba)?;
        writeln!(f, "End LBA: \t{}", self.end_lba)?;
        writeln!(
            f,
            "Size (MB): \t{}",
            (self.end_lba - self.start_lba) * 512 / 1024 / 1024
        )?;
        writeln!(f, "Attributes: \t{}", self.attributes)
    }
}

#[derive(Debug)]
pub struct PartitionTable {
    pub partitions: Vec<Partition>,
}

pub fn verify_mbr(buff: &Vec<u8>) -> Result<(), GPTError> {
    if buff.len() < MBR_LEN {
        return Err(GPTError::BuffLenError);
    }

    if buff[510] != 0x55 || buff[511] != 0xaa {
        return Err(GPTError::MBRError);
    }

    Ok(())
}

pub fn get_gpt_header(buff: &Vec<u8>) -> Result<GptHeader, GPTError> {
    if buff.len() < 0x210 {
        return Err(GPTError::BuffLenError);
    }

    let sign = &buff[MBR_LEN..(MBR_LEN + GPT_SIGN.len())];
    if sign != GPT_SIGN || sign.len() != 8 {
        return Err(GPTError::GTPSignError);
    }

    let head_len: usize = LittleEndian::read_u32(&buff[(MBR_LEN + 0xC)..=(MBR_LEN + 0xF)])
        .try_into()
        .unwrap();

    let header = &buff[MBR_LEN..=(MBR_LEN + head_len)];

    Ok(GptHeader {
        signature: Box::new(sign.clone()),
        revision: Box::new(&header[0x8..=0xB]),
        size: LittleEndian::read_u32(&header[0xC..=0xF]),
        head_crc32: LittleEndian::read_u32(&header[0x10..=0x14]),
        // 4 null bytes
        my_lba: LittleEndian::read_u64(&header[0x18..=0x1F]),
        alt_lba: LittleEndian::read_u64(&header[0x20..=0x27]),
        first_usable_lba: LittleEndian::read_u64(&header[0x28..=0x2F]),
        last_usable_lba: LittleEndian::read_u64(&header[0x30..=0x37]),
        guid: Box::new(&header[0x38..=0x47]),
        partition_lba: LittleEndian::read_u64(&header[0x48..=0x4F]),
        num_partitions: LittleEndian::read_u32(&header[0x50..=0x53]),
        size_part_entry: LittleEndian::read_u32(&header[0x54..=0x57]),
        part_crc32: LittleEndian::read_u32(&header[0x58..=0x5B]),
    })
}

pub fn get_partition_table_buf(buff: &Vec<u8>, header: &GptHeader) -> Vec<u8> {
    let table_addr: usize = SECT_SIZE * header.partition_lba as usize;
    let table_end = table_addr + header.num_partitions as usize * header.size_part_entry as usize;

    buff[table_addr..table_end]
        .try_into()
        .expect("Failed to convert slice into Vec at get_partition table")
}

fn read_name(slice: &[u8]) -> String {
    String::from_utf16(
        &slice
            .chunks_exact(2)
            .into_iter()
            .map(|a| u16::from_ne_bytes([a[0], a[1]]))
            .collect::<Vec<u16>>(),
    )
    .expect("Failed to convert from UTF16 to String")
}

pub fn get_partition_table(buff: &Vec<u8>, header: GptHeader) -> PartitionTable {
    let table_buf = get_partition_table_buf(buff, &header);
    PartitionTable {
        partitions: table_buf
            .chunks(header.size_part_entry as usize)
            .map(|a| Partition {
                type_guid: Box::new(a[0x0..=0xf].try_into().expect("Couldn't get type guid")),
                guid: Box::new(a[0x10..=0x1f].try_into().expect("Couldn't get guid")),
                start_lba: LittleEndian::read_u64(&a[0x20..=0x27]),
                end_lba: LittleEndian::read_u64(&a[0x28..=0x2F]),
                attributes: LittleEndian::read_u64(&a[0x30..=0x37]),
                // name: std::str::from_utf8(&a[0x38..]).unwrap().to_owned(),
                name: read_name(&a[0x38..]),
            })
            .filter(|e| *e.type_guid != [0; 16])
            .collect(),
    }
}

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
