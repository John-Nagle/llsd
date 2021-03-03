//
//  Library for serializing and de-serializing data in
//  Linden Lab Structured Data format.
//
//  Binary format.
//
//  Animats
//  February, 2021.
//  License: LGPL.
//
use super::LLSDValue;
use anyhow::{anyhow, Error};
use std::collections::HashMap;
use std::io::Write;
use uuid;
//
//  Constants
//
const LLSDBINARYPREFIX: &[u8] = b"<? LLSD/Binary ?>\n";            // binary LLSD prefix
/*
///    Parse LLSD expressed in XML into an LLSD tree.
pub fn parse(xmlstr: &str) -> Result<LLSDValue, Error> {
    let mut reader = Reader::from_str(xmlstr);
    reader.trim_text(true); // do not want trailing blanks
    reader.expand_empty_elements(true); // want end tag events always
    let mut buf = Vec::new();
    let mut output: Option<LLSDValue> = None;
    //  Outer parse. Find <llsd> and parse its interior.
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"llsd" => {
                        if output.is_some() {
                            return Err(anyhow!("More than one <llsd> block in data"));
                        }
                        let mut buf = Vec::new();
                        match reader.read_event(&mut buf) {
                            Ok(Event::Start(ref e)) => {
                                let tagname = std::str::from_utf8(e.name())?; // tag name as string to start parse
                                output = Some(parse_value(&mut reader, tagname, &e.attributes())?);
                                // parse next value
                            }
                            _ => {
                                return Err(anyhow!(
                                    "Expected LLSD data, found {:?} error at position {}",
                                    e.name(),
                                    reader.buffer_position()
                                ))
                            }
                        };
                    }
                    _ => {
                        return Err(anyhow!(
                            "Expected <llsd>, found {:?} error at position {}",
                            e.name(),
                            reader.buffer_position()
                        ))
                    }
                }
            }
            Ok(Event::Text(_e)) => (), // Don't actually need random text
            Ok(Event::End(ref _e)) => (), // Tag matching check is automatic.
            Ok(Event::Eof) => break,   // exits the loop when reaching end of file
            Err(e) => {
                return Err(anyhow!(
                    "Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ))
            }
            _ => (), // There are several other `Event`s we do not consider here
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear()
    }
    //  Final result, if stored
    match output {
        Some(out) => Ok(out),
        None => Err(anyhow!("Unexpected end of data, no <llsd> block.")),
    }
}

/// Parse one value - real, integer, map, etc. Recursive.
fn parse_value(
    reader: &mut Reader<&[u8]>,
    starttag: &str,
    attrs: &Attributes,
) -> Result<LLSDValue, Error> {
    //  Entered with a start tag alread parsed and in starttag
    match starttag {
        "undef" | "real" | "integer" | "boolean" | "string" | "uri" | "binary" | "uuid"
        | "date" => parse_primitive_value(reader, starttag, attrs),
        "map" => parse_map(reader),
        "array" => parse_array(reader),
        _ => Err(anyhow!(
            "Unknown data type <{}> at position {}",
            starttag,
            reader.buffer_position()
        )),
    }
}

/// Parse one value - real, integer, map, etc. Recursive.
fn parse_primitive_value(
    reader: &mut Reader<&[u8]>,
    starttag: &str,
    attrs: &Attributes,
) -> Result<LLSDValue, Error> {
    //  Entered with a start tag already parsed and in starttag
    let mut texts = Vec::new(); // accumulate text here
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Text(e)) => texts.push(e.unescape_and_decode(&reader)?),
            Ok(Event::End(ref e)) => {
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                if starttag != tagname {
                    return Err(anyhow!(
                        "Unmatched XML tags: <{}> .. <{}>",
                        starttag,
                        tagname
                    ));
                };
                //  End of an XML tag. Value is in text.
                let text = texts.join(" ").trim().to_string(); // combine into one big string
                texts.clear();
                //  Parse the primitive types.
                return match starttag {
                    "undef" => Ok(LLSDValue::Undefined),
                    "real" => Ok(LLSDValue::Real(
                        if text.to_lowercase() == "nan" {
                            "NaN".to_string()
                        } else {
                            text
                        }
                        .parse::<f64>()?,
                    )),
                    "integer" => Ok(LLSDValue::Integer(text.parse::<i32>()?)),
                    "boolean" => Ok(LLSDValue::Boolean(parse_boolean(&text)?)),
                    "string" => Ok(LLSDValue::String(text.to_string())),
                    "uri" => Ok(LLSDValue::String(text.to_string())),
                    "uuid" => Ok(LLSDValue::UUID(if text.is_empty() {
                        uuid::Uuid::nil()
                    } else {
                        uuid::Uuid::parse_str(&text)?
                    })),
                    "date" => Ok(LLSDValue::Date(parse_date(&text)?)),
                    "binary" => Ok(LLSDValue::Binary(parse_binary(&text, attrs)?)),
                    _ => Err(anyhow!(
                        "Unexpected primitive data type <{}> at position {}",
                        starttag,
                        reader.buffer_position()
                    )),
                };
            }
            Ok(Event::Eof) => {
                return Err(anyhow!(
                    "Unexpected end of data in primitive value at position {}",
                    reader.buffer_position()
                ))
            }
            Ok(Event::Comment(_)) => {} // ignore comment
            Err(e) => {
                return Err(anyhow!(
                    "Parse Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ))
            }
            _ => {
                return Err(anyhow!(
                    "Unexpected value parse error at position {} while parsing: {:?}",
                    reader.buffer_position(),
                    starttag
                ))
            }
        }
    }
}

//  Parse one map.
fn parse_map(reader: &mut Reader<&[u8]>) -> Result<LLSDValue, Error> {
    //  Entered with a "map" start tag just parsed.
    let mut map: HashMap<String, LLSDValue> = HashMap::new(); // accumulating map
    let mut texts = Vec::new(); // accumulate text here
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                match tagname {
                    "key" => {
                        let (k, v) = parse_map_entry(reader)?; // read one key/value pair
                        let _dup = map.insert(k, v); // insert into map
                                                     //  Duplicates are not errors, per LLSD spec.
                    }
                    _ => {
                        return Err(anyhow!("Expected 'key' in map, found '{}'", tagname));
                    }
                }
            }
            Ok(Event::Text(e)) => texts.push(e.unescape_and_decode(&reader)?),
            Ok(Event::End(ref e)) => {
                //  End of an XML tag. No text expected.
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                if "map" != tagname {
                    return Err(anyhow!("Unmatched XML tags: <{}> .. <{}>", "map", tagname));
                };
                return Ok(LLSDValue::Map(map)); // done, valid result
            }
            Ok(Event::Eof) => {
                return Err(anyhow!(
                    "Unexpected end of data in map at position {}",
                    reader.buffer_position()
                ))
            }
            Ok(Event::Comment(_)) => {} // ignore comment
            Err(e) => {
                return Err(anyhow!(
                    "Parse Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ))
            }
            _ => {
                return Err(anyhow!(
                    "Unexpected parse error at position {} while parsing a map",
                    reader.buffer_position()
                ))
            }
        }
    }
}

//  Parse one map entry.
//  Format <key> STRING> </key> LLSDVALUE
fn parse_map_entry(reader: &mut Reader<&[u8]>) -> Result<(String, LLSDValue), Error> {
    //  Entered with a "key" start tag just parsed.  Expecting text.
    let mut texts = Vec::new(); // accumulate text here
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                return Err(anyhow!("Expected 'key' in map, found '{}'", tagname));
            }
            Ok(Event::Text(e)) => texts.push(e.unescape_and_decode(&reader)?),
            Ok(Event::End(ref e)) => {
                //  End of an XML tag. Should be </key>
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                if "key" != tagname {
                    return Err(anyhow!("Unmatched XML tags: <{}> .. <{}>", "key", tagname));
                };
                let mut buf = Vec::new();
                let k = texts.join(" ").trim().to_string(); // the key
                match reader.read_event(&mut buf) {
                    Ok(Event::Start(ref e)) => {
                        let tagname = std::str::from_utf8(e.name())?; // tag name as string
                        let v = parse_value(reader, tagname, &e.attributes())?; // parse next value
                        return Ok((k, v)); // return key value pair
                    }
                    _ => {
                        return Err(anyhow!(
                            "Unexpected parse error at position {} while parsing map entry",
                            reader.buffer_position()
                        ))
                    }
                };
            }
            Ok(Event::Eof) => {
                return Err(anyhow!(
                    "Unexpected end of data at position {}",
                    reader.buffer_position()
                ))
            }
            Ok(Event::Comment(_)) => {} // ignore comment
            Err(e) => {
                return Err(anyhow!(
                    "Parse Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ))
            }
            _ => {
                return Err(anyhow!(
                    "Unexpected parse error at position {} while parsing a map entry",
                    reader.buffer_position()
                ))
            }
        }
    }
}

/// Parse one LLSD object. Recursive.
fn parse_array(reader: &mut Reader<&[u8]>) -> Result<LLSDValue, Error> {
    //  Entered with an <array> tag just parsed.
    let mut texts = Vec::new(); // accumulate text here
    let mut buf = Vec::new();
    let mut items: Vec<LLSDValue> = Vec::new(); // accumulate items.
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                                                              //  Parse one data item.
                items.push(parse_primitive_value(reader, tagname, &e.attributes())?);
            }
            Ok(Event::Text(e)) => texts.push(e.unescape_and_decode(&reader)?),
            Ok(Event::End(ref e)) => {
                //  End of an XML tag. Should be </array>
                let tagname = std::str::from_utf8(e.name())?; // tag name as string
                if "array" != tagname {
                    return Err(anyhow!(
                        "Unmatched XML tags: <{}> .. <{}>",
                        "array",
                        tagname
                    ));
                };
                break; // end of array
            }
            Ok(Event::Eof) => {
                return Err(anyhow!(
                    "Unexpected end of data at position {}",
                    reader.buffer_position()
                ))
            }
            Ok(Event::Comment(_)) => {} // ignore comment
            Err(e) => {
                return Err(anyhow!(
                    "Parse Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                ))
            }
            _ => {
                return Err(anyhow!(
                    "Unexpected parse error at position {} while parsing a map entry",
                    reader.buffer_position()
                ))
            }
        }
    }
    Ok(LLSDValue::Array(items)) // result is array of items
}

/// Parse binary object.
/// Input in base64, base16, or base85.
fn parse_binary(s: &str, attrs: &Attributes) -> Result<Vec<u8>, Error> {
    // "Parsers must support base64 encoding. Parsers may support base16 and base85."
    let encoding = match get_attr(attrs, b"encoding")? {
        Some(enc) => enc,
        None => "base64".to_string(), // default
    };
    //  Decode appropriately.
    Ok(match encoding.as_str() {
        "base64" => base64::decode(s)?,
        "base16" => hex::decode(s)?,
        "base85" => match ascii85::decode(s) {
            Ok(v) => v,
            Err(e) => return Err(anyhow!("Base 85 decode error: {:?}", e)),
        },
        _ => {
            return Err(anyhow!(
                "Unknown encoding: <binary encoding=\"{}\">",
                encoding
            ))
        }
    })
}

/// Parse ISO 9660 date, simple form.
fn parse_date(s: &str) -> Result<i64, Error> {
    Ok(chrono::DateTime::parse_from_rfc3339(s)?.timestamp())
}

//  Parse boolean. LSL allows 0, false, 1, true.
fn parse_boolean(s: &str) -> Result<bool, Error> {
    Ok(match s {
        "0" => false,
        "1" => true,
        _ => s.parse::<bool>()?,
    })
}

/// Search for attribute in attribute list
fn get_attr<'a>(attrs: &'a Attributes, key: &[u8]) -> Result<Option<String>, Error> {
    //  Each step has a possible error, so it's hard to do this more cleanly.
    for attr in attrs.clone() {
        let a = attr?;
        if a.key != key {
            continue;
        } // not this one
        let v = a.unescaped_value()?;
        let sv = std::str::from_utf8(&v)?;
        return Ok(Some(sv.to_string()));
    }
    Ok(None)
}
*/
/// Pretty prints out the value as XML. Takes an argument that's
/// the number of spaces to indent new blocks.
pub fn to_bytes(val: &LLSDValue) -> Result<Vec::<u8>, Error> {
    let mut s: Vec<u8> = Vec::new();
    s.write(LLSDBINARYPREFIX)?;  // prefix
    generate_value(&mut s, val)?;
    s.flush()?;
    Ok(s)
}

