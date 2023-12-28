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
