use std::borrow::Cow;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str;
use ::tag::Version;

pub use self::content::{Content, ExtendedText, ExtendedLink, Comment, Lyrics, Picture, PictureType};
pub use self::timestamp::Timestamp;

mod content;
mod timestamp;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ID {
    /// A valid 4-byte frame ID.
    Valid(String),
    /// If an ID3v2.2 ID could not be mapped to its ID3v2.4 counterpart, it is stored as is. This
    /// allows invalid ID3v2.2 frames to be retained.
    Invalid(String),
}

/// A structure representing an ID3 frame.
///
/// It is imporant to note that the (Partial)Eq and Hash implementations are based on the ID3 spec.
/// This means that text frames with equal ID's are equal but picture frames with both "APIC" as ID
/// are not because their uniqueness is also defined by their content.
#[derive(Clone, Debug, Eq)]
pub struct Frame {
    id: ID,
    content: Content,
    tag_alter_preservation: bool,
    file_alter_preservation: bool,
}

impl PartialEq for Frame {
    fn eq(&self, other: &Frame) -> bool {
        match self.content {
            Content::Text(_) => self.id == other.id,
            _ => {
                self.id == other.id && self.content == other.content
            },
        }
    }
}

impl Hash for Frame {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        match self.content {
            Content::Text(_) => self.id.hash(state),
            _ => {
                self.id.hash(state);
                self.content.hash(state);
            },
        }
    }
}

impl Frame {
    /// Creates a new ID3v2.3 frame with the specified identifier.
    ///
    /// # Panics
    /// If the id's length is not 3 or 4 bytes long or not known.
    #[deprecated(note = "Use with_content")]
    pub fn new<T: Into<String>>(id: T) -> Frame {
        Frame::with_content(&id.into(), Content::Unknown(Vec::new()))
    }

    /// Creates a frame with the specified ID and content.
    ///
    /// Both ID3v2.2 and >ID3v2.3 IDs are accepted, although they will be converted to ID3v2.3
    /// format. If an ID3v2.2 ID is supplied but could not be remapped, it is stored as-is.
    ///
    /// # Panics
    /// If the id's length is not 3 or 4 bytes long.
    pub fn with_content(id: &str, content: Content) -> Frame {
        assert!({
            let l = id.bytes().count();
            l == 3 || l == 4
        });
        Frame {
            id: if id.len() == 3 {
                match ::util::convert_id_2_to_3(id) {
                    Some(translated) => {
                        ID::Valid(translated.to_string())
                    },
                    None => ID::Invalid(id.to_string()),
                }
            } else {
                ID::Valid(id.to_string())
            },
            content: content,
            tag_alter_preservation: false,
            file_alter_preservation: false,
        }
    }

    /// Returns the ID of this frame.
    ///
    /// The string returned us usually 4 bytes long except when the frame was read from an ID3v2.2
    /// tag and the ID could not be mapped to an ID3v2.3 ID.
    pub fn id(&self) -> &str {
        match self.id {
              ID::Valid(ref id)
            | ID::Invalid(ref id) => id,
        }
    }

    /// Returns the ID that is compatible with specified version or None if no ID is available in
    /// that version.
    pub fn id_for_version(&self, version: Version) -> Option<&str> {
        match (version, &self.id) {
            (Version::Id3v22, &ID::Valid(ref id)) => ::util::convert_id_3_to_2(id),
            (Version::Id3v23, &ID::Valid(ref id))
            | (Version::Id3v24, &ID::Valid(ref id))
            | (Version::Id3v22, &ID::Invalid(ref id)) => Some(id),
            (_, &ID::Invalid(_)) => None,
        }
    }

    /// Returns the content of the frame.
    pub fn content(&self) -> &Content {
        &self.content
    }

    /// Returns whether the tag_alter_preservation flag is set.
    pub fn tag_alter_preservation(&self) -> bool {
        self.tag_alter_preservation
    }

    /// Sets the tag_alter_preservation flag.
    pub fn set_tag_alter_preservation(&mut self, tag_alter_preservation: bool) {
        self.tag_alter_preservation = tag_alter_preservation;
    }

    /// Returns whether the file_alter_preservation flag is set.
    pub fn file_alter_preservation(&self) -> bool {
        self.file_alter_preservation
    }

    /// Sets the file_alter_preservation flag.
    pub fn set_file_alter_preservation(&mut self, file_alter_preservation: bool) {
        self.file_alter_preservation = file_alter_preservation;
    }

    /// Returns a string representing the parsed content.
    ///
    /// Returns `None` if the parsed content can not be represented as text.
    ///
    /// # Example
    /// ```
    /// use id3::frame::{self, Frame, Content};
    ///
    /// let title_frame = Frame::with_content("TIT2", Content::Text("title".to_owned()));
    /// assert_eq!(&title_frame.text().unwrap()[..], "title");
    ///
    /// let mut txxx_frame = Frame::with_content("TXXX", Content::ExtendedText(frame::ExtendedText {
    ///     description: "description".to_owned(),
    ///     value: "value".to_owned()
    /// }));
    /// assert_eq!(&txxx_frame.text().unwrap()[..], "description: value");
    /// ```
    #[deprecated(note = "Format using fmt::Display")]
    pub fn text(&self) -> Option<Cow<str>> {
        Some(Cow::Owned(format!("{}", self)))
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.content {
              Content::Text(ref content)
            | Content::Link(ref content) => write!(f, "{}", content),
            Content::Lyrics(ref content) => write!(f, "{}", content.text),
            Content::ExtendedText(ref content) => write!(f, "{}: {}", content.description, content.value),
            Content::ExtendedLink(ref content) => write!(f, "{}: {}", content.description, content.link),
            Content::Comment(ref content) => write!(f, "{}: {}", content.description, content.text),
            Content::Picture(ref content) => write!(f, "{}: {:?} ({:?})", content.description, content.picture_type, content.mime_type),
            Content::Unknown(ref content) => write!(f, "unknown, {} bytes", content.len()),
        }
    }
}
