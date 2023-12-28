use crate::record::*;
use crate::util::write_usize;
pub struct OwnedRecord {
    pub header: [u8; 24],
    pub field_types: Vec<usize>,
    pub field_data: Vec<Vec<u8>>,
}

impl OwnedRecord {
    pub fn new() -> OwnedRecord {
        OwnedRecord {
            header: [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            field_types: Vec::new(),
            field_data: Vec::new(),
        }
    }

    pub fn add_field(&mut self, field: OwnedRecordField) {
        self.field_types.push(field.field_type);
        self.field_data.push(field.data);
    }

    pub fn add_field_from_iter(&mut self, field_iter: &mut dyn Iterator<Item = RecordField>) {
        for field in field_iter {
            self.add_field(field.to_owned());
        }
        self.update_len();
    }

    fn update_len(&mut self) {
        let data_len: usize = self.field_data.iter().map(|x| x.len()).sum();
        let dict_len: usize = 12 * self.field_data.len();
        let mut l: usize = data_len + dict_len + self.header.len();
        for i in 0..5 {
            self.header[4 - i] = b'0' + (l % 10) as u8;
            l /= 10;
        }
    }
}

struct OwnedRecordFieldIter<'s> {
    i: usize,
    field_types: Vec<usize>,
    record: &'s OwnedRecord,
}

impl<'s> Iterator for OwnedRecordFieldIter<'s> {
    type Item = RecordField<'s>;
    fn next(&mut self) -> Option<Self::Item> {
        while self.i < self.record.field_types.len() {
            let idx = self.i;
            self.i += 1;
            let field_type = self.record.field_types[idx];
            if self.field_types.binary_search(&field_type).is_ok() || self.field_types.len() == 0 {
                let field_data = &self.record.field_data[idx];
                return Some(RecordField {
                    field_type,
                    data: field_data,
                });
            }
        }
        None
    }
}

impl Record for OwnedRecord {
    fn record_type(&self) -> RecordType {
        todo!();
    }
    fn field_iter_vec(&self, field_types: &[usize]) -> Box<dyn Iterator<Item = RecordField> + '_> {
        Box::new(OwnedRecordFieldIter {
            i: 0,
            field_types: field_types.to_vec(),
            record: &self,
        })
    }

    fn field_iter(&self, field_types: Option<usize>) -> Box<dyn Iterator<Item = RecordField> + '_> {
        // todo we probably don't want to alloc a vec here
        if let Some(x) = field_types {
            self.field_iter_vec(&[x])
        } else {
            self.field_iter_vec(&Vec::new())
        }
    }
    fn to_marc21(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        writer.write_all(&self.header)?;
        let prefix_length = 0; //self.header.len() + 12*self.field_types.len();
        let mut start = prefix_length;
        for (i, field_type) in self.field_types.iter().cloned().enumerate() {
            let field_len = self.field_data[i].len() + 1; // +1 for field separator
            write_usize(field_type, 3, writer)?;
            write_usize(field_len, 4, writer)?;
            write_usize(start, 5, writer)?;
            start += field_len;
        }
        writer.write_all(&[b'\x1e'])?;
        for field in self.field_data.iter() {
            writer.write_all(field.as_slice())?;
            writer.write_all(&[b'\x1e'])?;
        }
        writer.write_all(&[b'\x1d'])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::marcrecord::*;
    use crate::ownedrecord::*;
    use crate::record::*;
    use crate::MarcReader;
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
                let record: &MarcRecord<'_> = &batch.records[0];
                let mut result: Vec<u8> = Vec::new();
                record.to_marc21(&mut result).unwrap();
                assert_eq!(result, STR);
                let owned_record: OwnedRecord = (*record).to_owned();
                assert_eq!(owned_record.field_iter(None).count(), 18);
                result.clear();
                owned_record.to_marc21(&mut result).expect("not ok");
                assert_eq!(std::str::from_utf8(&result), std::str::from_utf8(STR));
                Ok(())
            }
            _ => Err("something bad".to_string()),
        }
    }
}
