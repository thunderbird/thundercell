use xml::{attribute::OwnedAttribute, reader, writer, EventReader};

use crate::types::{EwsWrite, SOAP_NS_URI, TYPES_NS_URI};

/// Writes a struct as the body of a SOAP request.
pub fn write_request<W: std::io::Write, X: EwsWrite<W>>(
    sink: W,
    body: X,
) -> Result<(), writer::Error> {
    let mut writer = xml::EventWriter::new(sink);

    writer.write(
        xml::writer::XmlEvent::start_element("soap:Envelope")
            .ns("soap", SOAP_NS_URI)
            .ns("t", TYPES_NS_URI),
    )?;
    writer.write(xml::writer::XmlEvent::start_element("soap:Body"))?;

    body.write(&mut writer)?;

    writer.write(xml::writer::XmlEvent::end_element())?;
    writer.write(xml::writer::XmlEvent::end_element())
}

/// Skips ahead until it finds a matching element and returns its attributes.
pub fn read_element_start<R: std::io::Read>(
    reader: &mut EventReader<R>,
    tag: &str,
) -> Result<Vec<OwnedAttribute>, reader::Error> {
    loop {
        let event = reader.next()?;
        match event {
            xml::reader::XmlEvent::EndDocument => {
                panic!("Unexpected end of document looking for {tag}")
            }
            xml::reader::XmlEvent::StartElement {
                name, attributes, ..
            } => {
                if name.local_name == tag {
                    return Ok(attributes);
                }
            }
            _ => continue,
        }
    }
}

/// Skips ahead until it finds a matching element close tag.
pub fn read_element_end<R: std::io::Read>(
    reader: &mut EventReader<R>,
    tag: &str,
) -> Result<(), reader::Error> {
    loop {
        match reader.next()? {
            xml::reader::XmlEvent::EndDocument => {
                panic!("Unexpected end of document looking for {tag}")
            }
            xml::reader::XmlEvent::EndElement { name, .. } => {
                if name.local_name == tag {
                    break;
                }
            }
            _ => continue,
        }
    }

    Ok(())
}
