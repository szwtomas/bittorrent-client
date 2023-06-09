use super::types::BencodeDecodedValue;
use std::collections::HashMap;
use std::vec::Vec;

const INTEGER_START_TOKEN: u8 = b'i';
const LIST_START_TOKEN: u8 = b'l';
const DICTIONARY_START_TOKEN: u8 = b'd';
const END_TOKEN: u8 = b'e';
const STRING_START_TOKEN: u8 = b':';

#[allow(dead_code)]
/// Encodes a [`BencodeDecodedValue`] into a bencoded byte slice
///
/// ## Example
///
/// ```
/// use bittorrent_rustico::bencode::{encode, BencodeDecodedValue};
///
/// let decoded_value = BencodeDecodedValue::String(b"hola".to_vec());
/// let encoded_value = encode(&decoded_value);
/// assert_eq!(encoded_value, b"4:hola");
///
/// ```
pub fn encode(value: &BencodeDecodedValue) -> Vec<u8> {
    match *value {
        BencodeDecodedValue::Integer(integer) => encode_integer(integer),
        BencodeDecodedValue::String(ref string) => encode_string(string),
        BencodeDecodedValue::List(ref list) => encode_list(list),
        BencodeDecodedValue::Dictionary(ref dictionary) => encode_dictionary(dictionary),
        BencodeDecodedValue::End => vec![] as Vec<u8>,
    }
}

fn encode_integer(integer: i64) -> Vec<u8> {
    let mut bytes = vec![INTEGER_START_TOKEN];
    bytes.extend(integer.to_string().as_bytes());
    bytes.push(END_TOKEN);
    bytes
}

fn encode_string(string: &[u8]) -> Vec<u8> {
    let mut bytes = vec![];
    bytes.extend(string.len().to_string().as_bytes());
    bytes.push(STRING_START_TOKEN);
    bytes.extend(string);
    bytes
}

fn encode_list(list: &[BencodeDecodedValue]) -> Vec<u8> {
    let mut bytes = vec![LIST_START_TOKEN];
    for item in list {
        bytes.extend(encode(item));
    }
    bytes.push(END_TOKEN);
    bytes
}

fn encode_dictionary(dictionary: &HashMap<Vec<u8>, BencodeDecodedValue>) -> Vec<u8> {
    let mut bytes = vec![DICTIONARY_START_TOKEN];
    let mut items: Vec<_> = dictionary.iter().collect();
    items.sort_by_key(|&(key, _)| key);
    for (key, value) in items {
        bytes.extend(encode(&BencodeDecodedValue::String(key.clone())));
        bytes.extend(encode(value));
    }
    bytes.push(END_TOKEN);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encode_positive_number() {
        assert_eq!(
            encode(&BencodeDecodedValue::Integer(123)),
            b"i123e".to_vec()
        );
    }

    #[test]
    fn encode_negative_number() {
        assert_eq!(
            encode(&BencodeDecodedValue::Integer(-123)),
            b"i-123e".to_vec()
        );
    }

    #[test]
    fn encode_zero() {
        assert_eq!(encode(&BencodeDecodedValue::Integer(0)), b"i0e".to_vec());
    }

    #[test]
    fn encode_string() {
        assert_eq!(
            encode(&BencodeDecodedValue::String(b"hola".to_vec())),
            b"4:hola".to_vec()
        );
    }

    #[test]
    fn encode_empty_string() {
        assert_eq!(
            encode(&BencodeDecodedValue::String(b"".to_vec())),
            b"0:".to_vec()
        );
    }

    #[test]
    fn encode_list() {
        assert_eq!(
            encode(&BencodeDecodedValue::List(vec![
                BencodeDecodedValue::Integer(1),
                BencodeDecodedValue::Integer(2),
                BencodeDecodedValue::Integer(3),
            ])),
            b"li1ei2ei3ee".to_vec()
        );
    }

    #[test]
    fn encode_complex_nested_list() {
        let encoded = encode(&BencodeDecodedValue::List(vec![
            BencodeDecodedValue::Integer(1),
            BencodeDecodedValue::List(vec![
                BencodeDecodedValue::Integer(2),
                BencodeDecodedValue::Integer(3),
            ]),
            BencodeDecodedValue::String(b"hola".to_vec()),
        ]));
        assert_eq!(encoded, b"li1eli2ei3ee4:holae".to_vec());
    }

    #[test]
    fn encode_dictionary() {
        let encoded = encode(&BencodeDecodedValue::Dictionary(HashMap::from([
            (
                b"cow".to_vec(),
                BencodeDecodedValue::String(b"moo".to_vec()),
            ),
            (
                b"spam".to_vec(),
                BencodeDecodedValue::String(b"eggs".to_vec()),
            ),
        ])));
        assert_eq!(encoded, b"d3:cow3:moo4:spam4:eggse".to_vec());
    }

    #[test]
    fn encode_complex_nested_dictionary() {
        assert_eq!(
            b"d1:ai123e4:hola4:chau4:testd1:ai123e4:hola4:chauee".to_vec(),
            encode(&BencodeDecodedValue::Dictionary(HashMap::from([
                (b"a".to_vec(), BencodeDecodedValue::Integer(123)),
                (
                    b"hola".to_vec(),
                    BencodeDecodedValue::String(b"chau".to_vec())
                ),
                (
                    b"test".to_vec(),
                    BencodeDecodedValue::Dictionary(HashMap::from([
                        (b"a".to_vec(), BencodeDecodedValue::Integer(123)),
                        (
                            b"hola".to_vec(),
                            BencodeDecodedValue::String(b"chau".to_vec())
                        ),
                    ]))
                )
            ])))
        );
    }
}
