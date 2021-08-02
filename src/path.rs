use std::io::{Result, Read, BufRead, BufReader, Error, ErrorKind, Cursor};
use flate2::read::MultiGzDecoder;
use std::path::Path;

use crate::record::{Reader, Readers, Fastx};

/// Reader for a single path or Readers for multiple paths
pub enum Paths {
    Reader(Reader),
    Readers(Readers)
}

impl Paths {
    /// iterate a fatsx record for a Reader or Readers
    pub fn iter_record(&mut self) -> Option<Fastx> {
        match self {
            Paths::Reader(t) => t.iter_record_check(),
            Paths::Readers(t) => t.iter_record(),
        }
    }
}

/// parse path to a Reader or Readers
pub fn parse_path(path: Option<String>) -> Result<Paths>{
    let mut reader: Box<dyn BufRead> = match path.as_ref().map(String::as_str) {
        None | Some("-") => {
            Box::new(BufReader::with_capacity(65536, std::io::stdin()))
        },
        Some(path) => {
            Box::new(BufReader::with_capacity(65536, std::fs::File::open(path)?))
        }
    };
    let mut format_bytes = [0u8; 4];
    reader.read_exact(&mut format_bytes)?;
    reader = Box::new(Cursor::new(format_bytes.to_vec()).chain(reader));
    if &format_bytes[..2] == b"\x1f\x8b" {// for gz foramt
        reader = Box::new(BufReader::with_capacity(65536, MultiGzDecoder::new(reader)));
        format_bytes.iter_mut().for_each(|m| *m = 0);
        reader.read_exact(&mut format_bytes)?;
        reader = Box::new(Cursor::new(format_bytes.to_vec()).chain(reader));
    }

    match format_bytes[0] {
        b'@' | b'>' => {
            Ok(Paths::Reader(Reader::new(reader)))
        }
        _ => {// for a fofn file
            let mut paths = Readers::new();
            for _line in reader.lines().map(|l| l.unwrap()){
                let line = _line.trim();
                if line.starts_with('#') || line.is_empty(){
                    continue;
                }
                if Path::new(line).exists(){
                    match parse_path(Some(line.to_string())).unwrap() {
                        Paths::Reader(reader) => paths.readers.push(reader),
                        _ => unreachable!()
                    }
                }else{
                    return Err(Error::new(ErrorKind::InvalidData, "Not a valid fastq/fasta/fofn file"))
                }
            }
            Ok(Paths::Readers(paths))
        }
    }
}
