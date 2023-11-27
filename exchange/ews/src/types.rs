use ews_derive::{XmlAttribute, XmlElement};
use serde::Deserialize;

pub const MESSAGES_NS_URI: &str = "http://schemas.microsoft.com/exchange/services/2006/messages";
pub const SOAP_NS_URI: &str = "http://schemas.xmlsoap.org/soap/envelope/";
pub const TYPES_NS_URI: &str = "http://schemas.microsoft.com/exchange/services/2006/types";

#[derive(Deserialize, XmlElement)]
#[serde(rename_all = "PascalCase")]
#[xml_serialize(ns = ("soap", SOAP_NS_URI), ns = ("t", TYPES_NS_URI), ns_prefix = "soap")]
pub struct Envelope {
    pub body: Body,
}

#[derive(Deserialize, XmlElement)]
#[xml_serialize(ns_prefix = "soap")]
pub struct Body {
    #[serde(rename = "$value")]
    pub contents: BodyContents,
}

#[derive(Deserialize, XmlElement)]
pub enum BodyContents {
    FindItem(FindItem),
    FindItemResponse(FindItemResponse),
}

#[derive(Debug, Deserialize, XmlElement)]
#[serde(rename_all = "PascalCase")]
pub struct Mailbox;

/// An identifier for a remote folder.
#[derive(Debug, Deserialize, XmlElement)]
#[xml_serialize(ns_prefix = "t")]
pub enum FolderId {
    /// An identifier for an arbitrary folder.
    ///
    /// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/folderid>.
    FolderId {
        #[xml_serialize(attribute)]
        id: String,

        #[xml_serialize(attribute)]
        change_key: Option<String>,
    },

    /// An identifier for referencing a folder by name, e.g. "inbox" or
    /// "junkemail".
    ///
    /// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/distinguishedfolderid>.
    DistinguishedFolderId {
        // This should probably be an enum, but this is a proof of concept and
        // I'm not writing all of those out right now.
        #[xml_serialize(attribute)]
        id: String,

        #[xml_serialize(attribute)]
        change_key: Option<String>,

        mailbox: Option<Mailbox>,
    },
}

/// The base set of properties to be returned in response to our request, which
/// can be modified by the parent.
///
/// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/baseshape>.
#[derive(Debug, Deserialize, XmlElement)]
#[xml_serialize(ns_prefix = "t")]
pub enum BaseShape {
    IdOnly,
    Default,
    AllProperties,
}

/// The folder properties to include in the response.
///
/// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/foldershape>.
pub struct FolderShape {
    pub base_shape: BaseShape,
}

#[derive(Debug, Deserialize, XmlElement)]
pub struct ItemShape {
    pub base_shape: BaseShape,
}

#[derive(Clone, Copy, Debug, Deserialize, XmlAttribute)]
pub enum Traversal {
    Shallow,
    SoftDeleted,
    Associated,
}

/// A request to list any items matching provided filters. I didn't add support
/// for filters.
///
/// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/finditem>.
#[derive(Debug, Deserialize, XmlElement)]
#[xml_serialize(default_ns = MESSAGES_NS_URI, ns = ("t", TYPES_NS_URI))]
pub struct FindItem {
    /// The manner in which to traverse nested folders.
    #[xml_serialize(attribute)]
    traversal: Traversal,

    /// The desired properties to include in the response.
    item_shape: ItemShape,

    /// Identifiers for the folders in which to locate items.
    parent_folder_ids: ParentFolderIds,
}

#[derive(Debug, Deserialize, XmlElement)]
pub struct ParentFolderIds(Vec<FolderId>);

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
            parent_folder_ids: ParentFolderIds(parent_folder_ids),
        }
    }
}

#[derive(Debug, Deserialize, XmlElement)]
#[serde(rename_all = "PascalCase")]
pub struct ItemId {
    id: String,
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
#[derive(Debug, Deserialize, XmlElement)]
#[serde(rename_all = "PascalCase")]
pub struct Message {
    item_id: ItemId,
    subject: Subject,
}

#[derive(Debug, Deserialize, XmlElement)]
pub struct Subject(String);

impl Message {
    pub fn item_id(&self) -> &ItemId {
        &self.item_id
    }

    pub fn subject(&self) -> &str {
        &self.subject.0
    }
}

/// The response to a [`FindItem`] request.
///
/// See <https://learn.microsoft.com/en-us/exchange/client-developer/web-service-reference/finditemresponse>.
#[derive(Deserialize, XmlElement)]
#[serde(rename_all = "PascalCase")]
pub struct FindItemResponse {
    response_messages: ResponseMessages,
}

#[derive(Deserialize, XmlElement)]
pub struct ResponseMessages {
    #[serde(rename = "$value")]
    contents: Vec<ResponseMessageContents>,
}

#[derive(Deserialize, XmlElement)]
pub enum ResponseMessageContents {
    FindItemResponseMessage(FindItemResponseMessage),
}

#[derive(Deserialize, XmlElement)]
#[serde(rename_all = "PascalCase")]
pub struct FindItemResponseMessage {
    root_folder: RootFolder,
}

#[derive(Deserialize, XmlElement)]
#[serde(rename_all = "PascalCase")]
pub struct RootFolder {
    items: Items,
}

#[derive(Deserialize, XmlElement)]
pub struct Items {
    #[serde(rename = "$value")]
    items: Vec<EwsItem>,
}

#[derive(Deserialize, XmlElement)]
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
