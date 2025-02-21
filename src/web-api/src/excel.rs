use std::io::Cursor;

use base64::{Engine, prelude::BASE64_STANDARD};
use calamine::{RangeDeserializerBuilder, Reader, Xlsx, open_workbook_from_rs};
use serde::de::DeserializeOwned;

use crate::error::Result;

pub fn decode_excel_base64<T: DeserializeOwned>(input: impl AsRef<[u8]>) -> Result<Vec<T>> {
    let decoded = BASE64_STANDARD.decode(input)?;
    decode_excel(decoded)
}

pub fn decode_excel<T: DeserializeOwned>(input: impl AsRef<[u8]>) -> Result<Vec<T>> {
    let mut doc: Xlsx<_> = open_workbook_from_rs(Cursor::new(input))?;

    let mut vec = Vec::new();

    for (_, range) in doc.worksheets() {
        let iter = RangeDeserializerBuilder::new()
            .has_headers(false)
            .from_range(&range)?;

        for record in iter {
            vec.push(record?);
        }
    }

    Ok(vec)
}
