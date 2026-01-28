use std::collections::HashMap;

use serde::{Deserialize, Deserializer};

#[derive(Debug)]
pub struct QueryParams {
    map: HashMap<String, String>,
}

impl<'de> Deserialize<'de> for QueryParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize into a temporary normal map first
        let map = HashMap::<String, String>::deserialize(deserializer)?;
        Ok(QueryParams { map })
    }
}

impl QueryParams {
    pub fn get(&self, key: &str) -> Option<&str> {
        // First try to get the value the normal way.
        if let Some(val) = self.map.get(key) {
            return Some(val);
        }

        // Now uppercase the first letter and try again.
        let mut bkey = key.as_bytes().to_vec();
        if bkey.len() > 0 && bkey[0] >= b'a' && bkey[0] <= b'z' {
            bkey[0] -= 32;
            let key2 = std::str::from_utf8(&bkey).unwrap();
            return self.map.get(key2).map(|x| x.as_str());
        }
        None
    }

    pub fn has(&self, key: &str) -> bool {
        self.map.get(key).is_some()
    }
}
