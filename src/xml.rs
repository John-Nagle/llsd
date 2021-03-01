//
//  Library for serializing and de-serializing data in
//  Linden Lab Structured Data format.
//
//  XML format.
//
//  Animats
//  February, 2021.
//  License: LGPL.
//
use super::{LLSDValue, LLSDObject};
use anyhow::{anyhow, Error};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::{BufReader};

///    Parse LLSD expressed in XML into an LLSD tree.
pub fn parse(xmlstr: &str) -> Result<LLSDObject, Error> {
    let mut reader = Reader::from_str(xmlstr);
    reader.trim_text(true); // do not want trailing blanks

    let mut count = 0;
    let mut txt = Vec::new();
    let mut buf = Vec::new();

    // The `Reader` does not implement `Iterator` because it outputs borrowed data (`Cow`s)
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"tag1" => println!(
                        "attributes values: {:?}",
                        e.attributes().map(|a| a.unwrap().value).collect::<Vec<_>>()
                    ),
                    b"tag2" => count += 1,
                    _ => {
                        println!(
                            "<{:?}> attributes values: {:?} text: {:?}",
                            std::str::from_utf8(e.name()),
                            e.attributes().map(|a| a.unwrap().value).collect::<Vec<_>>(),
                            txt
                        ); // ***TEMP***
                        txt.clear();
                    }
                }
            }
            Ok(Event::Text(e)) => txt.push(e.unescape_and_decode(&reader).unwrap()),
            Ok(Event::End(ref e)) => println!("End <{:?}>", std::str::from_utf8(e.name())),
            Ok(Event::Eof) => break, // exits the loop when reaching end of file
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (), // There are several other `Event`s we do not consider here
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear()
    }
    return Err(anyhow!("Unimplemented"));
}

/// Parse one primitive value - real, integer, etc.
fn parse_primitive(reader: &mut Reader<BufReader<&[u8]>>, starttag: &str) -> Result<LLSDValue, Error> {
    let mut texts = Vec::new();                           // accumulate text here
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Text(e)) => texts.push(e.unescape_and_decode(&reader).unwrap()),
            Ok(Event::End(ref e)) => {
                println!("End <{:?}>", std::str::from_utf8(e.name()));
                //  End of an XML tag. Value is in text.
                let text = texts.join(" ");                 // combine into one big string
                return match starttag {
                    "real" => Ok(LLSDValue::Real(text.parse::<f64>()?)),
                    "integer" => Ok(LLSDValue::Integer(text.parse::<i32>()?)),
                    "bool" => Ok(LLSDValue::Boolean(text.parse::<bool>()?)),
                    _ => Err(anyhow!("Unexpected data type at position {}: {:?}", reader.buffer_position(), e)),
                }
            },
            Ok(Event::Eof) => return Err(anyhow!("Unexpected end of data at position {}", reader.buffer_position())),
            Err(e) => return Err(anyhow!("Parse Error at position {}: {:?}", reader.buffer_position(), e)),
            _ => return Err(anyhow!("Unexpected parse error at position {} while parsing: {:?}", reader.buffer_position(), starttag)),
        }
    }
}

/// Prints out the value as an XML string.
pub fn dump(val: &LLSDValue) -> String {
    return "Unimplemented".to_string();
}

/// Pretty prints out the value as XML. Takes an argument that's
/// the number of spaces to indent new blocks.
pub fn pretty(val: &LLSDValue, spaces: u16) -> String {
    return "Unimplemented".to_string();
}

// Unit tests

#[test]
fn xmlparsetest1() {
    const TESTXML1: &str = r#""
<?xml version="1.0" encoding="UTF-8"?>
<llsd>
<map>
  <key>region_id</key>
    <uuid>67153d5b-3659-afb4-8510-adda2c034649</uuid>
  <key>scale</key>
    <string>one minute</string>
  <key>simulator statistics</key>
  <map>
    <key>time dilation</key><real>0.9878624</real>
    <key>sim fps</key><real>44.38898</real>
    <key>pysics fps</key><real>44.38906</real>
    <key>agent updates per second</key><real>nan</real>
    <key>lsl instructions per second</key><real>0</real>
    <key>total task count</key><real>4</real>
    <key>active task count</key><real>0</real>
    <key>active script count</key><real>4</real>
    <key>main agent count</key><real>0</real>
    <key>child agent count</key><real>0</real>
    <key>inbound packets per second</key><real>1.228283</real>
    <key>outbound packets per second</key><real>1.277508</real>
    <key>pending downloads</key><real>0</real>
    <key>pending uploads</key><real>0.0001096525</real>
    <key>frame ms</key><real>0.7757886</real>
    <key>net ms</key><real>0.3152919</real>
    <key>sim other ms</key><real>0.1826937</real>
    <key>sim physics ms</key><real>0.04323055</real>
    <key>agent ms</key><real>0.01599029</real>
    <key>image ms</key><real>0.01865955</real>
    <key>script ms</key><real>0.1338836</real>
  </map>
</map>
</llsd>
""#;

    let result = parse(TESTXML1);
    println!("Parse of {:?}: \n{:#?}", TESTXML1, result);
}
