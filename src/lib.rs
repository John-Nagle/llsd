//
//  Library for serializing and de-serializing data in
//  Linden Lab Structured Data format.
//
//  Format documentation is at http://wiki.secondlife.com/wiki/LLSD
//
//  Animats
//  February, 2021.
//  License: LGPL.
//
//
//  Modules
//
pub mod binary;
pub mod xml;
//
use std::collections::HashMap;
use uuid;
use anyhow::{anyhow, Error};
//
///  The primitive LLSD data item.
#[derive(Debug, Clone, PartialEq)]
pub enum LLSDValue {
    Undefined,
    Boolean(bool),
    Real(f64),
    Integer(i32),
    UUID(uuid::Uuid),
    String(String),
    Date(i64),
    URI(String),
    Binary(Vec<u8>),
    Map(HashMap<String, LLSDValue>),
    Array(Vec<LLSDValue>),
}

//  Implementation

impl LLSDValue {

    /// Parse LLSD, detecting format.
    pub fn parse(msg: &[u8]) -> Result<LLSDValue, Error> {
        //  Try binary first
        if msg.len() >= binary::LLSDBINARYSENTINEL.len() &&
            &msg[0..binary::LLSDBINARYSENTINEL.len()] == binary::LLSDBINARYSENTINEL {
                return binary::parse(&msg[binary::LLSDBINARYSENTINEL.len()..]) }
        //  Check for binary without header. If array or map marker, parse.
        if msg.len() > 1 && msg[0] == msg[msg.len()-1] {
            match msg[0] {                          // check first char
                b'{'| b'[' => return binary::parse(msg),
                _ => {}
            }
        }
        //  No binary sentinel, try text format.
        let msgstring = std::str::from_utf8(msg)?; // convert to UTF-8 string
        if msgstring.trim_start().starts_with(xml::LLSDXMLSENTINEL) { // try XML
            return xml::parse(msgstring) }
        //  ***NEED TO RECOGNIZE BINARY WITHOUT HEADER***
        //  "Notation" syntax is not currently supported. 
        //  Trim sring to N chars for error msg.
        let snippet = msgstring.chars().zip(0..60).map(|(c,_)| c).collect::<String>();
        Err(anyhow!("LLSD format not recognized: {:?}", snippet))
    }
}

//  Unit tests

#[test]
fn testllsdvalue() {
    //  Convert an LLSD value through all serializations and back again.
    //  Construct a test value. Use only floats with exact binary representations.
    let test1map: HashMap<String, LLSDValue> = [
        ("val1".to_string(), LLSDValue::Real(456.0)),
        ("val2".to_string(), LLSDValue::Integer(999)),
    ]
    .iter()
    .cloned()
    .collect();
    let test1: LLSDValue = LLSDValue::Array(vec![
        LLSDValue::Real(123.5),
        LLSDValue::Integer(42),
        LLSDValue::Map(test1map),
        LLSDValue::String("Hello world".to_string()),
    ]);
    //  Convert to binary form.
    let test1bin = binary::to_bytes(&test1).unwrap();
    //  Convert back to value form.
    let test1value = LLSDValue::parse(&test1bin).unwrap();
    println!("Value after round-trip conversion: {:?}", test1value);
    //  Check that results match after round trip.
    assert_eq!(test1, test1value);
    //  Convert to XML
    let test2xml = xml::to_xml_string(&test1value, true).unwrap();
    println!("As XML:\n{}", test2xml);
    let test2value = LLSDValue::parse(test2xml.as_bytes()).unwrap();
    assert_eq!(test1, test2value);
    //  Test error cases
    match LLSDValue::parse(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<llsd><complex>2i</complex></llsd>") {
        Err(e) => println!("Error as expected: {:?}",e),
        Ok(val) => panic!("Bad input not detected: {:?}", val)
    }
    match LLSDValue::parse("String ending in emoji to check Unicode truncation. 😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀😀".as_bytes()) {
        Err(e) => println!("Error as expected: {:?}",e),
        Ok(val) => panic!("Bad input not detected: {:?}", val)
    }
}
