use anyhow::{anyhow, Result};
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::TypeTag;
use move_core_types::value::{MoveStruct, MoveValue};
use serde_json::{json, Map, Value};

pub fn move_value_to_json(val: MoveValue) -> Value {
    match val {
        MoveValue::U8(n) => serde_json::to_value(n).unwrap(),
        MoveValue::U64(n) => serde_json::to_value(n).unwrap(),
        MoveValue::U128(n) => serde_json::to_value(n.to_string()).unwrap(),
        MoveValue::Bool(b) => serde_json::to_value(b).unwrap(),
        MoveValue::Address(add) => serde_json::to_value(add).unwrap(),
        MoveValue::Vector(vals) => {
            // If this is a vector<u8>, convert it to hex string
            if is_non_empty_vector_u8(&vals) {
                let bytes = vec_to_vec_u8(vals).unwrap();
                serde_json::to_value(format!("0x{}", hex::encode(&bytes))).unwrap()
            } else {
                Value::Array(
                    vals.into_iter()
                        .map(|v| move_value_to_json(v))
                        .collect()
                )
            }
        }
        MoveValue::Struct(move_struct) => match move_struct {
            MoveStruct::Runtime(fields) => {
                Value::Array(
                    fields.into_iter()
                        .map(|v| move_value_to_json(v))
                        .collect()
                )
            }
            MoveStruct::WithFields( fields ) => struct_fields_to_json(fields),
            MoveStruct::WithTypes { type_, fields } =>
                struct_fields_to_json(fields)
        }
        MoveValue::Signer(add) => serde_json::to_value(add).unwrap()
    }
}

fn struct_fields_to_json(fields: Vec<(Identifier, MoveValue)>) -> Value {
    let mut iter = fields.into_iter();
    let mut map = Map::new();
    if let Some((field_name, field_value)) = iter.next() {
        map.insert(field_name.into_string(), move_value_to_json(field_value));
    }
    Value::Object(map)
}

fn is_non_empty_vector_u8(vec: &Vec<MoveValue>) -> bool {
    if vec.is_empty() {
        false
    } else {
        matches!(vec.last().unwrap(), MoveValue::U8(_))
    }
}

/// Converts the `Vec<MoveValue>` to a `Vec<u8>` if the inner `MoveValue` is a `MoveValue::U8`,
/// or returns an error otherwise.
pub fn vec_to_vec_u8(vec: Vec<MoveValue>) -> Result<Vec<u8>> {
    let mut vec_u8 = Vec::with_capacity(vec.len());

    for byte in vec {
        match byte {
            MoveValue::U8(u8) => {
                vec_u8.push(u8);
            }
            _ => {
                return Err(anyhow!(
                        "Expected inner MoveValue in Vec<MoveValue> to be a MoveValue::U8"
                            .to_string(),
                    ));
            }
        }
    }
    Ok(vec_u8)
}

#[cfg(test)]
mod tests {
    use move_core_types::account_address::AccountAddress;
    use move_core_types::value::MoveValue;
    use serde_json::json;
    use crate::converter::move_value_to_json;

    #[test]
    fn test_number_to_json() {
        let u8_val = MoveValue::U8(0);
        assert_eq!(move_value_to_json(u8_val), json!(0));

        let u64_val = MoveValue::U64(0);
        assert_eq!(move_value_to_json(u64_val), json!(0));

        let u128_val = MoveValue::U128(0);
        assert_eq!(move_value_to_json(u128_val), json!("0"));
    }

    #[test]
    fn test_bool_to_json() {
        let bool_val = MoveValue::Bool(true);
        assert_eq!(move_value_to_json(bool_val), json!(true));
    }
}