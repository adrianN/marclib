#![allow(dead_code)]
use crate::marcrecord::*;
use crate::record::*;

pub enum AuthorityRecordStatus {
    IncreaseEncodingLevel = b'a' as isize,
    Corrected = b'c' as isize,
    Deleted = b'd' as isize,
    New = b'n' as isize,
    Obsolete = b'o' as isize,
    Split = b's' as isize,
    Replaced = b'x' as isize,
}

pub enum AuthorityRecordCharacterCodingScheme {
    Marc8 = b'#' as isize,
    Unicode = b'a' as isize,
}

pub struct AuthorityRecordMeta {
    record_type: RecordType,
    status: AuthorityRecordStatus,
    character_coding_scheme: AuthorityRecordCharacterCodingScheme,
    // TODO we probably want to use an arena for these
    field_types: Vec<usize>,
    field_offsets: Vec<usize>,
    field_lengths: Vec<usize>,
}

// TODO we could implement a builder pattern to reuse things we already
// parsed during pre-filtering
impl AuthorityRecordMeta {
    pub fn empty_new() -> AuthorityRecordMeta {
        AuthorityRecordMeta {
            record_type: RecordType::Authority,
            status: AuthorityRecordStatus::New,
            character_coding_scheme: AuthorityRecordCharacterCodingScheme::Unicode,
            field_types: Vec::new(),
            field_offsets: Vec::new(),
            field_lengths: Vec::new(),
        }
    }

    pub fn new(r: &MarcRecord, dir: &MarcDirectory) -> AuthorityRecordMeta {
        let t = r.header().record_type();
        assert!(t == RecordType::Authority);

        // todo check whether other record types than authority parse differently here
        // and maybe move stuff to MarcHeader
        let s = match r.header().header[5] {
            b'a' => AuthorityRecordStatus::IncreaseEncodingLevel,
            b'c' => AuthorityRecordStatus::Corrected,
            b'd' => AuthorityRecordStatus::Deleted,
            b'n' => AuthorityRecordStatus::New,
            b'o' => AuthorityRecordStatus::Obsolete,
            b's' => AuthorityRecordStatus::Split,
            b'x' => AuthorityRecordStatus::Replaced,
            _ => panic!("oopsie"),
        };

        let coding_scheme = match r.header().header[9] {
            b'a' => AuthorityRecordCharacterCodingScheme::Unicode,
            _ => unimplemented!(),
        };

        // todo the remaining fields of the header

        let dir_len = dir.num_entries();

        let mut field_types = Vec::with_capacity(dir_len);
        let mut field_offsets = Vec::with_capacity(dir_len);
        let mut field_lengths = Vec::with_capacity(dir_len);

        for i in 0..dir_len {
            let entry = dir.get_entry(i);
            field_types.push(entry.entry_type());
            field_offsets.push(entry.start() + 1); // skip the entry sep char
            field_lengths.push(entry.len() - 1);
        }

        AuthorityRecordMeta {
            record_type: t,
            status: s,
            character_coding_scheme: coding_scheme,
            field_types,
            field_offsets,
            field_lengths,
        }
    }
    pub fn num_fields(&self) -> usize {
        self.field_types.len()
    }

    pub fn add_field(&mut self, field_type: usize, field_start: usize, field_len: usize) {
        self.field_types.push(field_type);
        assert!(self.field_lengths.iter().sum::<usize>() == field_start);
        self.field_offsets.push(field_start);
        self.field_lengths.push(field_len);
    }
    pub fn record_type(&self) -> RecordType {
        self.record_type.clone()
    }
}

pub enum RecordMeta {
    AuthorityMeta(AuthorityRecordMeta),
}

impl RecordMeta {
    pub fn empty_new(_t: RecordType) -> RecordMeta {
        //todo other types
        assert!(_t == RecordType::Authority);
        RecordMeta::AuthorityMeta(AuthorityRecordMeta::empty_new())
    }
    pub fn new(r: &MarcRecord, d: &MarcDirectory) -> RecordMeta {
        match r.header().record_type() {
            RecordType::Authority => RecordMeta::AuthorityMeta(AuthorityRecordMeta::new(r, d)),
        }
    }

    pub fn record_type(&self) -> RecordType {
        match self {
            Self::AuthorityMeta(record_meta) => record_meta.record_type(),
        }
    }

    pub fn get_field_type(&self, idx: usize) -> usize {
        match self {
            Self::AuthorityMeta(record_meta) => record_meta.field_types[idx],
        }
    }

    pub fn get_field_offset(&self, idx: usize) -> usize {
        match self {
            Self::AuthorityMeta(record_meta) => record_meta.field_offsets[idx],
        }
    }

