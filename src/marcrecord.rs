use crate::util::*;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

use crate::ownedrecord::OwnedRecord;
use crate::record::*;

const MARCHEADER_SIZE: usize = 24;

#[derive(Debug)]
pub struct MarcHeader<'s> {
    pub header: &'s [u8],
}

#[derive(Debug)]
pub struct MarcRecord<'s> {
    header: MarcHeader<'s>,
    data: &'s [u8],
}

pub struct MarcRecordEntries<'s> {
    directory: MarcDirectory<'s>,
    record_payload: &'s [u8],
}

impl<'s> MarcHeader<'s> {
    pub fn new(data: &'s [u8]) -> MarcHeader {
        assert!(data.len() == MARCHEADER_SIZE);
        MarcHeader { header: data }
    }

    pub fn record_length(&self) -> usize {
        parse_usize5(&self.header[0..5])
    }
    pub fn record_type(&self) -> RecordType {
        match self.header[6] {
            b'z' => RecordType::Authority,
            _ => todo!(),
        }
    }
}

// todo we want to iter over this
pub struct MarcDirectory<'s> {
    directory: &'s [u8],
}

#[derive(Debug)]
pub struct MarcDirectoryEntryRef<'s> {
    entry: &'s [u8],
}

impl<'s> MarcDirectoryEntryRef<'s> {
    pub fn entry_type(&self) -> usize {
        parse_usize3(&self.entry[0..3])
    }
    pub fn len(&self) -> usize {
        parse_usize4(&self.entry[3..7])
    }
    pub fn start(&self) -> usize {
        parse_usize5(&self.entry[7..12])
    }
}

impl<'s> MarcDirectory<'s> {
    pub fn get_entry(&self, i: usize) -> MarcDirectoryEntryRef {
        MarcDirectoryEntryRef {
            entry: &self.directory[12 * i..12 * (i + 1)],
        }
    }
    pub fn num_entries(&self) -> usize {
        self.byte_len() / 12
    }
    pub fn byte_len(&self) -> usize {
        self.directory.len()
    }
}

impl<'s> MarcRecord<'s> {
    pub fn new(h: MarcHeader<'s>, data: &'s [u8]) -> MarcRecord<'s> {
        MarcRecord {
            header: h,
            data: &data,
        }
    }

    pub fn header(&self) -> &MarcHeader<'s> {
        &self.header
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn record_length(&self) -> usize {
        self.data.len() + MARCHEADER_SIZE
    }
    pub fn directory(&self) -> MarcDirectory<'s> {
        let directory_end = end_of_entry_position(&self.data);
        MarcDirectory {
            directory: &self.data[0..directory_end.expect("malformed entry")],
        }
    }

    pub fn entries(&self) -> MarcRecordEntries<'s> {
        let d = self.directory();
        let d_len = d.byte_len();
        MarcRecordEntries {
            directory: d,
            record_payload: &self.data[d_len..],
        }
    }

    pub fn to_owned(&self) -> OwnedRecord {
        let mut record = OwnedRecord::new();
        for i in 0..record.header.len() {
            record.header[i] = self.header.header[i];
        }
        for entry in self.field_iter(None) {
            record.add_field(entry.to_owned());
        }

        record
    }
}

pub struct MarcRecordFieldIter<'s> {
    entries: MarcRecordEntries<'s>,
    idx: usize,
    field_type: Option<usize>,
}

impl<'s> MarcRecordFieldIter<'s> {
    pub fn new(r: &'s MarcRecord, field_type: Option<usize>) -> MarcRecordFieldIter<'s> {
        MarcRecordFieldIter {
            entries: r.entries(),
            idx: 0,
            field_type,
        }
    }
}

impl<'s> Iterator for MarcRecordFieldIter<'s> {
    type Item = RecordField<'s>;
    fn next(&mut self) -> Option<Self::Item> {
        let num_entries = self.entries.directory.num_entries();
        while self.idx < num_entries {
            let entry_ref = self.entries.directory.get_entry(self.idx);
            self.idx += 1;
            let entry_type = entry_ref.entry_type();
            if self.field_type.map_or(true, |t| t == entry_type) {
                // +1 because we want to skip the field separator
                let start = entry_ref.start() + 1;
                return Some(RecordField {
                    field_type: entry_type,
                    data: &self.entries.record_payload[start..start + entry_ref.len() - 1], // -1 because we skipped the field separator
                });
            }
        }
        None
    }
}

// TODO express non-vec version using this?
pub struct MarcRecordFieldIterVec<'s> {
    entries: MarcRecordEntries<'s>,
    idx: usize,
    field_types: Vec<usize>,
}

