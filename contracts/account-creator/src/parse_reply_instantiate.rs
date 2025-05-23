/// Originally cw0::parseReplyInstantiateData, which doesn't play nice with secret
use secret_cosmwasm_std::{Binary, Reply};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParseReplyError {
    #[error("Failure response from sub-message: {0}")]
    SubMsgFailure(String),

    #[error("Invalid reply from sub-message: {0}")]
    ParseFailure(String),

    #[error("Error occurred while converting from UTF-8")]
    BrokenUtf8(#[from] std::string::FromUtf8Error),
}

// Protobuf wire types (https://developers.google.com/protocol-buffers/docs/encoding)
const WIRE_TYPE_LENGTH_DELIMITED: u8 = 2;
// Up to 9 bytes of varints as a practical limit (https://github.com/multiformats/unsigned-varint#practical-maximum-of-9-bytes-for-security)
const VARINT_MAX_BYTES: usize = 9;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgInstantiateContractResponse {
    pub contract_address: String,
    pub data: Option<Binary>,
}

pub fn parse_reply_instantiate_data(
    msg: Reply,
) -> Result<MsgInstantiateContractResponse, ParseReplyError> {
    let data = msg
        .result
        .clone()
        .into_result()
        .map_err(ParseReplyError::SubMsgFailure)?
        .data
        .unwrap_or_else(|| Binary::from(vec![]));
    if data.len() > 0 {
        parse_instantiate_response_data(&data.0)
    } else {
        let events = msg
            .result
            .into_result()
            .map_err(ParseReplyError::SubMsgFailure)?
            .events;
        let res = MsgInstantiateContractResponse {
            contract_address: events
                .into_iter()
                .filter_map(|event| {
                    if event.ty == "instantiate" {
                        if event.attributes.len() > 1 {
                            Some(event.attributes[1].value.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()[0]
                .clone(),
            data: None,
        };
        Ok(res)
    }
}

pub fn parse_instantiate_response_data(
    data: &[u8],
) -> Result<MsgInstantiateContractResponse, ParseReplyError> {
    // Manual protobuf decoding
    let mut data = data.to_vec();
    // Parse contract addr
    let contract_addr = parse_protobuf_string(&mut data, 1)?;

    // Parse (optional) data
    let data = parse_protobuf_bytes(&mut data, 2)?;

    Ok(MsgInstantiateContractResponse {
        contract_address: contract_addr,
        data,
    })
}

fn parse_protobuf_string(data: &mut Vec<u8>, field_number: u8) -> Result<String, ParseReplyError> {
    let str_field = parse_protobuf_length_prefixed(data, field_number)?;
    Ok(String::from_utf8(str_field)?)
}

fn parse_protobuf_bytes(
    data: &mut Vec<u8>,
    field_number: u8,
) -> Result<Option<Binary>, ParseReplyError> {
    let bytes_field = parse_protobuf_length_prefixed(data, field_number)?;
    if bytes_field.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Binary(bytes_field)))
    }
}

/// Helper function to parse length-prefixed protobuf fields.
/// The remaining of the data is kept in the data parameter.
fn parse_protobuf_length_prefixed(
    data: &mut Vec<u8>,
    field_number: u8,
) -> Result<Vec<u8>, ParseReplyError> {
    if data.is_empty() {
        return Ok(vec![]);
    };
    let mut rest_1 = data.split_off(1);
    let wire_type = data[0] & 0b11;
    let field = data[0] >> 3;

    if field != field_number {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: invalid field #{} for field #{}",
            field, field_number
        )));
    }
    if wire_type != WIRE_TYPE_LENGTH_DELIMITED {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: field #{}: invalid wire type {}",
            field_number, wire_type
        )));
    }

    let len = parse_protobuf_varint(&mut rest_1, field_number)?;
    if rest_1.len() < len {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: field #{}: message too short",
            field_number
        )));
    }
    *data = rest_1.split_off(len);

    Ok(rest_1)
}

/// Base128 varint decoding.
/// The remaining of the data is kept in the data parameter.
fn parse_protobuf_varint(data: &mut Vec<u8>, field_number: u8) -> Result<usize, ParseReplyError> {
    let data_len = data.len();
    let mut len: u64 = 0;
    let mut i = 0;
    while i < VARINT_MAX_BYTES {
        if data_len == i {
            return Err(ParseReplyError::ParseFailure(format!(
                "failed to decode Protobuf message: field #{}: varint data too short",
                field_number
            )));
        }
        len += ((data[i] & 0x7f) as u64) << (i * 7);
        if data[i] & 0x80 == 0 {
            break;
        }
        i += 1;
    }
    if i == VARINT_MAX_BYTES {
        return Err(ParseReplyError::ParseFailure(format!(
            "failed to decode Protobuf message: field #{}: varint data too long",
            field_number
        )));
    }
    *data = data[i + 1..].to_owned();

    Ok(len as usize) // Gently fall back to the arch's max addressable size
}
