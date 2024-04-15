use memchr::memchr;
pub fn end_of_entry_position(data: &[u8]) -> Option<usize> {
    // data.iter().position(|&x| x == b'\x1e')
    memchr(b'\x1e', data)
}

pub fn end_of_subfield_position(data: &[u8]) -> Option<usize> {
    memchr(b'\x1f', data)
    //data.iter().position(|&x| x == b'\x1f')
}
pub struct OwnedRecordField {
    pub field_type: usize,
    pub data: Vec<u8>,
}

pub struct RecordField<'s> {
    pub field_type: usize,
    pub data: &'s [u8],
}

impl<'s> RecordField<'s> {
    pub fn utf8_data(&self) -> &str {
        std::str::from_utf8(self.data).unwrap()
    }
    pub fn to_owned(&self) -> OwnedRecordField {
        OwnedRecordField {
            field_type: self.field_type,
            data: self.data.to_vec(),
        }
    }
    pub fn is_data_field_type(field_type: usize) -> bool {
        // we have three digits for the tag, two leading zeros mean control type
        field_type >= 10
    }

    pub fn has_subfields(&self) -> bool {
        end_of_subfield_position(self.data).is_some()
    }
    /** For variable data fields, the first or second indicator, None for other fields **/
    pub fn indicator(&self, i: usize) -> Option<u8> {
        if !self.has_subfields() {
            return None;
        }
        assert!(i < 2);
        Some(self.data[i])
    }
    pub fn subfield_iter(&self) -> SubfieldIter<'s> {
        SubfieldIter { data: self.data }
    }
}

pub struct Subfield<'s> {
    data: &'s [u8],
}

pub struct SubfieldIter<'s> {
    data: &'s [u8],
}

impl<'s> Iterator for SubfieldIter<'s> {
    type Item = Subfield<'s>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pos) = end_of_subfield_position(self.data) {
            let r = &self.data[0..pos];
            self.data = &self.data[pos + 1..];
            Some(Subfield { data: r })
        } else if self.data.len() > 0 {
            let r = &self.data[0..];
            self.data = &[];
            Some(Subfield { data: r })
        } else {
            None
        }
    }
}

impl<'s> Subfield<'s> {
    pub fn utf8_data(&self) -> &str {
        std::str::from_utf8(self.data).unwrap()
    }
}

#[derive(std::cmp::PartialEq, Clone)]
pub enum RecordType {
    Authority = b'z' as isize,
}

impl RecordType {
    pub fn from_str(s: &str) -> Option<RecordType> {
        match s {
            "a" => Some(RecordType::Authority),
            "*" => None,
            _ => todo!(),
        }
    }
}

pub trait Record {
    fn record_type(&self) -> RecordType;
    // todo nightly features might avoid the box
    // https://stackoverflow.com/questions/39482131/is-it-possible-to-use-impl-trait-as-a-functions-return-type-in-a-trait-defini/39490692#39490692
    fn field_iter(&self, field_type: Option<usize>) -> Box<dyn Iterator<Item = RecordField> + '_>;
    fn field_iter_vec(&self, field_types: &[usize]) -> Box<dyn Iterator<Item = RecordField> + '_>;

    fn to_marc21(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()>;
}
