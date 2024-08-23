//! Module responsible for collecting and parsing exported instagram message data (json).

use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use anyhow::Result;

pub struct ConversationDirectory {
    _path: PathBuf,
    message_file_paths: Vec<PathBuf>,
}

#[derive(serde::Deserialize)]
pub struct ParsedConversation {
    pub participants: HashSet<Participant>,
    pub messages: Vec<Message>,
}

#[derive(serde::Deserialize, Eq, PartialEq, Hash, Clone)]
pub struct Participant {
    pub name: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct Message {
    pub sender_name: String,
    pub timestamp_ms: usize,
    #[serde(default)]
    // some messages (e.g. images) do not have content, so we default to an empty string that we can ignore later
    pub content: String,
}

impl TryFrom<PathBuf> for ConversationDirectory {
    type Error = std::io::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        if path.is_dir() {
            // ensure that the directory contains files named like `message_\d+.json`
            // and that the files are not empty
            let message_file_paths = path
                .read_dir()?
                // filter out non-files
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                // filter out files with the wrong naming convention
                .filter_map(|path| {
                    let stem = path.file_stem().and_then(std::ffi::OsStr::to_str)?;
                    let ext = path.extension().and_then(std::ffi::OsStr::to_str)?;
                    match (
                        stem.split_at(if stem.len() >= 8 { 8 } else { stem.len() }),
                        ext,
                    ) {
                        (("message_", num), "json") if num.parse::<u32>().is_ok() => Some(path),
                        _ => None,
                    }
                })
                .collect::<Vec<_>>();

            if message_file_paths.is_empty() {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Directory does not contain message data",
                ))
            } else {
                Ok(Self {
                    _path: path,
                    message_file_paths,
                })
            }
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Path is not a directory",
            ))
        }
    }
}

impl ConversationDirectory {
    pub fn parse(&self) -> Result<ParsedConversation> {
        Ok(ParsedConversation::merge(
            &self
                .message_file_paths
                .iter()
                .map(|path| {
                    let file = File::open(path)?;
                    let mut reader = BufReader::new(file);
                    let mut buffer = Vec::new();

                    // read the file into a buffer
                    reader.read_to_end(&mut buffer)?;

                    // decode the buffer into a string
                    let mut decoded_bytes = Vec::new();
                    let mut to_skip = 0;
                    for (i, c) in buffer.iter().enumerate() {
                        if to_skip > 0 {
                            to_skip -= 1;
                            continue;
                        }

                        // if we encounter an escaped character (format example: \u00f0), we need to decode it (example: 0xf0, which is ð)
                        // if the next character is not a 'u', we just push the character as is
                        // also ensure that the next 2 characters are 0's
                        if *c == b'\\'
                            && buffer[i + 1] == b'u'
                            && buffer[i + 2] == b'0'
                            && buffer[i + 3] == b'0'
                            && buffer[i + 4].is_ascii_alphanumeric()
                            && buffer[i + 5].is_ascii_alphanumeric()
                        {
                            let hex = u8::from_str_radix(
                                std::str::from_utf8(&buffer[i + 4..i + 6]).unwrap(),
                                16,
                            )
                            .unwrap();
                            decoded_bytes.push(hex);
                            to_skip = 5;
                        } else {
                            decoded_bytes.push(*c);
                        }
                    }

                    let decoded_string = String::from_utf8(decoded_bytes).unwrap();

                    let parsed_conversation: ParsedConversation =
                        serde_json::from_str(&decoded_string)?;
                    Ok(parsed_conversation)
                })
                .collect::<Result<Vec<ParsedConversation>>>()?,
        ))
    }
}

impl ParsedConversation {
    fn merge(conversations: &[Self]) -> Self {
        let participants = conversations
            .iter()
            .flat_map(|c| c.participants.iter())
            .cloned()
            .collect::<HashSet<_>>();
        let mut messages = conversations
            .iter()
            .flat_map(|c| c.messages.iter())
            // filter out empty messages
            .filter(|message| !message.content.is_empty())
            // // filter out messages that are just reacting to a message
            // .filter(|message| {
            //     !(message.content.starts_with("Reacted ")
            //         && message.content.ends_with(" to your message "))
            // })
            // filter out the "__ wasn't notified about this message" messages
            .filter(|message| {
                !message
                    .content
                    .ends_with(" wasn't notified about this message because they're in quiet mode.")
            })
            .cloned()
            .collect::<Vec<_>>();

        // sort messages by timestamp
        messages.sort_by_key(|message| message.timestamp_ms);

        Self {
            participants,
            messages,
        }
    }
}