impl<'s> MarcRecordFieldIterVec<'s> {
    pub fn new(r: &'s MarcRecord, field_types: &[usize]) -> MarcRecordFieldIterVec<'s> {
        MarcRecordFieldIterVec {
            entries: r.entries(),
            idx: 0,
            field_types: field_types.to_vec(), // todo avoid this copy, field_types should outlive the iterator
        }
    }
}

impl<'s> Iterator for MarcRecordFieldIterVec<'s> {
    type Item = RecordField<'s>;
    fn next(&mut self) -> Option<Self::Item> {
        let num_entries = self.entries.directory.num_entries();
        while self.idx < num_entries {
            let entry_ref = self.entries.directory.get_entry(self.idx);
            self.idx += 1;
            let entry_type = entry_ref.entry_type();
            if self.field_types.binary_search(&entry_type).is_ok() {
                // +1 because we want to skip the field separator
                let start = entry_ref.start() + 1;
                return Some(RecordField {
                    field_type: entry_type,
                    data: &self.entries.record_payload[start..start + entry_ref.len() - 1], // -1 because we skipped the field separator
                });
            }
        }
        None
    }
}

impl<'s> Record for MarcRecord<'s> {
    fn record_type(&self) -> RecordType {
        self.header().record_type()
    }
    fn field_iter(&self, field_type: Option<usize>) -> Box<dyn Iterator<Item = RecordField> + '_> {
        Box::new(MarcRecordFieldIter::new(&self, field_type))
    }

    fn field_iter_vec(&self, field_types: &[usize]) -> Box<dyn Iterator<Item = RecordField> + '_> {
        Box::new(MarcRecordFieldIterVec::new(&self, field_types))
    }

    fn to_marc21(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        writer.write_all(self.header().header)?;
        writer.write_all(self.data)?;
        Ok(())
    }
}

// Todo we want to be able to iter over this
#[derive(Debug)]
pub struct MarcRecordBatch<'s> {
    pub records: Vec<MarcRecord<'s>>,
}

pub struct BufferedMarcReader<R>
where
    R: Read + Seek,
{
    base_reader: R,
    buffer: Vec<u8>,
    offset: usize,
    eof: bool,
}

impl<R> BufferedMarcReader<R>
where
    R: Read + Seek,
{
    pub fn new(reader: R) -> BufferedMarcReader<R> {
        BufferedMarcReader {
            base_reader: reader,
            buffer: Vec::new(),
            offset: 0,
            eof: false,
        }
    }

    pub fn is_eof(&self) -> bool {
        return self.eof;
    }

    pub fn get_header(&self) -> Option<MarcHeader> {
        if self.buffer.len() - self.offset < MARCHEADER_SIZE {
            return None;
        }
        Some(MarcHeader::new(
            &self.buffer[self.offset..self.offset + MARCHEADER_SIZE],
        ))
    }

    pub fn get(&self) -> Option<MarcRecord> {
        let header = self.get_header()?;
        let record_length = header.record_length();
        if self.buffer.len() - self.offset < record_length {
            return None;
        }
        Some(MarcRecord::new(
            header,
            &self.buffer[self.offset + MARCHEADER_SIZE..self.offset + record_length],
        ))
    }

    /**
     * Return true if we advanced successfully
     */
    pub fn advance(&mut self) -> Result<bool, std::io::Error> {
        let current_record = self.get();
        if let Some(record) = current_record {
            // we already have a record in the buffer, advance to the next record
            let record_length = record.header.record_length();
            assert!(self.buffer.len() - self.offset >= record_length);
            self.offset += record_length;
            let success = self.refill_buffer(MARCHEADER_SIZE)?; // TODO EOF
            if !success {
                assert!(self.eof);
                return Ok(false);
            }
            // and now read the next record
            let next_record_length =
                MarcHeader::new(&self.buffer[self.offset..self.offset + MARCHEADER_SIZE])
                    .record_length();
            let success = self.refill_buffer(next_record_length)?;
            Ok(!self.eof)
        } else {
            // we don't have a record in the buffer, read one record
            self.refill_buffer(MARCHEADER_SIZE)?;
            if let Some(header) = self.get_header() {
                self.refill_buffer(header.record_length())?;
            }
            Ok(!self.eof)
        }
    }

    /**
     * Returns true if we read at least read_size, false if we failed to do so
     */
    fn refill_buffer(&mut self, read_size: usize) -> Result<bool, std::io::Error> {
        if self.buffer.len() - self.offset > read_size {
            return Ok(true);
        }
        if self.offset != 0 {
            // copy the end to the beginning
            let mut i = 0;
            for j in self.offset..self.buffer.len() {
                self.buffer[i] = self.buffer[j];
                i += 1;
            }
            self.buffer.truncate(self.buffer.len() - self.offset);
            self.offset = 0;
        }
        let old_size = self.buffer.len();
        self.buffer.resize(read_size, 0);
        // fill the buffer
        let read = self.base_reader.read(&mut self.buffer[old_size..])?;
        self.buffer.truncate(read + old_size);
        self.eof = self.buffer.len() < read_size;
        Ok(self.buffer.len() >= read_size)
    }
}

