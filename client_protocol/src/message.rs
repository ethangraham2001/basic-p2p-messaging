/*
 * File: message.rs
 * Author: Ethan Graham
 * Date: 07 Feb. 2024
 *
 * Description: contains Message struct and implementations
 */
use chrono::{DateTime, Local};
use uuid::Uuid;
use json::JsonValue;

// prevent typos
static DST_UUID_FIELD: &str = "dst_uuid";
static SRC_UUID_FIELD: &str = "src_uuid";
static CREATION_TIME_FIELD: &str = "creation_time";
static DATA_FIELD: &str = "data";

/// represents a message created by a peer
pub struct Message {
    pub dst_uuid: Uuid,
    pub src_uuid: Uuid,
    pub creation_time: DateTime<Local>,
    pub data: String,
}

impl Message {
    /// creates a new message. Returns an `Err` if `dst_uuid` and `src_uuid`
    /// are the same
    pub fn new(dst_uuid: Uuid, src_uuid: Uuid, data: &str) 
        -> Result<Message, MessageError>{
        Message::new_with_timestamp(dst_uuid, src_uuid, data, Local::now())
    }

    pub fn new_with_timestamp(dst_uuid: Uuid, src_uuid: Uuid, data: &str, 
               creation_time: DateTime<Local>) -> Result<Message, MessageError>{
        
        /*
        if dst_uuid == src_uuid {
            let err_msg = "dst_uuid and src_uuid should be different!"
                .to_string();
            return Err(MessageError::MessageCreationError(err_msg));
        }
        */

        let gen_msg = Message {
            dst_uuid,
            src_uuid,
            data: data.to_string(),
            creation_time,
        };

        Ok(gen_msg)
    }

    /// creates a message from Json data. Called for incoming messages.
    ///
    /// Assumes the following format:
    /// ```
    /// {
    ///     "dst_uuid"      : "<some 128-bit value>",
    ///     "src_uuid"      : "<different 128-bit value>",
    ///     "data"          : "<a string>",
    ///     "creation_time" : "<timestamp>"
    /// }
    /// ```
    pub fn from_json(json_data: JsonValue) -> Result<Message, MessageError> {
        let dst_uuid        = &json_data[DST_UUID_FIELD];
        let src_uuid        = &json_data[SRC_UUID_FIELD];
        let data            = &json_data[DATA_FIELD];
        let creation_time   = &json_data[CREATION_TIME_FIELD];

        // parse dst_uuid into a Uuid object
        let dst_uuid = match dst_uuid.to_string().parse::<Uuid>() {
            Ok(valid_uuid) => valid_uuid,
            Err(err) => {
                let err_msg = format!("Error parsing dst_uuid: {}", err);
                return Err(MessageError::JsonParseError(err_msg));
            }
        };

        // parse dst_uuid into a Uuid object
        let src_uuid = match src_uuid.to_string().parse::<Uuid>() {
            Ok(valid_uuid) => valid_uuid,
            Err(err) => {
                let err_msg = format!("Error parsing src_uuid: {}", err);
                return Err(MessageError::JsonParseError(err_msg));
            }
        };

        let data = &data.to_string();

        let time_now = Local::now();

        // parse creation_time into a valid DateTime<Local>
        let creation_time = match creation_time.to_string()
            .parse::<DateTime<Local>>() {
            Ok(valid_datetime) => valid_datetime,
            Err(err) => {
                let err_msg = format!("Error parsing creation_time: {}", err);
                return Err(MessageError::JsonParseError(err_msg));
            }
        };

        Message::new_with_timestamp(dst_uuid, src_uuid, data, time_now) 
    }

    /// parses a `Message` to json and returns this value
    pub fn to_json(&self) -> JsonValue {
        let mut json_val = JsonValue::new_object();
        json_val[DST_UUID_FIELD] = JsonValue::from(self.dst_uuid.to_string());
        json_val[SRC_UUID_FIELD] = JsonValue::from(self.src_uuid.to_string());
        json_val[DATA_FIELD] = JsonValue::from(self.data.clone());
        json_val[CREATION_TIME_FIELD] = JsonValue::from(self.creation_time
                                                    .to_string());
        json_val
    }
}

/* error handling */
use std::error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum MessageError {
    MessageCreationError(String),
    JsonParseError(String),
}

impl error::Error for MessageError {}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageError::MessageCreationError(msg) =>
                write!(f, "MessageError: {}", msg),
            MessageError::JsonParseError(msg) =>
                write!(f, "JsonParseError: {}", msg),
        }
    }
}