    pub fn get_field_length(&self, idx: usize) -> usize {
        match self {
            Self::AuthorityMeta(record_meta) => record_meta.field_lengths[idx],
        }
    }

    pub fn get_field<'s>(&self, idx: usize, record_data: &'s [u8]) -> RecordField<'s> {
        let field_type = self.get_field_type(idx);
        let field_offset = self.get_field_offset(idx);
        let field_length = self.get_field_length(idx);
        RecordField {
            field_type,
            data: &record_data[field_offset..field_offset + field_length],
        }
    }

    pub fn num_fields(&self) -> usize {
        match self {
            Self::AuthorityMeta(record_meta) => record_meta.num_fields(),
        }
    }

    pub fn add_field(&mut self, field_type: usize, field_start: usize, field_len: usize) {
        match self {
            Self::AuthorityMeta(record_meta) => {
                record_meta.add_field(field_type, field_start, field_len)
            }
        }
    }
}

pub struct ParsedRecord {
    meta: RecordMeta,
    // Todo we definitely want to use an arena for this
    field_data: Vec<u8>,
}

impl ParsedRecord {
    pub fn new(r: &MarcRecord) -> ParsedRecord {
        let dir = r.directory();
        ParsedRecord {
            meta: RecordMeta::new(r, &dir),
            field_data: r.data()[dir.byte_len()..].to_vec(),
        }
    }

    pub fn empty_new(t: RecordType) -> ParsedRecord {
        ParsedRecord {
            meta: RecordMeta::empty_new(t),
            field_data: Vec::new(),
        }
    }

    pub fn add_field(&mut self, field_type: usize, field_data: &[u8]) {
        self.meta
            .add_field(field_type, self.field_data.len(), field_data.len());
        self.field_data.extend_from_slice(field_data);
    }

    fn field_data(&self) -> &[u8] {
        &self.field_data
    }
    pub fn num_fields(&self) -> usize {
        self.meta.num_fields()
    }

    pub fn get_field(&self, idx: usize) -> RecordField {
        self.meta.get_field(idx, self.field_data())
    }
}

impl Record for ParsedRecord {
    fn record_type(&self) -> RecordType {
        self.meta.record_type()
    }
    fn field_iter(&self, field_type: Option<usize>) -> Box<dyn Iterator<Item = RecordField> + '_> {
        Box::new(ParsedRecordFieldIter::new(self, field_type))
    }

    fn field_iter_vec(&self, field_type: &[usize]) -> Box<dyn Iterator<Item = RecordField> + '_> {
        todo!()
    }

    fn to_marc21(&self, _writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        todo!()
    }
}

pub struct ParsedRecordFieldIter<'s> {
    record: &'s ParsedRecord,
    idx: usize,
    field_type: Option<usize>,
}

impl<'s> ParsedRecordFieldIter<'s> {
    pub fn new(r: &'s ParsedRecord, field_type: Option<usize>) -> ParsedRecordFieldIter<'s> {
        ParsedRecordFieldIter {
            record: r,
            idx: 0,
            field_type,
        }
    }
}

impl<'s> Iterator for ParsedRecordFieldIter<'s> {
    type Item = RecordField<'s>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.idx < self.record.num_fields() {
            // todo pushing the type check into get_field is a good
            // optimization
            let field = self.record.get_field(self.idx);
            self.idx += 1;
            if self.field_type.map_or(true, |t| t == field.field_type) {
                return Some(field);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::marcrecord::MarcHeader;
    use crate::marcrecord::MarcRecord;
    use crate::parsedrecord::ParsedRecord;
    use crate::record::*;
    static str : &[u8]= "00827nz  a2200241nc 4500\
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
    fn parse_one() -> Result<(), String> {
        let header = MarcHeader::new(&str[..24]);
        let unparsed_record = MarcRecord::new(header, &str[24..]);
        let parsed_record = ParsedRecord::new(&unparsed_record);
        assert_eq!(parsed_record.num_fields(), 18);
        assert_eq!(
            parsed_record.field_iter(None).count(),
            parsed_record.num_fields()
        );
        assert_eq!(parsed_record.field_iter(Some(35)).count(), 3);
        let mut it = parsed_record.field_iter(None);
        let first = it.next().ok_or_else(|| "not enough elements")?;
        let last = it.last().ok_or_else(|| "not enough elements")?;
        assert_eq!(first.utf8_data(), "040000028");
        assert_eq!(last.utf8_data(), "  SswdisaA 302 D0(DE-588c)4000002-3");
        Ok(())
    }
}
