use serde::{Deserialize, Serialize};

pub const MESSAGES_NS_URI: &str = "http://schemas.microsoft.com/exchange/services/2006/messages";
pub const SOAP_NS_URI: &str = "http://schemas.xmlsoap.org/soap/envelope/";
pub const TYPES_NS_URI: &str = "http://schemas.microsoft.com/exchange/services/2006/types";

pub trait EwsWrite<W> {
    /// Writes the struct as XML using the provided writer.
    fn write(&self, writer: &mut quick_xml::writer::Writer<W>) -> Result<(), quick_xml::Error>;
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SoapEnvelope {
    pub body: SoapBody,
}

#[derive(Deserialize)]
pub struct SoapBody {
    #[serde(rename = "$value")]
    pub contents: Response,
}

#[derive(Deserialize)]
pub enum Response {
    // Placeholder to demonstrate matching.
    ExportItemsResponse(String),

    FindItemResponse(FindItemResponse),
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Mailbox;

/// An identifier for a remote folder.
pub enum FolderId {
    /// An identifier for an arbitrary folder.
    ///
    /// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/folderid>.
    FolderId {
        id: String,
        change_key: Option<String>,
    },

    /// An identifier for referencing a folder by name, e.g. "inbox" or
    /// "junkemail".
    ///
    /// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/distinguishedfolderid>.
    DistinguishedFolderId {
        // This should probably be an enum, but this is a proof of concept and
        // I'm not writing all of those out right now.
        id: String,
        change_key: Option<String>,
        mailbox: Option<Mailbox>,
    },
}

impl<W: std::io::Write> EwsWrite<W> for FolderId {
    fn write(&self, writer: &mut quick_xml::writer::Writer<W>) -> Result<(), quick_xml::Error> {
        match self {
            FolderId::FolderId { .. } => todo!(),
            FolderId::DistinguishedFolderId { id, change_key, .. } => {
                let mut attrs = vec![("Id", id.as_str())];
                if let Some(change_key) = change_key {
                    attrs.push(("ChangeKey", change_key));
                }

                writer
                    .create_element("t:DistinguishedFolderId")
                    .with_attributes(attrs)
                    .write_empty()?;

                Ok(())
            }
        }
    }
}

/// The base set of properties to be returned in response to our request, which
/// can be modified by the parent.
///
/// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/baseshape>.
pub enum BaseShape {
    IdOnly,
    Default,
    AllProperties,
}

impl<W: std::io::Write> EwsWrite<W> for BaseShape {
    fn write(&self, writer: &mut quick_xml::writer::Writer<W>) -> Result<(), quick_xml::Error> {
        let value = match self {
            BaseShape::IdOnly => "IdOnly",
            BaseShape::Default => "Default",
            BaseShape::AllProperties => "AllProperties",
        };

        writer
            .create_element("t:BaseShape")
            .write_text_content(quick_xml::events::BytesText::new(value))?;

        Ok(())
    }
}

/// The folder properties to include in the response.
///
/// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/foldershape>.
pub struct FolderShape {
    pub base_shape: BaseShape,
}

impl<W: std::io::Write> EwsWrite<W> for FolderShape {
    fn write(&self, writer: &mut quick_xml::writer::Writer<W>) -> Result<(), quick_xml::Error> {
        writer
            .create_element("FolderShape")
            .write_inner_content::<_, quick_xml::Error>(|writer| {
                self.base_shape.write(writer)?;

                Ok(())
            })?;

        Ok(())
    }
}

pub struct ItemShape {
    pub base_shape: BaseShape,
}

impl<W: std::io::Write> EwsWrite<W> for ItemShape {
    fn write(&self, writer: &mut quick_xml::writer::Writer<W>) -> Result<(), quick_xml::Error> {
        writer
            .create_element("ItemShape")
            .write_inner_content(|writer| self.base_shape.write(writer))?;

        Ok(())
    }
}

#[derive(Clone, Copy)]
pub enum Traversal {
    Shallow,
    SoftDeleted,
    Associated,
}

impl From<Traversal> for &str {
    fn from(value: Traversal) -> Self {
        match value {
            Traversal::Shallow => "Shallow",
            Traversal::SoftDeleted => "SoftDeleted",
            Traversal::Associated => "Associated",
        }
    }
}

/// A request to list any items matching provided filters. I didn't add support
/// for filters.
///
/// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/finditem>.
pub struct FindItem {
    /// The manner in which to traverse nested folders.
    traversal: Traversal,

    /// The desired properties to include in the response.
    item_shape: ItemShape,

    /// Identifiers for the folders in which to locate items.
    parent_folder_ids: Vec<FolderId>,
}

impl FindItem {
    /// Creates a new FindItem request object.
    pub fn new(
        traversal: Traversal,
        item_shape: ItemShape,
        parent_folder_ids: Vec<FolderId>,
    ) -> Self {
        Self {
            traversal,
            item_shape,
            parent_folder_ids,
        }
    }
}

impl<W: std::io::Write> EwsWrite<W> for FindItem {
    fn write(&self, writer: &mut quick_xml::writer::Writer<W>) -> Result<(), quick_xml::Error> {
        writer
            .create_element("FindItem")
            .with_attributes([
                ("xmlns", MESSAGES_NS_URI),
                ("xmlns:t", TYPES_NS_URI),
                ("Traversal", self.traversal.into()),
            ])
            .write_inner_content::<_, quick_xml::Error>(|writer| {
                self.item_shape.write(writer)?;

                writer
                    .create_element("ParentFolderIds")
                    .write_inner_content::<_, quick_xml::Error>(|writer| {
                        for id in &self.parent_folder_ids {
                            id.write(writer)?;
                        }

                        Ok(())
                    })?;

                Ok(())
            })?;

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ItemId {
    #[serde(rename = "@Id")]
    id: String,

    #[serde(rename = "@ChangeKey")]
    change_key: String,
}

impl ItemId {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn change_key(&self) -> &str {
        &self.change_key
    }
}

/// An email message.
///
/// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/message-ex15websvcsotherref>.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Message {
    item_id: ItemId,
    subject: String,
}

impl Message {
    pub fn item_id(&self) -> &ItemId {
        &self.item_id
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }
}

/// The response to a [`FindItem`] request.
///
/// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/finditemresponse>.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FindItemResponse {
    response_messages: ResponseMessages,
}

#[derive(Deserialize, Serialize)]
pub struct ResponseMessages {
    #[serde(rename = "$value")]
    contents: Vec<ResponseMessageContents>,
}

#[derive(Deserialize, Serialize)]
pub enum ResponseMessageContents {
    FindItemResponseMessage(FindItemResponseMessage),

    // Placeholder just to demonstrate matching.
    GetRemindersResponse(String),
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FindItemResponseMessage {
    root_folder: RootFolder,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RootFolder {
    items: Items,
}

#[derive(Deserialize, Serialize)]
pub struct Items {
    #[serde(rename = "$value")]
    items: Vec<EwsItem>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum EwsItem {
    Message(Message),
}

impl FindItemResponse {
    pub fn messages(&self) -> Vec<&Message> {
        self.response_messages
            .contents
            .iter()
            .filter_map(|message| match message {
                ResponseMessageContents::FindItemResponseMessage(message) => Some(message),
                _ => None,
            })
            .next()
            .unwrap()
            .root_folder
            .items
            .items
            .iter()
            .filter_map(|item| match item {
                EwsItem::Message(message) => Some(message),
            })
            .collect()
    }
}

pub struct GetFolder {
    pub folder_ids: Vec<FolderId>,
    pub folder_shape: FolderShape,
}
