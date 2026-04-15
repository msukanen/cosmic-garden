/// (De)serialize Vec<String> ↔ Hashmap<String,Bool>
/// 
pub mod string_vec_to_bool_map {
    use std::collections::HashMap;

    use serde::{Deserialize, Deserializer, Serializer, ser::SerializeSeq};

    pub fn serialize<S>(map: &HashMap<String,bool>, s:S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        let mut seq = s.serialize_seq(Some(map.len()))?;
        for id in map.keys() {
            seq.serialize_element(id)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(d:D) -> Result<HashMap<String,bool>, D::Error>
    where D: Deserializer<'de>
    {
        let ids = Vec::<String>::deserialize(d)?;
        Ok(ids.into_iter().map(|id|(id, false)).collect())
    }
}