#[derive(Debug)]
pub struct MarcReader<R>
where
    R: Read + Seek,
{
    base_reader: R,
}

impl<R> MarcReader<R>
where
    R: Read + Seek,
{
    // TODO maybe take R instead of BufReader
    pub fn new(reader: R) -> MarcReader<R> {
        MarcReader {
            base_reader: reader,
        }
    }

    pub fn read_batch<'s>(
        &mut self,
        mem: &'s mut [u8],
    ) -> Result<Option<MarcRecordBatch<'s>>, std::io::Error> {
        let mut records: Vec<MarcRecord> = Vec::with_capacity(mem.len() / 10000);
        let mut i = 0;
        let start_pos = self.base_reader.stream_position().unwrap();
        let read = self.base_reader.read(mem)?;
        if read == 0 {
            return Ok(None);
        }
        while i + MARCHEADER_SIZE < read {
            let header = MarcHeader {
                header: &mem[i..i + MARCHEADER_SIZE],
            };
            let record_length = header.record_length();
            //assert!(record_length < mem.len());
            if record_length + i <= read {
                // still fits in mem
                records.push(MarcRecord::new(
                    header,
                    &mem[i + MARCHEADER_SIZE..i + record_length],
                ));
                i += record_length;
            } else {
                break;
            }
        }
        if i == 0 {
            use std::io::{Error, ErrorKind};
            return Err(Error::new(
                ErrorKind::InvalidData,
                "failed to read a single record",
            ));
        }
        // mem full, backpedal
        //self.base_reader.seek_relative(-MARCHEADER_SIZE);
        // TODO seek_relative is unstable in my version of rust
        self.base_reader
            .seek(SeekFrom::Start(start_pos + i as u64))?;
        //        let num_bytes = records
        //            .iter()
        //            .map(|r| r.header().record_length())
        //            .sum::<usize>() as u64;
        //        let stream_pos = self.base_reader.stream_position().unwrap();
        //        let bytes_consumed = stream_pos - start_pos;
        //        assert!(bytes_consumed == (num_bytes));

        Ok(Some(MarcRecordBatch { records }))
    }
}

#[cfg(test)]
mod tests {
    use crate::marcrecord::*;
    use crate::record::*;
    use std::io::BufReader;
    use std::io::Cursor;
    static STR : &[u8]= "00827nz  a2200241nc 4500\
001001000000\
003000700010\
005001700017\
008004100034\
024005100075\
035002200126\
035002200148\
035002900170\
040004000199\
042000900239\
065001600248\
075001400264\
079000900278\
083004200287\
150001200329\
550019200341\
670001200533\
913004000545\
040000028DE-10120100106125650.0880701n||azznnbabn           | ana    |c7 a4000002-30http://d-nb.info/gnd/4000002-32gnd  a(DE-101)040000028  a(DE-588)4000002-3  z(DE-588c)4000002-39v:zg  aDE-101cDE-1019r:DE-101bgerd0832  agnd1  a31.9b2sswd  bs2gndgen  agqs04a621.3815379d:29t:2010-01-06223/ger  aA 302 D  0(DE-101)0402724270(DE-588)4027242-40https://d-nb.info/gnd/4027242-4aIntegrierte Schaltung4obal4https://d-nb.info/standards/elementset/gnd#broaderTermGeneralwriOberbegriff allgemein  aVorlage  SswdisaA 302 D0(DE-588c)4000002-3".as_bytes();

