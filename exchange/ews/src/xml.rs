use xml::writer;

use crate::types::{Body, BodyContents, Envelope};

/// Writes a struct as the body of a SOAP request.
pub fn write_request<W: std::io::Write>(sink: W, body: BodyContents) -> Result<(), writer::Error> {
    let mut writer = xml::EventWriter::new(sink);

    let request_body = Envelope {
        body: Body { contents: body },
    };
    request_body.write_as_element(&mut writer)
}

pub(crate) trait XmlElement {
    /// Writes the component as an XML component.
    fn write_as_element<W: std::io::Write>(
        &self,
        writer: &mut xml::EventWriter<W>,
    ) -> Result<(), xml::writer::Error>;
}

impl XmlElement for String {
    fn write_as_element<W: std::io::Write>(
        &self,
        writer: &mut xml::EventWriter<W>,
    ) -> Result<(), xml::writer::Error> {
        writer.write(xml::writer::events::XmlEvent::characters(self))
    }
}

impl<T> XmlElement for Option<T>
where
    T: XmlElement,
{
    fn write_as_element<W: std::io::Write>(
        &self,
        writer: &mut xml::EventWriter<W>,
    ) -> Result<(), xml::writer::Error> {
        match self {
            Some(component) => component.write_as_element(writer),
            None => Ok(()),
        }
    }
}

impl<T> XmlElement for Vec<T>
where
    T: XmlElement,
{
    fn write_as_element<W: std::io::Write>(
        &self,
        writer: &mut xml::EventWriter<W>,
    ) -> Result<(), xml::writer::Error> {
        self.iter()
            .map(|component| component.write_as_element(writer))
            .collect()
    }
}

pub(crate) trait XmlAttribute {
    fn write_as_attribute<'a>(
        &'a self,
        builder: xml::writer::events::StartElementBuilder<'a>,
        attr_name: &'a str,
    ) -> xml::writer::events::StartElementBuilder<'a>;
}

impl XmlAttribute for Option<String> {
    fn write_as_attribute<'a>(
        &'a self,
        builder: xml::writer::events::StartElementBuilder<'a>,
        attr_name: &'a str,
    ) -> xml::writer::events::StartElementBuilder<'a> {
        match self {
            Some(value) => builder.attr(attr_name, value),
            None => builder,
        }
    }
}

impl XmlAttribute for String {
    fn write_as_attribute<'a>(
        &'a self,
        builder: xml::writer::events::StartElementBuilder<'a>,
        attr_name: &'a str,
    ) -> xml::writer::events::StartElementBuilder<'a> {
        builder.attr(attr_name, self)
    }
}

pub(crate) fn verify_attribute_field<T: XmlAttribute>(_: &T) {}
pub(crate) fn verify_element_field<T: XmlElement>(_: &T) {}