/// Generate one <TYPE> VALUE </TYPE> output. VALUE is recursive.
fn generate_value(s: &mut Vec<u8>, val: &LLSDValue) -> Result<(), Error>{
    /*
    fn tag(s: &mut Vec<u8>, tag: &str, close: bool, indent: usize) {
        if indent > 0 {
            let _ = write!(*s, "{:1$}", " ", indent);
        };
        let _ = write!(*s, "<{}{}>\n", if close { "/" } else { "" }, tag);
    }
    fn tag_value(s: &mut Vec<u8>, tag: &str, text: &str, indent: usize) {
        if indent > 0 {
            let _ = write!(*s, "{:1$}", " ", indent);
        };
        let _ = write!(*s, "<{}>{}</{}>\n", tag, xml_escape(text), tag);
    }

    //  Use SL "nan", not Rust "NaN"
    fn f64_to_xml(v: f64) -> String {
        let ss = v.to_string();
        if ss == "NaN" {
            "nan".to_string()
        } else {
            ss
        }
    }
    */
    //  Emit XML for all possible types.
    match val {
        LLSDValue::Undefined => s.write(b"!")?,
        LLSDValue::Boolean(v) => s.write(if *v { b"1" } else { b"0"})?,
        LLSDValue::String(v) =>  { 
            s.write(b"s")?; s.write(&(v.len() as u32).to_le_bytes())?; s.write(&v.as_bytes())? },
        LLSDValue::URI(v) => { 
            s.write(b"l")?; s.write(&(v.len() as u32).to_le_bytes())?; s.write(v.as_bytes())? },
        LLSDValue::Integer(v) => { s.write(b"i")?; s.write(&v.to_le_bytes())? }
        LLSDValue::Real(v) => { s.write(b"r")?; s.write(&v.to_le_bytes())? }
        LLSDValue::UUID(v) => { s.write(b"u")?; s.write(v.as_bytes())? }
        LLSDValue::Binary(v) =>  { 
            s.write(b"b")?; s.write(&(v.len() as u32).to_le_bytes())?; s.write(v)? },
        LLSDValue::Date(v) => { s.write(b"d")?; s.write(&v.to_le_bytes())? }

        //  Map is { childcnt key value key value ... }
        LLSDValue::Map(v) => {
            //  Output count of key/value pairs
            s.write(b"{")?; s.write(&(v.len() as u32).to_le_bytes())?;
            //  Output key/value pairs
            for (key, value) in v {
                s.write(&(key.len() as u32).to_le_bytes())?; s.write(&key.as_bytes())?;
                generate_value(s, value)?;
            }
            s.write(b"}")?
        }
        //  Array is [ childcnt child child ... ]
        LLSDValue::Array(v) => {
            //  Output count of array entries
            s.write(b"[")?; s.write(&(v.len() as u32).to_le_bytes())?;
            //  Output array entries
            for value in v {
                generate_value(s, value)?;
            }
            s.write(b"]")?
        }
    };
    Ok(())
}

// Unit tests

#[test]
fn binaryparsetest1() {
}