    #[test]
    fn read_one_buffered() -> Result<(), String> {
        dbg!(STR.len());
        let mut bigStr = Vec::new();
        for i in 0..2 {
            bigStr.extend_from_slice(STR);
        }
        let len = bigStr.len();
        bigStr[len - 4] = '$' as u8;
        let c = Cursor::new(bigStr);
        let breader = BufReader::new(c);
        let mut mreader = BufferedMarcReader::new(breader);
        assert!(!mreader.is_eof());
        for i in 0..2 {
            let success = mreader.advance().expect("io error");
            assert!(!mreader.is_eof());
            assert!(success);
            let record = mreader.get().unwrap();
            assert_eq!(record.record_length(), 827);
            let dir = record.directory();
            dbg!(std::str::from_utf8(dir.directory).unwrap());
            assert_eq!(dir.num_entries(), 18);
            let entry_types = [
                1, 3, 5, 8, 24, 35, 35, 35, 40, 42, 65, 75, 79, 83, 150, 550, 670, 913,
            ];
            let entry_lengths = [
                10, 7, 17, 41, 51, 22, 22, 29, 40, 9, 16, 14, 9, 42, 12, 192, 12, 40,
            ];
            let entry_starts = [
                0, 10, 17, 34, 75, 126, 148, 170, 199, 239, 248, 264, 278, 287, 329, 341, 533, 545,
            ];

            for i in 0..18 {
                let entry = dir.get_entry(i);
                dbg!(std::str::from_utf8(entry.entry).unwrap());
                assert_eq!(entry.entry_type(), entry_types[i], "i {}", i);
                assert_eq!(entry.len(), entry_lengths[i], "i {}", i);
                assert_eq!(entry.start(), entry_starts[i], "i {}", i);
            }
            let mut it = record.field_iter(None);
            let first = it.next().ok_or_else(|| "not enough elements")?;
            let last = it.last().ok_or_else(|| "not enough elements")?;
            assert_eq!(first.utf8_data(), "040000028");
            if i == 0 {
                assert_eq!(last.utf8_data(), "  SswdisaA 302 D0(DE-588c)4000002-3");
            } else {
                assert_eq!(last.utf8_data(), "  SswdisaA 302 D0(DE-588c)4000002$3");
            }
        }
        let success = mreader.advance().expect("io error");
        assert!(!success);

        assert!(mreader.is_eof());
        Ok(())
    }

    #[test]
    fn read_one() -> Result<(), String> {
        dbg!(STR.len());
        let c = Cursor::new(STR);
        let breader = BufReader::new(c);
        let mut mreader = MarcReader::new(breader);
        let mut v: Vec<u8> = Vec::new();
        v.resize(10000, 0);
        let r = mreader.read_batch(&mut v);
        match r {
            Ok(Some(batch)) => {
                assert_eq!(batch.records.len(), 1);
                let record = &batch.records[0];
                assert_eq!(record.record_length(), 827);
                let dir = record.directory();
                dbg!(std::str::from_utf8(dir.directory).unwrap());
                assert_eq!(dir.num_entries(), 18);
                let entry_types = [
                    1, 3, 5, 8, 24, 35, 35, 35, 40, 42, 65, 75, 79, 83, 150, 550, 670, 913,
                ];
                let entry_lengths = [
                    10, 7, 17, 41, 51, 22, 22, 29, 40, 9, 16, 14, 9, 42, 12, 192, 12, 40,
                ];
                let entry_starts = [
                    0, 10, 17, 34, 75, 126, 148, 170, 199, 239, 248, 264, 278, 287, 329, 341, 533,
                    545,
                ];

                for i in 0..18 {
                    let entry = dir.get_entry(i);
                    dbg!(std::str::from_utf8(entry.entry).unwrap());
                    assert_eq!(entry.entry_type(), entry_types[i], "i {}", i);
                    assert_eq!(entry.len(), entry_lengths[i], "i {}", i);
                    assert_eq!(entry.start(), entry_starts[i], "i {}", i);
                }
                let mut it = record.field_iter(None);
                let first = it.next().ok_or_else(|| "not enough elements")?;
                let last = it.last().ok_or_else(|| "not enough elements")?;
                assert_eq!(first.utf8_data(), "040000028");
                assert_eq!(last.utf8_data(), "  SswdisaA 302 D0(DE-588c)4000002-3");
                Ok(())
            }
            _ => Err("something bad".to_string()),
        }
    }

    #[test]
    fn conv_back() -> Result<(), String> {
        let c = Cursor::new(STR);
        let breader = BufReader::new(c);
        let mut mreader = MarcReader::new(breader);
        let mut v: Vec<u8> = Vec::new();
        v.resize(10000, 0);
        let r = mreader.read_batch(&mut v);
        match r {
            Ok(Some(batch)) => {
                assert_eq!(batch.records.len(), 1);
                let record = &batch.records[0];
                let mut result: Vec<u8> = Vec::new();
                record.to_marc21(&mut result).unwrap();
                assert_eq!(result, STR);
                Ok(())
            }
            _ => Err("something bad".to_string()),
        }
    }
}
